use anyhow::{Context, Result};
use bycard_api::config::Config;
use tokio::{net::TcpListener, signal};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = Config::from_env().context("invalid application configuration")?;
    let pool = bycard_api::database::connect(&config.database_url).await?;
    let listener = TcpListener::bind(config.socket_address())
        .await
        .context("failed to bind API listener")?;

    info!(
        environment = %config.app_env,
        address = %listener.local_addr().context("failed to read listener address")?,
        "Bycard API started"
    );

    axum::serve(
        listener,
        bycard_api::app::build_router(pool, config.web_origin, config.auth)?,
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("API server stopped unexpectedly")
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install termination signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    warn!("shutdown signal received");
}
