use std::{collections::HashSet, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

const MAX_FIXTURE_BYTES: u64 = 2 * 1024 * 1024;
const EXPECTED_SET_COUNT: usize = 2;
const EXPECTED_CARD_COUNT: usize = 18;
const ALLOWED_RARITIES: [&str; 4] = ["comum", "incomum", "rara", "especial"];

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogFixture {
    pub schema_version: u32,
    pub game: FixtureGame,
    pub sets: Vec<FixtureSet>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureGame {
    pub slug: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureSet {
    pub external_key: String,
    pub slug: String,
    pub name: String,
    pub series_name: Option<String>,
    pub release_date: String,
    pub total_cards: i32,
    pub cover_image_url: Option<String>,
    pub language: String,
    pub is_published: bool,
    pub cards: Vec<FixtureCard>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureCard {
    pub external_key: String,
    pub local_number: String,
    pub printed_number: String,
    pub name: String,
    pub rarity: Option<String>,
    pub artist: Option<String>,
    pub image_small_url: Option<String>,
    pub image_large_url: Option<String>,
    pub sort_order: i32,
    pub is_published: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ChangeCounts {
    pub created: u32,
    pub updated: u32,
    pub unchanged: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportSummary {
    pub games: ChangeCounts,
    pub sets: ChangeCounts,
    pub cards: ChangeCounts,
}

#[derive(Clone, Copy)]
enum Change {
    Created,
    Updated,
    Unchanged,
}

struct UpsertResult {
    id: Uuid,
    change: Change,
}

pub fn load_fixture(path: &Path) -> Result<CatalogFixture> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect fixture at {}", path.display()))?;
    if !metadata.is_file() {
        bail!("fixture path must point to a regular file");
    }
    if metadata.len() > MAX_FIXTURE_BYTES {
        bail!("fixture exceeds the 2 MiB size limit");
    }

    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read fixture at {}", path.display()))?;
    parse_fixture(&source)
}

pub fn parse_fixture(source: &str) -> Result<CatalogFixture> {
    let fixture: CatalogFixture =
        serde_json::from_str(source).context("fixture is not valid catalog JSON")?;
    validate_fixture(&fixture)?;
    Ok(fixture)
}

pub fn validate_fixture(fixture: &CatalogFixture) -> Result<()> {
    if fixture.schema_version != 1 {
        bail!(
            "unsupported fixture schemaVersion: {}",
            fixture.schema_version
        );
    }
    validate_key("game.slug", &fixture.game.slug)?;
    validate_text("game.name", &fixture.game.name, 120)?;

    if fixture.sets.len() != EXPECTED_SET_COUNT {
        bail!("demo catalog must contain exactly {EXPECTED_SET_COUNT} sets");
    }

    let mut set_keys = HashSet::new();
    let mut set_slugs = HashSet::new();
    for set in &fixture.sets {
        if !set_keys.insert(set.external_key.as_str()) {
            bail!("duplicate set externalKey: {}", set.external_key);
        }
        if !set_slugs.insert(set.slug.as_str()) {
            bail!("duplicate set slug: {}", set.slug);
        }
        validate_set(set)?;
    }

    Ok(())
}

fn validate_set(set: &FixtureSet) -> Result<()> {
    validate_key("set.externalKey", &set.external_key)?;
    validate_key("set.slug", &set.slug)?;
    validate_text("set.name", &set.name, 160)?;
    validate_optional_text("set.seriesName", set.series_name.as_deref(), 160)?;
    validate_date(&set.release_date)?;
    validate_asset_path("set.coverImageUrl", set.cover_image_url.as_deref())?;
    if set.language != "pt-BR" {
        bail!("demo set language must be pt-BR");
    }
    if set.cards.len() != EXPECTED_CARD_COUNT || set.total_cards != EXPECTED_CARD_COUNT as i32 {
        bail!(
            "set {} must declare and contain exactly {EXPECTED_CARD_COUNT} cards",
            set.external_key
        );
    }

    let mut card_keys = HashSet::new();
    let mut local_numbers = HashSet::new();
    let mut sort_orders = HashSet::new();
    for card in &set.cards {
        if !card_keys.insert(card.external_key.as_str()) {
            bail!(
                "duplicate card externalKey in {}: {}",
                set.external_key,
                card.external_key
            );
        }
        if !local_numbers.insert(card.local_number.as_str()) {
            bail!(
                "duplicate card localNumber in {}: {}",
                set.external_key,
                card.local_number
            );
        }
        if !sort_orders.insert(card.sort_order) {
            bail!(
                "duplicate card sortOrder in {}: {}",
                set.external_key,
                card.sort_order
            );
        }
        validate_card(set, card)?;
    }

    Ok(())
}

fn validate_card(set: &FixtureSet, card: &FixtureCard) -> Result<()> {
    validate_key("card.externalKey", &card.external_key)?;
    if !card
        .external_key
        .starts_with(&format!("{}-", set.external_key))
    {
        bail!("card externalKey must start with its set externalKey");
    }
    validate_number("card.localNumber", &card.local_number)?;
    validate_printed_number(&card.printed_number)?;
    validate_text("card.name", &card.name, 160)?;
    validate_optional_text("card.artist", card.artist.as_deref(), 120)?;
    if let Some(rarity) = card.rarity.as_deref()
        && !ALLOWED_RARITIES.contains(&rarity)
    {
        bail!("unsupported card rarity: {rarity}");
    }
    validate_asset_path("card.imageSmallUrl", card.image_small_url.as_deref())?;
    validate_asset_path("card.imageLargeUrl", card.image_large_url.as_deref())?;
    if card.image_large_url.is_some() && card.image_small_url.is_none() {
        bail!("imageLargeUrl requires imageSmallUrl");
    }
    if !(1..=EXPECTED_CARD_COUNT as i32).contains(&card.sort_order) {
        bail!("card sortOrder must be between 1 and {EXPECTED_CARD_COUNT}");
    }

    Ok(())
}

fn validate_key(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 80 {
        bail!("{field} must contain between 1 and 80 characters");
    }
    let segments_are_valid = value.split('-').all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    });
    if !segments_are_valid {
        bail!("{field} must use lowercase letters, digits and single hyphens");
    }
    Ok(())
}

fn validate_text(field: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim() != value || value.is_empty() || value.chars().count() > max_len {
        bail!("{field} must be trimmed and contain between 1 and {max_len} characters");
    }
    if value.chars().any(char::is_control) {
        bail!("{field} cannot contain control characters");
    }
    Ok(())
}

fn validate_optional_text(field: &str, value: Option<&str>, max_len: usize) -> Result<()> {
    if let Some(value) = value {
        validate_text(field, value, max_len)?;
    }
    Ok(())
}

fn validate_number(field: &str, value: &str) -> Result<()> {
    if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("{field} must contain exactly three digits");
    }
    Ok(())
}

fn validate_printed_number(value: &str) -> Result<()> {
    let Some((number, total)) = value.split_once('/') else {
        bail!("card.printedNumber must use NNN/NNN");
    };
    validate_number("card.printedNumber", number)?;
    validate_number("card.printedNumber", total)?;
    if total != "018" {
        bail!("card.printedNumber total must be 018");
    }
    Ok(())
}

fn validate_asset_path(field: &str, value: Option<&str>) -> Result<()> {
    let Some(path) = value else {
        return Ok(());
    };
    if path.len() > 240
        || !path.starts_with("/demo/placeholders/")
        || !path.ends_with(".svg")
        || path.contains("..")
        || path.contains(['?', '#', '\\'])
    {
        bail!("{field} must reference a local SVG under /demo/placeholders/");
    }
    Ok(())
}

fn validate_date(value: &str) -> Result<()> {
    let components: Vec<_> = value.split('-').collect();
    if components.len() != 3
        || components[0].len() != 4
        || components[1].len() != 2
        || components[2].len() != 2
    {
        bail!("set.releaseDate must use YYYY-MM-DD");
    }
    let year: u32 = components[0]
        .parse()
        .context("set.releaseDate has an invalid year")?;
    let month: u32 = components[1]
        .parse()
        .context("set.releaseDate has an invalid month")?;
    let day: u32 = components[2]
        .parse()
        .context("set.releaseDate has an invalid day")?;
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => bail!("set.releaseDate has an invalid month"),
    };
    if day == 0 || day > max_day {
        bail!("set.releaseDate has an invalid day");
    }
    Ok(())
}

pub async fn import_catalog(pool: &PgPool, fixture: &CatalogFixture) -> Result<ImportSummary> {
    validate_fixture(fixture)?;
    persist_catalog(pool, fixture).await
}

pub async fn import_external_catalog(
    pool: &PgPool,
    fixture: &CatalogFixture,
) -> Result<ImportSummary> {
    validate_external_catalog(fixture)?;
    persist_catalog(pool, fixture).await
}

fn validate_external_catalog(fixture: &CatalogFixture) -> Result<()> {
    if fixture.schema_version != 1 {
        bail!("unsupported catalog schemaVersion");
    }
    validate_key("game.slug", &fixture.game.slug)?;
    validate_text("game.name", &fixture.game.name, 120)?;
    if fixture.sets.is_empty() || fixture.sets.len() > 20 {
        bail!("external catalog must contain between 1 and 20 sets");
    }

    let mut set_keys = HashSet::new();
    for set in &fixture.sets {
        if !set_keys.insert(set.external_key.as_str()) {
            bail!("duplicate external set key");
        }
        validate_key("set.externalKey", &set.external_key)?;
        validate_key("set.slug", &set.slug)?;
        validate_text("set.name", &set.name, 160)?;
        validate_optional_text("set.seriesName", set.series_name.as_deref(), 160)?;
        validate_date(&set.release_date)?;
        validate_external_asset("set.coverImageUrl", set.cover_image_url.as_deref())?;
        if set.language != "en-US" {
            bail!("external physical catalog language must be en-US");
        }
        if set.cards.is_empty()
            || set.cards.len() > 500
            || set.total_cards != set.cards.len() as i32
        {
            bail!("external set card count is invalid");
        }

        let mut card_keys = HashSet::new();
        let mut local_numbers = HashSet::new();
        for card in &set.cards {
            if !card_keys.insert(card.external_key.as_str())
                || !local_numbers.insert(card.local_number.as_str())
            {
                bail!("external set contains duplicate cards");
            }
            validate_key("card.externalKey", &card.external_key)?;
            validate_text("card.localNumber", &card.local_number, 24)?;
            validate_text("card.printedNumber", &card.printed_number, 32)?;
            validate_text("card.name", &card.name, 160)?;
            validate_optional_text("card.rarity", card.rarity.as_deref(), 120)?;
            validate_optional_text("card.artist", card.artist.as_deref(), 120)?;
            validate_external_asset("card.imageSmallUrl", card.image_small_url.as_deref())?;
            validate_external_asset("card.imageLargeUrl", card.image_large_url.as_deref())?;
            if card.sort_order < 1 || card.sort_order > 500 {
                bail!("external card sortOrder is invalid");
            }
        }
    }
    Ok(())
}

fn validate_external_asset(field: &str, value: Option<&str>) -> Result<()> {
    let Some(url) = value else {
        return Ok(());
    };
    let is_official_collection_logo = field == "set.coverImageUrl"
        && url.starts_with("https://d1i787aglh9bmb.cloudfront.net/assets/img/me-expansions/")
        && url.ends_with(".png")
        && !url.contains(['?', '#', '\\']);
    if is_official_collection_logo {
        return Ok(());
    }
    if url.len() > 300
        || !url.starts_with("https://assets.tcgdex.net/")
        || url.contains(['?', '#', '\\'])
        || !url.ends_with(".webp")
    {
        bail!("{field} must be a TCGdex HTTPS WebP asset");
    }
    Ok(())
}

async fn persist_catalog(pool: &PgPool, fixture: &CatalogFixture) -> Result<ImportSummary> {
    let mut transaction = pool
        .begin()
        .await
        .context("failed to begin catalog import")?;
    let mut summary = ImportSummary::default();

    let game = upsert_game(&mut transaction, &fixture.game).await?;
    record_change(&mut summary.games, game.change);

    for set in &fixture.sets {
        let persisted_set = upsert_set(&mut transaction, game.id, set).await?;
        record_change(&mut summary.sets, persisted_set.change);

        for card in &set.cards {
            let change = upsert_card(&mut transaction, persisted_set.id, card).await?;
            record_change(&mut summary.cards, change);
        }
    }

    transaction
        .commit()
        .await
        .context("failed to commit catalog import")?;
    Ok(summary)
}

fn record_change(counts: &mut ChangeCounts, change: Change) {
    match change {
        Change::Created => counts.created += 1,
        Change::Updated => counts.updated += 1,
        Change::Unchanged => counts.unchanged += 1,
    }
}

async fn upsert_game(
    transaction: &mut Transaction<'_, Postgres>,
    game: &FixtureGame,
) -> Result<UpsertResult> {
    let new_id = Uuid::now_v7();
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO games (id, slug, name, is_active) VALUES ($1, $2, $3, $4) ON CONFLICT (slug) DO NOTHING RETURNING id",
    )
    .bind(new_id)
    .bind(&game.slug)
    .bind(&game.name)
    .bind(game.is_active)
    .fetch_optional(&mut **transaction)
    .await
    .context("failed to insert catalog game")?;
    if let Some(id) = inserted {
        return Ok(UpsertResult {
            id,
            change: Change::Created,
        });
    }

    let (id, name, is_active) = sqlx::query_as::<_, (Uuid, String, bool)>(
        "SELECT id, name, is_active FROM games WHERE slug = $1",
    )
    .bind(&game.slug)
    .fetch_one(&mut **transaction)
    .await
    .context("failed to read existing catalog game")?;
    if name == game.name && is_active == game.is_active {
        return Ok(UpsertResult {
            id,
            change: Change::Unchanged,
        });
    }

    sqlx::query("UPDATE games SET name = $2, is_active = $3 WHERE id = $1")
        .bind(id)
        .bind(&game.name)
        .bind(game.is_active)
        .execute(&mut **transaction)
        .await
        .context("failed to update catalog game")?;
    Ok(UpsertResult {
        id,
        change: Change::Updated,
    })
}

