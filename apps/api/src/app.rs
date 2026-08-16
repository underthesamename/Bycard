use std::time::Duration;

use axum::{
    Router,
    extract::FromRef,
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    middleware,
    routing::get,
};
use sqlx::PgPool;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
};

use crate::{auth, catalog, collections, health, profile, request_context};

pub use crate::auth::AuthSettings;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub(crate) auth: auth::AuthService,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

pub fn build_router(
    pool: PgPool,
    web_origin: HeaderValue,
    auth_settings: AuthSettings,
) -> anyhow::Result<Router> {
    let auth = auth::AuthService::new(auth_settings, web_origin.clone())?;
    let state = AppState { pool, auth };
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(web_origin))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(true);

    let standard_api = Router::new()
        .merge(catalog::router())
        .merge(auth::router())
        .merge(collections::router())
        .merge(profile::router())
        .layer(RequestBodyLimitLayer::new(16 * 1024));
    let avatar_api = profile::avatar_router()
        .layer(RequestBodyLimitLayer::new(profile::MAX_AVATAR_UPLOAD_BYTES));

    Ok(Router::new()
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .nest("/api/v1", standard_api.merge(avatar_api))
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(15),
        ))
        .layer(cors)
        .layer(middleware::from_fn(request_context::attach_request_id)))
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::{AuthSettings, build_router};

    #[tokio::test]
    async fn liveness_does_not_depend_on_the_database() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://bycard:bycard@127.0.0.1:1/bycard")
            .expect("test database URL should be valid");
        let web_origin = "http://localhost:3000"
            .parse()
            .expect("the test web origin must be valid");
        let app = build_router(pool, web_origin, test_auth_settings())
            .expect("test auth settings must be valid");

        let response = app
            .oneshot(
                Request::get("/health/live")
                    .body(Body::empty())
                    .expect("the liveness request must be valid"),
            )
            .await
            .expect("the router must answer the liveness request");

        assert_eq!(response.status(), 200);
        assert!(response.headers().contains_key("x-request-id"));
    }

    fn test_auth_settings() -> AuthSettings {
        AuthSettings {
            secure_cookie: false,
            hmac_key: b"test-key-with-at-least-thirty-two-bytes".to_vec(),
            session_ttl: std::time::Duration::from_secs(3600),
            idle_ttl: std::time::Duration::from_secs(1800),
            touch_interval: std::time::Duration::from_secs(300),
            csrf_ttl: std::time::Duration::from_secs(300),
        }
    }
}
