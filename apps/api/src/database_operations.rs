use anyhow::{Context, Result, bail};
use sqlx::{AssertSqlSafe, PgPool, migrate::Migrator};

use crate::config::DatabaseRoles;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, PartialEq, Eq)]
pub struct DatabaseSummary {
    pub applied_migrations: i64,
    pub published_sets: i64,
    pub published_cards: i64,
}

pub async fn migrate(pool: &PgPool, roles: Option<&DatabaseRoles>) -> Result<()> {
    MIGRATOR
        .run(pool)
        .await
        .context("failed to apply database migrations")?;

    if let Some(roles) = roles {
        configure_access(pool, roles).await?;
    }
    Ok(())
}

async fn configure_access(pool: &PgPool, roles: &DatabaseRoles) -> Result<()> {
    let application_role = quote_existing_role(pool, &roles.application).await?;
    let backup_role = quote_existing_role(pool, &roles.backup).await?;
    let mut transaction = pool
        .begin()
        .await
        .context("failed to begin database access configuration")?;

    for statement in [
        "REVOKE CREATE ON SCHEMA public FROM PUBLIC".to_owned(),
        "REVOKE ALL ON ALL TABLES IN SCHEMA public FROM PUBLIC".to_owned(),
        format!("REVOKE ALL ON ALL TABLES IN SCHEMA public FROM {application_role}, {backup_role}"),
        format!("GRANT USAGE ON SCHEMA public TO {application_role}, {backup_role}"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {application_role}"
        ),
        format!("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {backup_role}"),
        format!(
            "REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER ON TABLE _sqlx_migrations FROM {application_role}"
        ),
        "ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES FROM PUBLIC".to_owned(),
        format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {application_role}"
        ),
        format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO {backup_role}"
        ),
    ] {
        // PostgreSQL does not bind identifiers. Both role names were allowlisted and
        // quoted by quote_ident before reaching these fixed grant templates.
        sqlx::query(AssertSqlSafe(statement.as_str()))
            .execute(&mut *transaction)
            .await
            .context("failed to configure database role privileges")?;
    }

    transaction
        .commit()
        .await
        .context("failed to commit database role privileges")
}

async fn quote_existing_role(pool: &PgPool, role: &str) -> Result<String> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = $1)")
            .bind(role)
            .fetch_one(pool)
            .await
            .context("failed to inspect database roles")?;
    if !exists {
        bail!("required database role does not exist: {role}");
    }

    sqlx::query_scalar("SELECT quote_ident($1)")
        .bind(role)
        .fetch_one(pool)
        .await
        .context("failed to quote database role")
}

pub async fn verify(pool: &PgPool, expected_catalog_keys: &[String]) -> Result<DatabaseSummary> {
    let expected_migrations = i64::try_from(MIGRATOR.iter().count())
        .context("migration count exceeded the supported range")?;
    let applied = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations WHERE success = TRUE ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .context("failed to inspect applied migrations")?;
    let applied_migrations = i64::try_from(applied.len())
        .context("applied migration count exceeded the supported range")?;
    if applied_migrations != expected_migrations {
        bail!(
            "database has {applied_migrations} successful migrations; expected {expected_migrations}"
        );
    }
    for (expected, actual) in MIGRATOR.iter().zip(&applied) {
        if expected.version != actual.0 || expected.checksum.as_ref() != actual.1 {
            bail!(
                "database migration {} does not match this release",
                actual.0
            );
        }
    }

    let failed_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = FALSE")
            .fetch_one(pool)
            .await
            .context("failed to inspect failed migrations")?;
    if failed_migrations != 0 {
        bail!("database contains a failed migration");
    }

    let (published_sets, published_cards): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(DISTINCT s.id), COUNT(c.id) FROM sets s JOIN cards c ON c.set_id = s.id AND c.is_published = TRUE WHERE s.is_published = TRUE",
    )
    .fetch_one(pool)
    .await
    .context("failed to inspect the published catalog")?;
    if published_sets == 0 || published_cards == 0 {
        bail!("database does not contain a published catalog");
    }

    let inconsistent_sets: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sets s WHERE s.is_published = TRUE AND s.total_cards <> (SELECT COUNT(*) FROM cards c WHERE c.set_id = s.id AND c.is_published = TRUE)",
    )
    .fetch_one(pool)
    .await
    .context("failed to validate catalog card counts")?;
    if inconsistent_sets != 0 {
        bail!("published catalog contains inconsistent card counts");
    }

    if !expected_catalog_keys.is_empty() {
        let matching_sets: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sets WHERE is_published = TRUE AND external_key = ANY($1)",
        )
        .bind(expected_catalog_keys)
        .fetch_one(pool)
        .await
        .context("failed to validate the expected catalog")?;
        let expected_set_count = i64::try_from(expected_catalog_keys.len())
            .context("expected catalog count exceeded the supported range")?;
        if matching_sets != expected_set_count {
            bail!(
                "database contains {matching_sets} of {expected_set_count} expected catalog sets"
            );
        }
    }

    Ok(DatabaseSummary {
        applied_migrations,
        published_sets,
        published_cards,
    })
}
