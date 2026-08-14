use std::{env, ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};
use bycard_api::{catalog_import, config, database, database_operations, tcgdex_import};

const DEFAULT_FIXTURE_PATH: &str = "../../fixtures/demo-catalog/catalog.json";

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let mut arguments = env::args_os().skip(1);
    let operation = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .context("usage: database-operations <migrate|import-demo|import-tcgdex|verify>")?;

    match operation.as_str() {
        "migrate" => migrate(arguments).await,
        "import-demo" => import_demo(arguments).await,
        "import-tcgdex" => import_tcgdex(arguments).await,
        "verify" => verify(arguments).await,
        _ => bail!("unknown database operation: {operation}"),
    }
}

async fn migrate(mut arguments: impl Iterator<Item = OsString>) -> Result<()> {
    reject_extra_arguments(&mut arguments)?;
    let database_url = config::migration_database_url_from_env()?;
    let roles = config::database_roles_from_env()?;
    let pool = database::connect(&database_url).await?;
    database_operations::migrate(&pool, roles.as_ref()).await?;
    println!("Database migrations applied successfully.");
    Ok(())
}

async fn import_demo(mut arguments: impl Iterator<Item = OsString>) -> Result<()> {
    let fixture_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_FIXTURE_PATH));
    reject_extra_arguments(&mut arguments)?;

    let fixture = catalog_import::load_fixture(&fixture_path)?;
    let database_url = config::database_url_from_env()?;
    let pool = database::connect(&database_url).await?;
    let summary = catalog_import::import_catalog(&pool, &fixture).await?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

async fn import_tcgdex(arguments: impl Iterator<Item = OsString>) -> Result<()> {
    let set_ids = catalog_set_ids(arguments)?;
    let catalog = tcgdex_import::fetch_catalog(&set_ids).await?;
    let database_url = config::database_url_from_env()?;
    let pool = database::connect(&database_url).await?;
    let summary = catalog_import::import_external_catalog(&pool, &catalog).await?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

async fn verify(arguments: impl Iterator<Item = OsString>) -> Result<()> {
    let set_ids = catalog_set_ids(arguments)?;
    let expected_catalog_keys = if set_ids.is_empty() {
        Vec::new()
    } else {
        tcgdex_import::external_set_keys(&set_ids)?
    };
    let database_url = config::database_url_from_env()?;
    let pool = database::connect(&database_url).await?;
    let summary = database_operations::verify(&pool, &expected_catalog_keys).await?;
    println!(
        "Database verified: {} migrations, {} published sets, {} published cards.",
        summary.applied_migrations, summary.published_sets, summary.published_cards
    );
    Ok(())
}

fn catalog_set_ids(arguments: impl Iterator<Item = OsString>) -> Result<Vec<String>> {
    let argument_ids = arguments
        .map(|value| {
            value
                .into_string()
                .map_err(|_| anyhow::anyhow!("catalog set IDs must be valid Unicode"))
        })
        .collect::<Result<Vec<_>>>()?;
    if !argument_ids.is_empty() {
        return Ok(argument_ids);
    }

    match env::var("TCGDEX_SET_IDS") {
        Ok(value) => Ok(value.split_ascii_whitespace().map(str::to_owned).collect()),
        Err(env::VarError::NotPresent) => Ok(Vec::new()),
        Err(error) => Err(error).context("TCGDEX_SET_IDS is not valid Unicode"),
    }
}

fn reject_extra_arguments(arguments: &mut impl Iterator<Item = OsString>) -> Result<()> {
    if arguments.next().is_some() {
        bail!("this database operation does not accept arguments");
    }
    Ok(())
}