async fn upsert_set(
    transaction: &mut Transaction<'_, Postgres>,
    game_id: Uuid,
    set: &FixtureSet,
) -> Result<UpsertResult> {
    let new_id = Uuid::now_v7();
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO sets (id, game_id, external_key, slug, name, series_name, release_date, total_cards, cover_image_url, language, is_published) VALUES ($1, $2, $3, $4, $5, $6, $7::date, $8, $9, $10, $11) ON CONFLICT (game_id, external_key, language) DO NOTHING RETURNING id",
    )
    .bind(new_id).bind(game_id).bind(&set.external_key).bind(&set.slug).bind(&set.name)
    .bind(&set.series_name).bind(&set.release_date).bind(set.total_cards).bind(&set.cover_image_url)
    .bind(&set.language).bind(set.is_published)
    .fetch_optional(&mut **transaction).await.context("failed to insert catalog set")?;
    if let Some(id) = inserted {
        return Ok(UpsertResult {
            id,
            change: Change::Created,
        });
    }

    let existing = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, i32, Option<String>, bool)>(
        "SELECT id, slug, name, series_name, release_date::text, total_cards, cover_image_url, is_published FROM sets WHERE game_id = $1 AND external_key = $2 AND language = $3",
    )
    .bind(game_id).bind(&set.external_key).bind(&set.language)
    .fetch_one(&mut **transaction).await.context("failed to read existing catalog set")?;
    let unchanged = existing.1 == set.slug
        && existing.2 == set.name
        && existing.3 == set.series_name
        && existing.4 == set.release_date
        && existing.5 == set.total_cards
        && existing.6 == set.cover_image_url
        && existing.7 == set.is_published;
    if unchanged {
        return Ok(UpsertResult {
            id: existing.0,
            change: Change::Unchanged,
        });
    }

    sqlx::query("UPDATE sets SET slug = $2, name = $3, series_name = $4, release_date = $5::date, total_cards = $6, cover_image_url = $7, is_published = $8, updated_at = NOW() WHERE id = $1")
        .bind(existing.0).bind(&set.slug).bind(&set.name).bind(&set.series_name).bind(&set.release_date)
        .bind(set.total_cards).bind(&set.cover_image_url).bind(set.is_published)
        .execute(&mut **transaction).await.context("failed to update catalog set")?;
    Ok(UpsertResult {
        id: existing.0,
        change: Change::Updated,
    })
}

