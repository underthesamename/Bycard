use std::env;

use anyhow::Result;
use bycard_api::{catalog_import, config, database, tcgdex_import};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = config::database_url_from_env()?;
    let set_ids = env::args().skip(1).collect::<Vec<_>>();
    let catalog = tcgdex_import::fetch_catalog(&set_ids).await?;
    let pool = database::connect(&database_url).await?;
    let summary = catalog_import::import_external_catalog(&pool, &catalog).await?;

    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
