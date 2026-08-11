use std::env;

use anyhow::{Context, Result, bail};
use bycard_api::{catalog_import, database, tcgdex_import};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    if database_url.trim().is_empty() {
        bail!("DATABASE_URL cannot be empty");
    }
    let set_ids = env::args().skip(1).collect::<Vec<_>>();
    let catalog = tcgdex_import::fetch_catalog(&set_ids).await?;
    let pool = database::connect(&database_url).await?;
    let summary = catalog_import::import_external_catalog(&pool, &catalog).await?;

    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