async fn upsert_card(
    transaction: &mut Transaction<'_, Postgres>,
    set_id: Uuid,
    card: &FixtureCard,
) -> Result<Change> {
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO cards (id, set_id, external_key, local_number, printed_number, name, rarity, artist, image_small_url, image_large_url, sort_order, is_published) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (set_id, external_key) DO NOTHING RETURNING id",
    )
    .bind(Uuid::now_v7()).bind(set_id).bind(&card.external_key).bind(&card.local_number)
    .bind(&card.printed_number).bind(&card.name).bind(&card.rarity).bind(&card.artist)
    .bind(&card.image_small_url).bind(&card.image_large_url).bind(card.sort_order).bind(card.is_published)
    .fetch_optional(&mut **transaction).await.context("failed to insert catalog card")?;
    if inserted.is_some() {
        return Ok(Change::Created);
    }

    let existing = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, i32, bool)>(
        "SELECT id, local_number, printed_number, name, rarity, artist, image_small_url, image_large_url, sort_order, is_published FROM cards WHERE set_id = $1 AND external_key = $2",
    )
    .bind(set_id).bind(&card.external_key)
    .fetch_one(&mut **transaction).await.context("failed to read existing catalog card")?;
    let unchanged = existing.1 == card.local_number
        && existing.2 == card.printed_number
        && existing.3 == card.name
        && existing.4 == card.rarity
        && existing.5 == card.artist
        && existing.6 == card.image_small_url
        && existing.7 == card.image_large_url
        && existing.8 == card.sort_order
        && existing.9 == card.is_published;
    if unchanged {
        return Ok(Change::Unchanged);
    }

    sqlx::query("UPDATE cards SET local_number = $2, printed_number = $3, name = $4, rarity = $5, artist = $6, image_small_url = $7, image_large_url = $8, sort_order = $9, is_published = $10, updated_at = NOW() WHERE id = $1")
        .bind(existing.0).bind(&card.local_number).bind(&card.printed_number).bind(&card.name)
        .bind(&card.rarity).bind(&card.artist).bind(&card.image_small_url).bind(&card.image_large_url)
        .bind(card.sort_order).bind(card.is_published)
        .execute(&mut **transaction).await.context("failed to update catalog card")?;
    Ok(Change::Updated)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::parse_fixture;

    fn fixture_source() -> String {
        include_str!("../../../fixtures/demo-catalog/catalog.json").to_owned()
    }

    #[test]
    fn accepts_the_versioned_demo_fixture() {
        let fixture =
            parse_fixture(&fixture_source()).expect("the versioned fixture must be valid");
        assert_eq!(fixture.sets.len(), 2);
        assert!(fixture.sets.iter().all(|set| set.cards.len() == 18));
    }

    #[test]
    fn rejects_a_missing_required_field() {
        let mut document: Value =
            serde_json::from_str(&fixture_source()).expect("the test fixture must be valid JSON");
        document["sets"][0]["cards"][0]
            .as_object_mut()
            .expect("the test card must be an object")
            .remove("name");

        let mutated_source =
            serde_json::to_string(&document).expect("the mutated fixture must serialize");
        let error =
            parse_fixture(&mutated_source).expect_err("a missing card name must be rejected");
        assert!(error.to_string().contains("valid catalog JSON"));
    }

    #[test]
    fn rejects_duplicate_card_ids_inside_a_set() {
        let mut document: Value =
            serde_json::from_str(&fixture_source()).expect("the test fixture must be valid JSON");
        let duplicate = document["sets"][0]["cards"][0]["externalKey"].clone();
        document["sets"][0]["cards"][1]["externalKey"] = duplicate;

        let mutated_source =
            serde_json::to_string(&document).expect("the mutated fixture must serialize");
        let error =
            parse_fixture(&mutated_source).expect_err("duplicate card IDs must be rejected");
        assert!(error.to_string().contains("duplicate card externalKey"));
    }
}
