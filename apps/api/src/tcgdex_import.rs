use std::{collections::HashSet, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::catalog_import::{CatalogFixture, FixtureCard, FixtureGame, FixtureSet};

const API_BASE_URL: &str = "https://api.tcgdex.net/v2/en/sets/";
const PITCH_BLACK_LOGO_URL: &str =
    "https://d1i787aglh9bmb.cloudfront.net/assets/img/me-expansions/me05/logo/pt-br/me05-logo.png";
const MAX_RESPONSE_BYTES: usize = 6 * 1024 * 1024;
const MAX_SET_COUNT: usize = 10;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TcgdexSet {
    id: String,
    name: String,
    release_date: String,
    logo: Option<String>,
    serie: TcgdexSerie,
    cards: Vec<TcgdexCard>,
}

#[derive(Debug, Deserialize)]
struct TcgdexSerie {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TcgdexCard {
    id: String,
    local_id: String,
    name: String,
    image: Option<String>,
}

pub async fn fetch_catalog(set_ids: &[String]) -> Result<CatalogFixture> {
    validate_set_ids(set_ids)?;
    let client = Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(20))
        .user_agent("Bycard catalog importer/0.1")
        .build()
        .context("failed to build TCGdex client")?;

    let mut sets = Vec::with_capacity(set_ids.len());
    for set_id in set_ids {
        sets.push(fetch_set(&client, set_id).await?);
    }

    Ok(CatalogFixture {
        schema_version: 1,
        game: FixtureGame {
            slug: "pokemon-tcg".to_owned(),
            name: "Pokémon TCG".to_owned(),
            is_active: true,
        },
        sets,
    })
}

pub fn external_set_keys(set_ids: &[String]) -> Result<Vec<String>> {
    validate_set_ids(set_ids)?;
    set_ids
        .iter()
        .map(|set_id| normalize_key(set_id).map(|key| format!("tcgdex-{key}")))
        .collect()
}

fn validate_set_ids(set_ids: &[String]) -> Result<()> {
    if set_ids.is_empty() || set_ids.len() > MAX_SET_COUNT {
        bail!("provide between 1 and {MAX_SET_COUNT} TCGdex set IDs");
    }
    let mut unique_ids = HashSet::new();
    for set_id in set_ids {
        if set_id.is_empty()
            || set_id.len() > 24
            || !set_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
            || !unique_ids.insert(set_id)
        {
            bail!("invalid or duplicate TCGdex set ID: {set_id}");
        }
    }
    Ok(())
}

async fn fetch_set(client: &Client, set_id: &str) -> Result<FixtureSet> {
    let url = Url::parse(API_BASE_URL)
        .context("TCGdex base URL is invalid")?
        .join(set_id)
        .context("failed to build TCGdex set URL")?;
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to request TCGdex set {set_id}"))?
        .error_for_status()
        .with_context(|| format!("TCGdex rejected set {set_id}"))?;
    let body = response
        .bytes()
        .await
        .with_context(|| format!("failed to read TCGdex set {set_id}"))?;
    if body.len() > MAX_RESPONSE_BYTES {
        bail!("TCGdex set {set_id} exceeded the response size limit");
    }
    let source: TcgdexSet = serde_json::from_slice(&body)
        .with_context(|| format!("TCGdex returned an invalid set {set_id}"))?;
    convert_set(source)
}

fn convert_set(source: TcgdexSet) -> Result<FixtureSet> {
    if source.serie.id == "tcgp" {
        bail!("TCGdex set {} belongs to Pokemon TCG Pocket", source.id);
    }
    let set_key = normalize_key(&source.id)?;
    let total_cards = i32::try_from(source.cards.len()).context("set has too many cards")?;
    let printed_total = source.cards.len();
    let cards = source
        .cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| convert_card(card, printed_total, index + 1))
        .collect::<Result<Vec<_>>>()?;

    let set_name = localized_set_name(&source.id)
        .unwrap_or(&source.name)
        .to_owned();
    let series_name = localized_series_name(&source.serie.id)
        .unwrap_or(&source.serie.name)
        .to_owned();

    Ok(FixtureSet {
        external_key: format!("tcgdex-{set_key}"),
        slug: format!("tcgdex-{set_key}"),
        name: set_name,
        series_name: Some(series_name),
        release_date: source.release_date,
        total_cards,
        cover_image_url: collection_cover(&source.id, source.logo.as_deref()),
        language: "en-US".to_owned(),
        is_published: true,
        cards,
    })
}

fn collection_cover(set_id: &str, source_logo: Option<&str>) -> Option<String> {
    if set_id == "me05" {
        return Some(PITCH_BLACK_LOGO_URL.to_owned());
    }

    source_logo.map(|url| format!("{url}.webp"))
}

fn localized_set_name(set_id: &str) -> Option<&'static str> {
    match set_id {
        "me05" => Some("Escuridão Absoluta"),
        "sv09" => Some("Amigos de Jornada"),
        "sv08.5" => Some("Evoluções Prismáticas"),
        _ => None,
    }
}

fn localized_series_name(series_id: &str) -> Option<&'static str> {
    match series_id {
        "me" => Some("Megaevolução"),
        "swsh" => Some("Espada e Escudo"),
        "sv" => Some("Escarlate e Violeta"),
        _ => None,
    }
}

fn convert_card(card: TcgdexCard, printed_total: usize, sort_order: usize) -> Result<FixtureCard> {
    let card_key = normalize_key(&card.id)?;
    let sort_order = i32::try_from(sort_order).context("card sort order overflowed")?;
    let image_small_url = card.image.as_ref().map(|url| format!("{url}/low.webp"));
    let image_large_url = card.image.map(|url| format!("{url}/high.webp"));
    Ok(FixtureCard {
        external_key: format!("tcgdex-{card_key}"),
        local_number: card.local_id.clone(),
        printed_number: format!("{}/{printed_total}", card.local_id),
        name: card.name,
        rarity: None,
        artist: None,
        image_small_url,
        image_large_url,
        sort_order,
        is_published: true,
    })
}

fn normalize_key(value: &str) -> Result<String> {
    let mut normalized = String::with_capacity(value.len());
    let mut previous_was_separator = false;
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() {
            normalized.push(char::from(byte.to_ascii_lowercase()));
            previous_was_separator = false;
        } else if matches!(byte, b'.' | b'-' | b'_') && !previous_was_separator {
            normalized.push('-');
            previous_was_separator = true;
        } else {
            bail!("TCGdex identifier contains unsupported characters");
        }
    }
    let normalized = normalized.trim_matches('-').to_owned();
    if normalized.is_empty() || normalized.len() > 64 {
        bail!("TCGdex identifier has an invalid length");
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_pocket_sets_and_unsafe_identifiers() {
        assert!(normalize_key("sv08.5").is_ok_and(|value| value == "sv08-5"));
        assert!(normalize_key("../../bad").is_err());
        assert!(validate_set_ids(&["A1".to_owned(), "A1".to_owned()]).is_err());
    }

    #[test]
    fn localizes_the_requested_collection_names() {
        assert_eq!(localized_set_name("me05"), Some("Escuridão Absoluta"));
        assert_eq!(localized_set_name("sv09"), Some("Amigos de Jornada"));
        assert_eq!(localized_set_name("sv08.5"), Some("Evoluções Prismáticas"));
        assert_eq!(localized_set_name("unknown"), None);
        assert_eq!(
            collection_cover("me05", Some("https://invalid.example/logo")),
            Some(PITCH_BLACK_LOGO_URL.to_owned())
        );
    }
}
