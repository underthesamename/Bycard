use std::{env, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use bycard_api::{
    app::{AuthSettings, build_router},
    catalog_import,
};
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool, postgres::PgConnectOptions};
use tower::ServiceExt;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn catalog_http_contract_uses_published_postgres_data() -> Result<()> {
    dotenvy::dotenv().ok();
    let admin_url = env::var("TEST_DATABASE_URL").context(
        "TEST_DATABASE_URL is required for PostgreSQL integration tests; start the test database first",
    )?;
    let admin_pool = PgPool::connect(&admin_url).await?;
    let database_name = format!("bycard_api_test_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database_name}")))
        .execute(&admin_pool)
        .await?;

    let options = PgConnectOptions::from_str(&admin_url)?.database(&database_name);
    let pool = PgPool::connect_with(options).await?;
    let result = exercise_catalog_contract(&pool).await;
    pool.close().await;

    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE {database_name} WITH (FORCE)"
    )))
    .execute(&admin_pool)
    .await?;
    admin_pool.close().await;
    result
}

async fn exercise_catalog_contract(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await?;
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo-catalog/catalog.json");
    let fixture = catalog_import::load_fixture(&fixture_path)?;
    catalog_import::import_catalog(pool, &fixture).await?;
    let (empty_set_id, first_set_id) = add_visibility_scenarios(pool).await?;

    let origin = "http://localhost:3000".parse()?;
    let app = build_router(pool.clone(), origin, test_auth_settings())?;

    let (status, request_id, body) = get_json(&app, "/api/v1/sets").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(!request_id.is_empty());
    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["pageSize"], 20);
    assert_eq!(body["pagination"]["totalItems"], 3);
    assert!(
        body["data"]
            .as_array()
            .is_some_and(|sets| { sets.iter().all(|set| set["name"] != "Coleção Oculta") })
    );

    let (status, _, detail) = get_json(&app, &format!("/api/v1/sets/{first_set_id}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["data"]["name"], "Horizonte Solar");

    assert_error(
        &app,
        "/api/v1/sets/id-invalido",
        StatusCode::BAD_REQUEST,
        "invalid_id",
    )
    .await?;
    assert_error(
        &app,
        &format!("/api/v1/sets/{}", Uuid::now_v7()),
        StatusCode::NOT_FOUND,
        "catalog_not_found",
    )
    .await?;

    let cards_path = format!("/api/v1/sets/{first_set_id}/cards");
    let (status, _, cards) = get_json(&app, &cards_path).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cards["pagination"]["totalItems"], 18);
    assert_eq!(cards["data"][0]["sortOrder"], 1);
    assert_eq!(cards["data"][17]["sortOrder"], 18);
    assert!(
        cards["data"]
            .as_array()
            .is_some_and(|cards| { cards.iter().all(|card| card["name"] != "Carta Oculta") })
    );

    let (_, _, by_name) = get_json(
        &app,
        &format!("{cards_path}?search=Primeiro%20%20%20Clar%C3%A3o"),
    )
    .await?;
    assert_eq!(by_name["pagination"]["totalItems"], 1);
    let (_, _, by_number) = get_json(&app, &format!("{cards_path}?search=016")).await?;
    assert_eq!(by_number["pagination"]["totalItems"], 1);
    assert!(by_number["data"][0]["imageSmallUrl"].is_null());
    assert!(by_number["data"][0]["imageLargeUrl"].is_null());

    let (_, _, hostile) = get_json(
        &app,
        &format!("{cards_path}?search=%25%27%20OR%201%3D1%20--"),
    )
    .await?;
    assert_eq!(hostile["pagination"]["totalItems"], 0);

    let (_, _, second_page) = get_json(&app, &format!("{cards_path}?page=2&pageSize=5")).await?;
    assert_eq!(second_page["data"].as_array().map(Vec::len), Some(5));
    assert_eq!(second_page["data"][0]["sortOrder"], 6);
    assert_error(
        &app,
        &format!("{cards_path}?pageSize=101"),
        StatusCode::BAD_REQUEST,
        "invalid_parameter",
    )
    .await?;
    assert_error(
        &app,
        &format!("{cards_path}?page=zero"),
        StatusCode::BAD_REQUEST,
        "invalid_parameter",
    )
    .await?;
    assert_error(
        &app,
        &format!("{cards_path}?sort=qualquer_coisa"),
        StatusCode::BAD_REQUEST,
        "invalid_parameter",
    )
    .await?;

    let (_, _, empty_cards) = get_json(&app, &format!("/api/v1/sets/{empty_set_id}/cards")).await?;
    assert_eq!(empty_cards["data"], Value::Array(Vec::new()));
    assert_eq!(empty_cards["pagination"]["totalPages"], 0);
    Ok(())
}

fn test_auth_settings() -> AuthSettings {
    AuthSettings {
        secure_cookie: false,
        hmac_key: b"catalog-test-key-with-at-least-thirty-two-bytes".to_vec(),
        session_ttl: std::time::Duration::from_secs(3600),
        idle_ttl: std::time::Duration::from_secs(1800),
        touch_interval: std::time::Duration::from_secs(300),
        csrf_ttl: std::time::Duration::from_secs(300),
    }
}

async fn add_visibility_scenarios(pool: &PgPool) -> Result<(Uuid, Uuid)> {
    let game_id: Uuid = sqlx::query_scalar("SELECT id FROM games WHERE slug = 'bycard-demo'")
        .fetch_one(pool)
        .await?;
    let first_set_id: Uuid =
        sqlx::query_scalar("SELECT id FROM sets WHERE external_key = 'hsl-01'")
            .fetch_one(pool)
            .await?;
    let empty_set_id = Uuid::now_v7();
    sqlx::query("INSERT INTO sets (id, game_id, external_key, slug, name, release_date, total_cards, language, is_published) VALUES ($1, $2, 'empty-01', 'arquivo-vazio', 'Arquivo Vazio', '2026-08-05', 0, 'pt-BR', TRUE)")
        .bind(empty_set_id)
        .bind(game_id)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO sets (id, game_id, external_key, slug, name, release_date, total_cards, language, is_published) VALUES ($1, $2, 'hidden-01', 'colecao-oculta', 'Coleção Oculta', '2026-08-05', 1, 'pt-BR', FALSE)")
        .bind(Uuid::now_v7())
        .bind(game_id)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO cards (id, set_id, external_key, local_number, printed_number, name, sort_order, is_published) VALUES ($1, $2, 'hsl-01-999', '999', '999/999', 'Carta Oculta', 999, FALSE)")
        .bind(Uuid::now_v7())
        .bind(first_set_id)
        .execute(pool)
        .await?;
    Ok((empty_set_id, first_set_id))
}

async fn get_json(app: &Router, path: &str) -> Result<(StatusCode, String, Value)> {
    let response = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty())?)
        .await?;
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .context("response must include x-request-id")?
        .to_str()?
        .to_owned();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    Ok((status, request_id, serde_json::from_slice(&bytes)?))
}

async fn assert_error(
    app: &Router,
    path: &str,
    expected_status: StatusCode,
    expected_code: &str,
) -> Result<()> {
    let (status, request_id, body) = get_json(app, path).await?;
    assert_eq!(status, expected_status);
    assert_eq!(body["error"]["code"], expected_code);
    assert_eq!(body["error"]["requestId"], request_id);
    Ok(())
}
