use std::{env, net::IpAddr, time::Duration};

use anyhow::{Context, Result, bail};
use axum::http::HeaderValue;
use url::Url;

use crate::app::AuthSettings;

const SUPPORTED_ENVIRONMENTS: [&str; 3] = ["local", "test", "production"];
const MINIMUM_HMAC_KEY_BYTES: usize = 32;
const INSECURE_SECRET_MARKERS: [&str; 3] =
    ["local-only", "change-this", "__set_in_secret_manager__"];

#[derive(Clone)]
pub struct Config {
    pub app_env: String,
    pub bind_address: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub web_origin: HeaderValue,
    pub auth: AuthSettings,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DatabaseRoles {
    pub application: String,
    pub backup: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let app_env = required("APP_ENV")?;
        validate_environment(&app_env)?;

        let bind_address = required("API_HOST")?
            .parse()
            .context("API_HOST must be a valid IP address")?;
        let port = validate_port(&required("API_PORT")?)?;
        let database_url = validate_database_url(&app_env, &required("DATABASE_URL")?)?;

        let web_origin = validate_web_origin(&app_env, &required("WEB_ORIGIN")?)?;
        let hmac_key = validate_session_hmac_key(&app_env, &required("SESSION_HMAC_KEY")?)?;
        let session_ttl = duration("SESSION_TTL_SECONDS")?;
        let idle_ttl = duration("SESSION_IDLE_TTL_SECONDS")?;
        let touch_interval = duration("SESSION_TOUCH_INTERVAL_SECONDS")?;
        let csrf_ttl = duration("CSRF_TTL_SECONDS")?;
        let auth = AuthSettings {
            secure_cookie: app_env == "production",
            hmac_key,
            session_ttl,
            idle_ttl,
            touch_interval,
            csrf_ttl,
        };

        Ok(Self {
            app_env,
            bind_address,
            port,
            database_url,
            web_origin,
            auth,
        })
    }

    pub fn socket_address(&self) -> (IpAddr, u16) {
        (self.bind_address, self.port)
    }
}

pub fn database_url_from_env() -> Result<String> {
    let app_env = required("APP_ENV")?;
    validate_environment(&app_env)?;
    validate_database_url(&app_env, &required("DATABASE_URL")?)
}

pub fn migration_database_url_from_env() -> Result<String> {
    let app_env = required("APP_ENV")?;
    validate_environment(&app_env)?;

    let application_url = required("DATABASE_URL")?;
    validate_database_url(&app_env, &application_url)?;

    let migration_url = match env::var("DATABASE_MIGRATION_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        Ok(_) if app_env == "production" => bail!("DATABASE_MIGRATION_URL cannot be empty"),
        Ok(_) => application_url.clone(),
        Err(env::VarError::NotPresent) if app_env != "production" => application_url.clone(),
        Err(env::VarError::NotPresent) => bail!("DATABASE_MIGRATION_URL is required"),
        Err(error) => return Err(error).context("DATABASE_MIGRATION_URL is not valid Unicode"),
    };
    validate_database_url(&app_env, &migration_url)?;

    if app_env == "production" {
        validate_distinct_database_roles(&application_url, &migration_url)?;
    }

    Ok(migration_url)
}

pub fn database_roles_from_env() -> Result<Option<DatabaseRoles>> {
    let app_env = required("APP_ENV")?;
    validate_environment(&app_env)?;

    let backup_role = match env::var("DATABASE_BACKUP_ROLE") {
        Ok(value) if !value.trim().is_empty() => value,
        Ok(_) if app_env == "production" => bail!("DATABASE_BACKUP_ROLE cannot be empty"),
        Ok(_) => return Ok(None),
        Err(env::VarError::NotPresent) if app_env != "production" => return Ok(None),
        Err(env::VarError::NotPresent) => bail!("DATABASE_BACKUP_ROLE is required"),
        Err(error) => return Err(error).context("DATABASE_BACKUP_ROLE is not valid Unicode"),
    };

    let application_url =
        Url::parse(&database_url_from_env()?).context("DATABASE_URL must be a valid URL")?;
    let application_role = application_url.username().to_owned();
    validate_database_role("DATABASE_URL username", &application_role)?;
    validate_database_role("DATABASE_BACKUP_ROLE", &backup_role)?;
    if application_role == backup_role {
        bail!("application and backup database roles must be different");
    }

    Ok(Some(DatabaseRoles {
        application: application_role,
        backup: backup_role,
    }))
}

