use std::{env, path::PathBuf, str::FromStr, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use bycard_api::{
    app::{AuthSettings, build_router},
    catalog_import,
};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgConnectOptions};
use tower::ServiceExt;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
const ORIGIN: &str = "http://localhost:3000";
const PASSWORD: &str = "uma-senha-segura-com-15";

#[tokio::test]
async fn personal_collections_holdings_and_isolation_contract() -> Result<()> {
    dotenvy::dotenv().ok();
    let admin_url = env::var("TEST_DATABASE_URL").context(
        "TEST_DATABASE_URL is required for PostgreSQL integration tests; start the test database first",
    )?;
    let admin_pool = PgPool::connect(&admin_url).await?;
    let database_name = format!("bycard_holdings_test_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database_name}")))
        .execute(&admin_pool)
        .await?;
    let options = PgConnectOptions::from_str(&admin_url)?.database(&database_name);
    let pool = PgPool::connect_with(options).await?;
    let result = exercise_contract(&pool).await;
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE {database_name} WITH (FORCE)"
    )))
    .execute(&admin_pool)
    .await?;
    admin_pool.close().await;
    result
}

async fn exercise_contract(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await?;
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo-catalog/catalog.json");
    let fixture = catalog_import::load_fixture(&fixture_path)?;
    catalog_import::import_catalog(pool, &fixture).await?;
    let app = build_router(pool.clone(), ORIGIN.parse()?, auth_settings())?;
    let first_set: Uuid = sqlx::query_scalar("SELECT id FROM sets ORDER BY release_date LIMIT 1")
        .fetch_one(pool)
        .await?;
    let second_set: Uuid =
        sqlx::query_scalar("SELECT id FROM sets ORDER BY release_date DESC LIMIT 1")
            .fetch_one(pool)
            .await?;
    let first_card: Uuid =
        sqlx::query_scalar("SELECT id FROM cards WHERE set_id = $1 ORDER BY sort_order LIMIT 1")
            .bind(first_set)
            .fetch_one(pool)
            .await?;
    let second_set_card: Uuid =
        sqlx::query_scalar("SELECT id FROM cards WHERE set_id = $1 ORDER BY sort_order LIMIT 1")
            .bind(second_set)
            .fetch_one(pool)
            .await?;

    let ana = register(&app, "Ana", "ana-holdings@example.com").await?;
    let bia = register(&app, "Bia", "bia-holdings@example.com").await?;

    let created = mutate(
        &app,
        "POST",
        "/api/v1/me/collections",
        json!({ "setId": first_set }),
        &ana,
    )
    .await?;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.body["data"]["totalUnique"], 18);
    assert_eq!(created.body["data"]["completionPercentage"], 0.0);
    let duplicate = mutate(
        &app,
        "POST",
        "/api/v1/me/collections",
        json!({ "setId": first_set }),
        &ana,
    )
    .await?;
    assert_eq!(duplicate.status, StatusCode::OK);

    let untracked = mutate(
        &app,
        "PUT",
        &format!("/api/v1/me/collections/{second_set}/cards/{second_set_card}"),
        json!({ "quantity": 1 }),
        &ana,
    )
    .await?;
    assert_eq!(untracked.status, StatusCode::NOT_FOUND);
    let wrong_set = mutate(
        &app,
        "PUT",
        &format!("/api/v1/me/collections/{first_set}/cards/{second_set_card}"),
        json!({ "quantity": 1 }),
        &ana,
    )
    .await?;
    assert_eq!(wrong_set.status, StatusCode::NOT_FOUND);

    let holding_path = format!("/api/v1/me/collections/{first_set}/cards/{first_card}");
    let owned = mutate(&app, "PUT", &holding_path, json!({ "quantity": 1 }), &ana).await?;
    assert_eq!(owned.status, StatusCode::OK);
    assert_eq!(owned.body["data"]["collection"]["ownedUnique"], 1);
    assert_eq!(owned.body["data"]["collection"]["totalCopies"], 1);
    let repeated = mutate(&app, "PUT", &holding_path, json!({ "quantity": 3 }), &ana).await?;
    assert_eq!(repeated.body["data"]["collection"]["duplicateCopies"], 2);
    assert_eq!(repeated.body["data"]["collection"]["totalCopies"], 3);

    let invalid = mutate(
        &app,
        "PUT",
        &holding_path,
        json!({ "quantity": 1000 }),
        &ana,
    )
    .await?;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid.body["error"]["code"], "invalid_quantity");

    let bia_view = get(
        &app,
        &format!("/api/v1/me/collections/{first_set}"),
        &bia.cookie,
    )
    .await?;
    assert_eq!(bia_view.status, StatusCode::NOT_FOUND);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_card_holdings")
            .fetch_one(pool)
            .await?,
        1
    );

    let concurrent_a = mutate(&app, "PUT", &holding_path, json!({ "quantity": 4 }), &ana);
    let concurrent_b = mutate(&app, "PUT", &holding_path, json!({ "quantity": 4 }), &ana);
    let (a, b) = tokio::join!(concurrent_a, concurrent_b);
    assert_eq!(a?.status, StatusCode::OK);
    assert_eq!(b?.status, StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT quantity FROM user_card_holdings WHERE card_id = $1")
            .bind(first_card)
            .fetch_one(pool)
            .await?,
        4
    );

    let zeroed = mutate(&app, "PUT", &holding_path, json!({ "quantity": 0 }), &ana).await?;
    assert_eq!(zeroed.body["data"]["collection"]["ownedUnique"], 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_card_holdings")
            .fetch_one(pool)
            .await?,
        0
    );
    mutate(&app, "PUT", &holding_path, json!({ "quantity": 2 }), &ana).await?;
    let removed = mutate(
        &app,
        "DELETE",
        &format!("/api/v1/me/collections/{first_set}"),
        Value::Null,
        &ana,
    )
    .await?;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_card_holdings")
            .fetch_one(pool)
            .await?,
        0
    );
    Ok(())
}

