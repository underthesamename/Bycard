use anyhow::{Context, Result};
use bycard_api::{config, database};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = config::database_url_from_env()?;

    let pool = database::connect(&database_url).await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to apply database migrations")?;

    println!("Database migrations applied successfully.");
    Ok(())
}
