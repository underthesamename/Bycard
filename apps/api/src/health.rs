use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct HealthStatus {
    status: &'static str,
}

pub async fn liveness() -> impl IntoResponse {
    Json(HealthStatus { status: "ok" })
}

pub async fn readiness(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(1) => (StatusCode::OK, Json(HealthStatus { status: "ready" })),
        Ok(_) | Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthStatus {
                status: "unavailable",
            }),
        ),
    }
}