struct Session {
    cookie: String,
    csrf: String,
}

async fn register(app: &Router, display_name: &str, email: &str) -> Result<Session> {
    let response = send(
        app,
        Request::post("/api/v1/auth/register")
            .header(header::ORIGIN, ORIGIN)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "displayName": display_name, "username": format!("user-{}", Uuid::now_v7().simple()).chars().take(24).collect::<String>(), "email": email, "password": PASSWORD })
                    .to_string(),
            ))?,
    )
    .await?;
    let cookie = response.cookie.context("registration must set a cookie")?;
    let csrf_response = get(app, "/api/v1/auth/csrf", &cookie).await?;
    Ok(Session {
        cookie,
        csrf: csrf_response.body["csrfToken"]
            .as_str()
            .context("CSRF response must contain a token")?
            .to_owned(),
    })
}

async fn mutate(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    session: &Session,
) -> Result<TestResponse> {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::ORIGIN, ORIGIN)
        .header(header::COOKIE, cookie_value(&session.cookie))
        .header("x-csrf-token", &session.csrf)
        .header(header::CONTENT_TYPE, "application/json")
        .body(if body.is_null() {
            Body::empty()
        } else {
            Body::from(body.to_string())
        })?;
    send(app, request).await
}

async fn get(app: &Router, path: &str, cookie: &str) -> Result<TestResponse> {
    send(
        app,
        Request::get(path)
            .header(header::COOKIE, cookie_value(cookie))
            .body(Body::empty())?,
    )
    .await
}

async fn send(app: &Router, request: Request<Body>) -> Result<TestResponse> {
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    Ok(TestResponse {
        status,
        body,
        cookie,
    })
}

fn cookie_value(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap_or(set_cookie)
}

fn auth_settings() -> AuthSettings {
    AuthSettings {
        secure_cookie: false,
        hmac_key: b"holdings-test-key-with-at-least-thirty-two-bytes".to_vec(),
        session_ttl: Duration::from_secs(3600),
        idle_ttl: Duration::from_secs(1800),
        touch_interval: Duration::from_secs(300),
        csrf_ttl: Duration::from_secs(300),
    }
}

struct TestResponse {
    status: StatusCode,
    body: Value,
    cookie: Option<String>,
}