fn validate_environment(app_env: &str) -> Result<()> {
    if !SUPPORTED_ENVIRONMENTS.contains(&app_env) {
        bail!("APP_ENV must be one of: local, test, production");
    }
    Ok(())
}

fn validate_port(raw_port: &str) -> Result<u16> {
    let port = raw_port.parse().context("API_PORT must be a valid port")?;
    if port == 0 {
        bail!("API_PORT must be greater than zero");
    }
    Ok(port)
}

fn validate_database_url(app_env: &str, raw_url: &str) -> Result<String> {
    let url = Url::parse(raw_url).context("DATABASE_URL must be a valid URL")?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        bail!("DATABASE_URL must use the postgres or postgresql scheme");
    }
    if url.host_str().is_none() || url.path().trim_matches('/').is_empty() {
        bail!("DATABASE_URL must include a host and database name");
    }

    if app_env == "production" {
        if url.username().is_empty() || url.password().is_none() {
            bail!("production database URLs must include username and password");
        }
        let ssl_modes = url
            .query_pairs()
            .filter_map(|(key, value)| (key == "sslmode").then_some(value))
            .collect::<Vec<_>>();
        if ssl_modes.len() != 1 || ssl_modes[0] != "verify-full" {
            bail!("DATABASE_URL must set sslmode=verify-full in production");
        }
    }

    Ok(raw_url.to_owned())
}

fn validate_distinct_database_roles(application_url: &str, migration_url: &str) -> Result<()> {
    let application_url =
        Url::parse(application_url).context("DATABASE_URL must be a valid URL")?;
    let migration_url =
        Url::parse(migration_url).context("DATABASE_MIGRATION_URL must be a valid URL")?;

    if application_url.username() == migration_url.username() {
        bail!("DATABASE_URL and DATABASE_MIGRATION_URL must use different database roles");
    }

    Ok(())
}

fn validate_database_role(field: &str, role: &str) -> Result<()> {
    let mut bytes = role.bytes();
    let Some(first) = bytes.next() else {
        bail!("{field} must be a valid PostgreSQL role name");
    };
    if !(first.is_ascii_lowercase() || first == b'_')
        || role.len() > 63
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        bail!("{field} must use lowercase letters, digits, and underscores");
    }
    Ok(())
}

fn validate_web_origin(app_env: &str, raw_origin: &str) -> Result<HeaderValue> {
    let url = Url::parse(raw_origin).context("WEB_ORIGIN must be a valid URL")?;
    let is_https = url.scheme() == "https";
    if url.scheme() != "http" && !is_https {
        bail!("WEB_ORIGIN must use the http or https scheme");
    }
    if app_env == "production" && !is_https {
        bail!("WEB_ORIGIN must use https in production");
    }
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("WEB_ORIGIN must contain only scheme, host, and optional port");
    }

    url.origin()
        .ascii_serialization()
        .parse()
        .context("WEB_ORIGIN must be a valid HTTP header value")
}

fn validate_session_hmac_key(app_env: &str, raw_key: &str) -> Result<Vec<u8>> {
    if raw_key.len() < MINIMUM_HMAC_KEY_BYTES {
        bail!("SESSION_HMAC_KEY must contain at least 32 bytes");
    }
    let normalized_key = raw_key.to_ascii_lowercase();
    if app_env == "production"
        && INSECURE_SECRET_MARKERS
            .iter()
            .any(|marker| normalized_key.contains(marker))
    {
        bail!("SESSION_HMAC_KEY must be replaced with a random production secret");
    }
    Ok(raw_key.as_bytes().to_vec())
}

