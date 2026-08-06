use std::env;

use anyhow::{Context, Result, bail};
use bycard_api::database;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    if database_url.trim().is_empty() {
        bail!("DATABASE_URL cannot be empty");
    }

    let pool = database::connect(&database_url).await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to apply database migrations")?;

    println!("Database migrations applied successfully.");
    Ok(())
}
