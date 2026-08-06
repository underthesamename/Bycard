use std::{env, str::FromStr, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use bycard_api::app::{AuthSettings, build_router};
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

    let registration = post_json(
        &app,
        "/api/v1/auth/register",
        json!({
            "displayName": "  Ana Colecionadora  ",
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
        json!({"displayName": "Outra Ana", "email": "ana@example.com", "password": PASSWORD}),
        None,
        None,
    )
    .await?;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
    assert_eq!(duplicate.body["error"]["code"], "email_already_registered");

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
        json!({"email": "ana@example.com", "password": PASSWORD}),
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
        json!({"displayName": "Sessão Curta", "email": email, "password": PASSWORD}),
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
        json!({"displayName": "Cookie Seguro", "email": email, "password": PASSWORD}),
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
        json!({"email": email, "password": password}),
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
