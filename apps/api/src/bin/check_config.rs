use anyhow::{Context, Result};
use bycard_api::config::Config;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    Config::from_env().context("invalid application configuration")?;
    println!("Application configuration is valid.");
    Ok(())
}