fn duration(name: &str) -> Result<Duration> {
    let seconds = required(name)?
        .parse::<u64>()
        .with_context(|| format!("{name} must be a positive number of seconds"))?;
    if seconds == 0 {
        bail!("{name} must be greater than zero");
    }
    Ok(Duration::from_secs(seconds))
}

fn required(name: &str) -> Result<String> {
    let value = env::var(name).with_context(|| format!("{name} is required"))?;
    if value.trim().is_empty() {
        bail!("{name} cannot be empty");
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        validate_database_role, validate_database_url, validate_distinct_database_roles,
        validate_port, validate_session_hmac_key, validate_web_origin,
    };

    #[test]
    fn api_port_must_be_a_nonzero_u16() {
        assert_eq!(validate_port("8080").expect("8080 is a valid port"), 8080);
        assert!(validate_port("0").is_err());
        assert!(validate_port("65536").is_err());
        assert!(validate_port("not-a-port").is_err());
    }

    #[test]
    fn production_requires_https_origin_without_extra_components() {
        assert!(validate_web_origin("production", "http://localhost:3000").is_err());
        assert!(validate_web_origin("production", "https://bycard.example/path").is_err());
        assert!(validate_web_origin("production", "https://bycard.example?debug=1").is_err());
        assert!(validate_web_origin("production", "https://user@bycard.example").is_err());
        assert_eq!(
            validate_web_origin("production", "https://bycard.example/")
                .expect("a valid production origin should be accepted"),
            "https://bycard.example"
        );
    }

    #[test]
    fn local_accepts_http_but_rejects_non_http_schemes() {
        assert!(validate_web_origin("local", "http://localhost:3000").is_ok());
        assert!(validate_web_origin("local", "javascript://unsafe").is_err());
    }

    #[test]
    fn production_database_requires_full_certificate_verification() {
        let base_url = "postgresql://user:password@db.example/bycard";
        assert!(validate_database_url("production", base_url).is_err());
        assert!(
            validate_database_url("production", &format!("{base_url}?sslmode=require")).is_err()
        );
        assert!(
            validate_database_url("production", &format!("{base_url}?sslmode=verify-full")).is_ok()
        );
        assert!(validate_database_url("local", base_url).is_ok());
    }

    #[test]
    fn production_database_requires_authenticated_distinct_roles() {
        assert!(
            validate_database_url(
                "production",
                "postgresql://db.example/bycard?sslmode=verify-full"
            )
            .is_err()
        );
        assert!(
            validate_distinct_database_roles(
                "postgresql://bycard_app:secret@db.example/bycard?sslmode=verify-full",
                "postgresql://bycard_app:other@db.example/bycard?sslmode=verify-full"
            )
            .is_err()
        );
        assert!(
            validate_distinct_database_roles(
                "postgresql://bycard_app:secret@db.example/bycard?sslmode=verify-full",
                "postgresql://bycard_owner:other@db.example/bycard?sslmode=verify-full"
            )
            .is_ok()
        );
    }

    #[test]
    fn database_roles_use_safe_portable_identifiers() {
        assert!(validate_database_role("role", "bycard_app").is_ok());
        assert!(validate_database_role("role", "owner2").is_ok());
        assert!(validate_database_role("role", "Bycard-App").is_err());
        assert!(validate_database_role("role", "1owner").is_err());
        assert!(validate_database_role("role", "role;drop_table").is_err());
    }

    #[test]
    fn production_rejects_default_or_short_session_secrets() {
        assert!(validate_session_hmac_key("production", "short").is_err());
        assert!(
            validate_session_hmac_key(
                "production",
                "local-only-change-this-key-before-any-deploy-32-bytes"
            )
            .is_err()
        );
        assert!(
            validate_session_hmac_key(
                "production",
                "YlYyMVBHMEJYeVF5d3Vycm9XcEx3T1h2OE9Td3dpTnJOTG90Q1B1eg"
            )
            .is_ok()
        );
    }
}
