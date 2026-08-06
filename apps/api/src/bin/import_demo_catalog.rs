use std::{env, path::PathBuf};

use anyhow::{Context, Result, bail};
use bycard_api::{catalog_import, database};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    if database_url.trim().is_empty() {
        bail!("DATABASE_URL cannot be empty");
    }

    let fixture_path = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo-catalog/catalog.json")
    });
    let fixture = catalog_import::load_fixture(&fixture_path)?;
    let pool = database::connect(&database_url).await?;
    let summary = catalog_import::import_catalog(&pool, &fixture).await?;

    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
