use std::{env, io::Cursor, str::FromStr, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode, header},
};
use bycard_api::app::{AuthSettings, build_router};
use image::{DynamicImage, ImageFormat};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgConnectOptions};
use tower::ServiceExt;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
const ORIGIN: &str = "http://localhost:3000";
const PASSWORD: &str = "uma-senha-segura-com-15";

#[tokio::test]
async fn authentication_http_contract() -> Result<()> {
    dotenvy::dotenv().ok();
    let admin_url = env::var("TEST_DATABASE_URL").context(
        "TEST_DATABASE_URL is required for PostgreSQL integration tests; start the test database first",
    )?;
    let admin_pool = PgPool::connect(&admin_url).await?;
    let database_name = format!("bycard_auth_test_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database_name}")))
        .execute(&admin_pool)
        .await?;

    let options = PgConnectOptions::from_str(&admin_url)?.database(&database_name);
    let pool = PgPool::connect_with(options).await?;
    let result = exercise_authentication_contract(&pool).await;
    pool.close().await;

    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE {database_name} WITH (FORCE)"
    )))
    .execute(&admin_pool)
    .await?;
    admin_pool.close().await;
    result
}

async fn exercise_authentication_contract(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await?;
    let app = build_router(
        pool.clone(),
        ORIGIN.parse()?,
        auth_settings(false, Duration::from_secs(3600)),
    )?;

    let missing_special = post_json(
        &app,
        "/api/v1/auth/register",
        json!({
            "displayName": "Senha sem símbolo",
            "username": "sem.simbolo",
            "email": "sem-simbolo@example.com",
            "password": "123456789012345"
        }),
        None,
        None,
    )
    .await?;
    assert_eq!(missing_special.status, StatusCode::BAD_REQUEST);
    assert_eq!(missing_special.body["error"]["code"], "invalid_password");

    let registration = post_json(
        &app,
        "/api/v1/auth/register",
        json!({
            "displayName": "  Ana Colecionadora  ",
            "username": "  Ana.TCG  ",
            "email": "  ANA@EXAMPLE.COM ",
            "password": PASSWORD
        }),
        None,
        None,
    )
    .await?;
    assert_eq!(registration.status, StatusCode::CREATED);
    assert_eq!(
        registration.body["user"]["displayName"],
        "Ana Colecionadora"
    );
    assert_eq!(registration.body["user"]["email"], "ana@example.com");
    assert_eq!(registration.body["user"]["username"], "ana.tcg");
    let cookie = registration
        .cookie
        .context("registration must set a cookie")?;
    assert!(cookie.starts_with("bycard_session="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(!cookie.contains("Secure"));
    assert!(!registration.raw_body.contains("bycard_session"));
    assert!(!registration.raw_body.contains(PASSWORD));

    let duplicate = post_json(
        &app,
        "/api/v1/auth/register",
        json!({"displayName": "Outra Ana", "username": "outra.ana", "email": "ana@example.com", "password": PASSWORD}),
        None,
        None,
    )
    .await?;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
    assert_eq!(
        duplicate.body["error"]["code"],
        "account_already_registered"
    );
    verify_username_uniqueness(pool, &app).await?;

    let valid_session = get(&app, "/api/v1/auth/me", Some(&cookie)).await?;
    assert_eq!(valid_session.status, StatusCode::OK);
    assert_eq!(valid_session.body["user"]["email"], "ana@example.com");

    let wrong_password = login(&app, "ana@example.com", "senha-totalmente-incorreta").await?;
    let missing_user = login(&app, "ninguem@example.com", "senha-totalmente-incorreta").await?;
    assert_eq!(wrong_password.status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_password.raw_body, missing_user.raw_body);
    assert_eq!(wrong_password.body["error"]["code"], "invalid_credentials");

    let login = login(&app, "ANA@example.com", PASSWORD).await?;
    assert_eq!(login.status, StatusCode::OK);
    let login_cookie = login.cookie.context("login must rotate the cookie")?;
    assert_ne!(cookie_value(&cookie), cookie_value(&login_cookie));

    let missing_origin = post_json(
        &app,
        "/api/v1/auth/login",
        json!({"identifier": "ana@example.com", "password": PASSWORD}),
        None,
        Some(""),
    )
    .await?;
    assert_eq!(missing_origin.status, StatusCode::FORBIDDEN);
    assert_eq!(missing_origin.body["error"]["code"], "invalid_origin");

    let csrf = get(&app, "/api/v1/auth/csrf", Some(&login_cookie)).await?;
    assert_eq!(csrf.status, StatusCode::OK);
    let csrf_token = csrf.body["csrfToken"]
        .as_str()
        .context("CSRF response must contain a token")?;
    assert!(!csrf.raw_body.contains(cookie_value(&login_cookie)));
    verify_profile_contract(&app, &login_cookie, csrf_token).await?;

    let rejected_logout = post_json(
        &app,
        "/api/v1/auth/logout",
        Value::Null,
        Some(&login_cookie),
        Some("invalid-token"),
    )
    .await?;
    assert_eq!(rejected_logout.status, StatusCode::FORBIDDEN);

    let logout = post_json(
        &app,
        "/api/v1/auth/logout",
        Value::Null,
        Some(&login_cookie),
        Some(csrf_token),
    )
    .await?;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    assert!(
        logout
            .cookie
            .as_deref()
            .is_some_and(|value| value.contains("Max-Age=0"))
    );
    assert_eq!(
        get(&app, "/api/v1/auth/me", Some(&login_cookie))
            .await?
            .status,
        StatusCode::UNAUTHORIZED
    );

    verify_expired_session(pool).await?;
    verify_rate_limit(pool).await?;
    verify_production_cookie(pool).await?;
    Ok(())
}

async fn verify_profile_contract(app: &Router, cookie: &str, csrf_token: &str) -> Result<()> {
    let private_avatar = app
        .clone()
        .oneshot(Request::get("/api/v1/me/avatar").body(Body::empty())?)
        .await?;
    assert_eq!(private_avatar.status(), StatusCode::UNAUTHORIZED);

    let rejected_update = put_json(
        app,
        "/api/v1/me/profile",
        json!({ "displayName": "Ana Atualizada" }),
        Some(cookie),
        None,
    )
    .await?;
    assert_eq!(rejected_update.status, StatusCode::FORBIDDEN);
    assert_eq!(rejected_update.body["error"]["code"], "csrf_rejected");

    let update = put_json(
        app,
        "/api/v1/me/profile",
        json!({ "displayName": "  Ana Atualizada  " }),
        Some(cookie),
        Some(csrf_token),
    )
    .await?;
    assert_eq!(update.status, StatusCode::OK);
    assert_eq!(update.body["user"]["displayName"], "Ana Atualizada");
    assert_eq!(update.body["user"]["avatarVersion"], Value::Null);

    let invalid_avatar = put_bytes(
        app,
        "/api/v1/me/avatar",
        b"not-an-image".to_vec(),
        "image/png",
        cookie,
        csrf_token,
    )
    .await?;
    assert_eq!(invalid_avatar.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid_avatar.body["error"]["code"], "invalid_avatar");

    let oversized_avatar = app
        .clone()
        .oneshot(
            Request::put("/api/v1/me/avatar")
                .header(header::CONTENT_TYPE, "image/png")
                .header(header::ORIGIN, ORIGIN)
                .header(header::COOKIE, cookie_value(cookie))
                .header("x-csrf-token", csrf_token)
                .body(Body::from(vec![0_u8; 2 * 1024 * 1024 + 1]))?,
        )
        .await?;
    assert_eq!(oversized_avatar.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let upload = put_bytes(
        app,
        "/api/v1/me/avatar",
        test_png()?,
        "image/png",
        cookie,
        csrf_token,
    )
    .await?;
    assert_eq!(upload.status, StatusCode::OK);
    let avatar_version = upload.body["avatarVersion"]
        .as_str()
        .context("avatar upload must return its version")?;

    let avatar = get_binary(app, "/api/v1/me/avatar", cookie, None).await?;
    assert_eq!(avatar.status, StatusCode::OK);
    assert_eq!(
        avatar.headers.get(header::CONTENT_TYPE),
        Some(&"image/jpeg".parse()?)
    );
    assert!(avatar.body.starts_with(&[0xff, 0xd8, 0xff]));
    assert!(avatar.body.len() <= 128 * 1024);
    assert_eq!(
        avatar.headers.get(header::ETAG),
        Some(&format!("\"{avatar_version}\"").parse()?)
    );

    let not_modified = get_binary(
        app,
        "/api/v1/me/avatar",
        cookie,
        Some(&format!("\"{avatar_version}\"")),
    )
    .await?;
    assert_eq!(not_modified.status, StatusCode::NOT_MODIFIED);
    assert!(not_modified.body.is_empty());

    let deletion = delete(app, "/api/v1/me/avatar", cookie, csrf_token).await?;
    assert_eq!(deletion.status, StatusCode::NO_CONTENT);
    let missing_avatar = get_binary(app, "/api/v1/me/avatar", cookie, None).await?;
    assert_eq!(missing_avatar.status, StatusCode::NOT_FOUND);

    let session = get(app, "/api/v1/auth/me", Some(cookie)).await?;
    assert_eq!(session.body["user"]["displayName"], "Ana Atualizada");
    assert_eq!(session.body["user"]["avatarVersion"], Value::Null);
    Ok(())
}

async fn verify_username_uniqueness(pool: &PgPool, app: &Router) -> Result<()> {
    let normalized_duplicate = post_json(
        app,
        "/api/v1/auth/register",
        json!({
            "displayName": "Outra colecionadora",
            "username": "  ANA.TCG  ",
            "email": "outra-ana@example.com",
            "password": PASSWORD
        }),
        None,
        None,
    )
    .await?;
    assert_eq!(normalized_duplicate.status, StatusCode::CONFLICT);
    assert_eq!(
        normalized_duplicate.body["error"]["code"],
        "account_already_registered"
    );

    let first_registration = post_json(
        app,
        "/api/v1/auth/register",
        json!({
            "displayName": "Primeira colecionadora",
            "username": "Corrida.TCG",
            "email": "primeira-corrida@example.com",
            "password": PASSWORD
        }),
        None,
        None,
    );
    let second_registration = post_json(
        app,
        "/api/v1/auth/register",
        json!({
            "displayName": "Segunda colecionadora",
            "username": " corrida.tcg ",
            "email": "segunda-corrida@example.com",
            "password": PASSWORD
        }),
        None,
        None,
    );
    let (first_registration, second_registration) =
        tokio::join!(first_registration, second_registration);
    let registrations = [first_registration?, second_registration?];

    assert_eq!(
        registrations
            .iter()
            .filter(|response| response.status == StatusCode::CREATED)
            .count(),
        1
    );
    assert_eq!(
        registrations
            .iter()
            .filter(|response| response.status == StatusCode::CONFLICT)
            .count(),
        1
    );
    let conflict = registrations
        .iter()
        .find(|response| response.status == StatusCode::CONFLICT)
        .context("one concurrent registration must be rejected")?;
    assert_eq!(conflict.body["error"]["code"], "account_already_registered");

    let stored_users =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE username = $1")
            .bind("corrida.tcg")
            .fetch_one(pool)
            .await?;
    assert_eq!(stored_users, 1);
    Ok(())
}

async fn verify_expired_session(pool: &PgPool) -> Result<()> {
    let app = build_router(
        pool.clone(),
        ORIGIN.parse()?,
        auth_settings(false, Duration::from_secs(1)),
    )?;
    let email = format!("expired-{}@example.com", Uuid::now_v7());
    let response = post_json(
        &app,
        "/api/v1/auth/register",
        json!({"displayName": "Sessão Curta", "username": format!("curta-{}", Uuid::now_v7().simple()).chars().take(24).collect::<String>(), "email": email, "password": PASSWORD}),
        None,
        None,
    )
    .await?;
    let cookie = response.cookie.context("registration must set a cookie")?;
    sqlx::query(
        "UPDATE sessions SET created_at = NOW() - INTERVAL '2 seconds', expires_at = NOW() - INTERVAL '1 second'",
    )
        .execute(pool)
        .await?;
    assert_eq!(
        get(&app, "/api/v1/auth/me", Some(&cookie)).await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

async fn verify_rate_limit(pool: &PgPool) -> Result<()> {
    let app = build_router(
        pool.clone(),
        ORIGIN.parse()?,
        auth_settings(false, Duration::from_secs(3600)),
    )?;
    let email = format!("rate-{}@example.com", Uuid::now_v7());
    for _ in 0..5 {
        assert_eq!(
            login(&app, &email, "senha-totalmente-incorreta")
                .await?
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
    let limited = login(&app, &email, "senha-totalmente-incorreta").await?;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.body["error"]["code"], "rate_limit_exceeded");
    Ok(())
}

async fn verify_production_cookie(pool: &PgPool) -> Result<()> {
    let app = build_router(
        pool.clone(),
        ORIGIN.parse()?,
        auth_settings(true, Duration::from_secs(3600)),
    )?;
    let email = format!("secure-{}@example.com", Uuid::now_v7());
    let response = post_json(
        &app,
        "/api/v1/auth/register",
        json!({"displayName": "Cookie Seguro", "username": format!("seguro-{}", Uuid::now_v7().simple()).chars().take(24).collect::<String>(), "email": email, "password": PASSWORD}),
        None,
        None,
    )
    .await?;
    let cookie = response.cookie.context("registration must set a cookie")?;
    assert!(cookie.starts_with("__Host-bycard_session="));
    assert!(cookie.contains("; Secure"));
    Ok(())
}

fn auth_settings(secure_cookie: bool, session_ttl: Duration) -> AuthSettings {
    AuthSettings {
        secure_cookie,
        hmac_key: b"auth-test-key-with-at-least-thirty-two-bytes".to_vec(),
        session_ttl,
        idle_ttl: session_ttl,
        touch_interval: Duration::from_millis(500),
        csrf_ttl: Duration::from_secs(300),
    }
}

async fn login(app: &Router, email: &str, password: &str) -> Result<TestResponse> {
    post_json(
        app,
        "/api/v1/auth/login",
        json!({"identifier": email, "password": password}),
        None,
        None,
    )
    .await
}

async fn get(app: &Router, path: &str, cookie: Option<&str>) -> Result<TestResponse> {
    let mut builder = Request::get(path).header(header::ACCEPT, "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie_value(cookie));
    }
    send(app, builder.body(Body::empty())?).await
}

async fn post_json(
    app: &Router,
    path: &str,
    body: Value,
    cookie: Option<&str>,
    csrf_token: Option<&str>,
) -> Result<TestResponse> {
    let mut builder = Request::post(path)
        .header(header::ACCEPT, "application/json")
        .header(header::ORIGIN, ORIGIN);
    if body != Value::Null {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie_value(cookie));
    }
    if let Some(csrf_token) = csrf_token {
        if csrf_token.is_empty() {
            builder = Request::post(path).header(header::ACCEPT, "application/json");
        } else {
            builder = builder.header("x-csrf-token", csrf_token);
        }
    }
    let bytes = if body == Value::Null {
        Vec::new()
    } else {
        serde_json::to_vec(&body)?
    };
    send(app, builder.body(Body::from(bytes))?).await
}

async fn put_json(
    app: &Router,
    path: &str,
    body: Value,
    cookie: Option<&str>,
    csrf_token: Option<&str>,
) -> Result<TestResponse> {
    let mut builder = Request::put(path)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, ORIGIN);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie_value(cookie));
    }
    if let Some(csrf_token) = csrf_token {
        builder = builder.header("x-csrf-token", csrf_token);
    }
    send(app, builder.body(Body::from(serde_json::to_vec(&body)?))?).await
}

async fn put_bytes(
    app: &Router,
    path: &str,
    body: Vec<u8>,
    content_type: &str,
    cookie: &str,
    csrf_token: &str,
) -> Result<TestResponse> {
    let request = Request::put(path)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ORIGIN, ORIGIN)
        .header(header::COOKIE, cookie_value(cookie))
        .header("x-csrf-token", csrf_token)
        .body(Body::from(body))?;
    send(app, request).await
}

async fn delete(app: &Router, path: &str, cookie: &str, csrf_token: &str) -> Result<TestResponse> {
    let request = Request::delete(path)
        .header(header::ACCEPT, "application/json")
        .header(header::ORIGIN, ORIGIN)
        .header(header::COOKIE, cookie_value(cookie))
        .header("x-csrf-token", csrf_token)
        .body(Body::empty())?;
    send(app, request).await
}

async fn get_binary(
    app: &Router,
    path: &str,
    cookie: &str,
    etag: Option<&str>,
) -> Result<BinaryTestResponse> {
    let mut builder = Request::get(path)
        .header(header::ACCEPT, "image/jpeg")
        .header(header::COOKIE, cookie_value(cookie));
    if let Some(etag) = etag {
        builder = builder.header(header::IF_NONE_MATCH, etag);
    }
    let response = app.clone().oneshot(builder.body(Body::empty())?).await?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 1024 * 1024).await?.to_vec();
    Ok(BinaryTestResponse {
        status,
        headers,
        body,
    })
}

fn test_png() -> Result<Vec<u8>> {
    let image = DynamicImage::new_rgba8(32, 24);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

async fn send(app: &Router, request: Request<Body>) -> Result<TestResponse> {
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .map(|value| value.to_str().map(str::to_owned))
        .transpose()?;
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let raw_body = String::from_utf8(bytes.to_vec())?;
    let body = if raw_body.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&raw_body)?
    };
    Ok(TestResponse {
        status,
        cookie,
        raw_body,
        body,
    })
}

fn cookie_value(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap_or(set_cookie)
}

struct TestResponse {
    status: StatusCode,
    cookie: Option<String>,
    raw_body: String,
    body: Value,
}

struct BinaryTestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}
