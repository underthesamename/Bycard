use std::{env, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use bycard_api::catalog_import::{self, ChangeCounts};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgConnectOptions};
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn catalog_import_is_idempotent_transactional_and_constrained() -> Result<()> {
    dotenvy::dotenv().ok();
    let admin_url = env::var("TEST_DATABASE_URL").context(
        "TEST_DATABASE_URL is required for PostgreSQL integration tests; start the test database first",
    )?;
    let admin_pool = PgPool::connect(&admin_url).await?;
    let database_name = format!("bycard_test_{}", Uuid::now_v7().simple());

    // The identifier is generated internally from UUID hex, so it cannot contain SQL syntax.
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database_name}")))
        .execute(&admin_pool)
        .await?;

    let options = PgConnectOptions::from_str(&admin_url)?.database(&database_name);
    let pool = PgPool::connect_with(options).await?;
    let result = exercise_import_contract(&pool).await;
    pool.close().await;

    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE {database_name} WITH (FORCE)"
    )))
    .execute(&admin_pool)
    .await?;
    admin_pool.close().await;

    result
}

async fn exercise_import_contract(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await?;
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo-catalog/catalog.json");
    let fixture = catalog_import::load_fixture(&fixture_path)?;

    install_controlled_failure_trigger(pool).await?;
    let mut failing_fixture = fixture.clone();
    failing_fixture.sets[0].cards[9].name = "Falha Controlada".to_owned();
    assert!(
        catalog_import::import_catalog(pool, &failing_fixture)
            .await
            .is_err()
    );
    assert_catalog_counts(pool, (0, 0, 0)).await?;
    remove_controlled_failure_trigger(pool).await?;

    let first = catalog_import::import_catalog(pool, &fixture).await?;
    assert_eq!(first.games.created, 1);
    assert_eq!(first.sets.created, 2);
    assert_eq!(first.cards.created, 36);
    assert_catalog_counts(pool, (1, 2, 36)).await?;

    let second = catalog_import::import_catalog(pool, &fixture).await?;
    assert_eq!(
        second.games,
        ChangeCounts {
            created: 0,
            updated: 0,
            unchanged: 1
        }
    );
    assert_eq!(
        second.sets,
        ChangeCounts {
            created: 0,
            updated: 0,
            unchanged: 2
        }
    );
    assert_eq!(
        second.cards,
        ChangeCounts {
            created: 0,
            updated: 0,
            unchanged: 36
        }
    );
    assert_catalog_counts(pool, (1, 2, 36)).await?;

    let mut changed_fixture = fixture.clone();
    changed_fixture.sets[0].cards[0].name = "Clarão Renovado".to_owned();
    let changed = catalog_import::import_catalog(pool, &changed_fixture).await?;
    assert_eq!(changed.cards.updated, 1);
    assert_eq!(changed.cards.unchanged, 35);
    let persisted_name: String =
        sqlx::query_scalar("SELECT name FROM cards WHERE external_key = 'hsl-01-001'")
            .fetch_one(pool)
            .await?;
    assert_eq!(persisted_name, "Clarão Renovado");
    assert_catalog_counts(pool, (1, 2, 36)).await?;

    assert_database_constraints(pool).await?;
    let generated_id: Uuid = sqlx::query_scalar("SELECT id FROM cards LIMIT 1")
        .fetch_one(pool)
        .await?;
    assert_eq!(generated_id.get_version_num(), 7);
    Ok(())
}

async fn install_controlled_failure_trigger(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(
        "CREATE FUNCTION reject_controlled_card() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.name = 'Falha Controlada' THEN RAISE EXCEPTION 'controlled import failure'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_controlled_card BEFORE INSERT OR UPDATE ON cards FOR EACH ROW EXECUTE FUNCTION reject_controlled_card();",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn remove_controlled_failure_trigger(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(
        "DROP TRIGGER reject_controlled_card ON cards; DROP FUNCTION reject_controlled_card();",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn assert_catalog_counts(pool: &PgPool, expected: (i64, i64, i64)) -> Result<()> {
    let games: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
        .fetch_one(pool)
        .await?;
    let sets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sets")
        .fetch_one(pool)
        .await?;
    let cards: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards")
        .fetch_one(pool)
        .await?;
    assert_eq!((games, sets, cards), expected);
    Ok(())
}

async fn assert_database_constraints(pool: &PgPool) -> Result<()> {
    let card_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM cards WHERE set_id = (SELECT id FROM sets WHERE external_key = 'hsl-01') ORDER BY sort_order LIMIT 2",
    )
    .fetch_all(pool)
    .await?;
    let negative_order = sqlx::query("UPDATE cards SET sort_order = -1 WHERE id = $1")
        .bind(card_ids[0])
        .execute(pool)
        .await;
    assert!(negative_order.is_err());

    let duplicate_order = sqlx::query(
        "UPDATE cards SET sort_order = (SELECT sort_order FROM cards WHERE id = $2) WHERE id = $1",
    )
    .bind(card_ids[0])
    .bind(card_ids[1])
    .execute(pool)
    .await;
    assert!(duplicate_order.is_err());
    Ok(())
}
