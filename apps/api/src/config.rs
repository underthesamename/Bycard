use std::{env, net::IpAddr, time::Duration};

use anyhow::{Context, Result, bail};
use axum::http::HeaderValue;

const SUPPORTED_ENVIRONMENTS: [&str; 3] = ["local", "test", "production"];

#[derive(Clone)]
pub struct Config {
    pub app_env: String,
    pub bind_address: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub web_origin: HeaderValue,
    pub auth: bycard_api::app::AuthSettings,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let app_env = required("APP_ENV")?;
        if !SUPPORTED_ENVIRONMENTS.contains(&app_env.as_str()) {
            bail!("APP_ENV must be one of: local, test, production");
        }

        let bind_address = required("API_HOST")?
            .parse()
            .context("API_HOST must be a valid IP address")?;
        let port = required("API_PORT")?
            .parse()
            .context("API_PORT must be a valid port")?;
        let database_url = required("DATABASE_URL")?;
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            bail!("DATABASE_URL must use the postgres or postgresql scheme");
        }

        let web_origin = validate_web_origin(&app_env, &required("WEB_ORIGIN")?)?;
        let hmac_key = required("SESSION_HMAC_KEY")?.into_bytes();
        let session_ttl = duration("SESSION_TTL_SECONDS")?;
        let idle_ttl = duration("SESSION_IDLE_TTL_SECONDS")?;
        let touch_interval = duration("SESSION_TOUCH_INTERVAL_SECONDS")?;
        let csrf_ttl = duration("CSRF_TTL_SECONDS")?;
        let auth = bycard_api::app::AuthSettings {
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

fn validate_web_origin(app_env: &str, raw_origin: &str) -> Result<HeaderValue> {
    let is_https = raw_origin.starts_with("https://");
    if !raw_origin.starts_with("http://") && !is_https {
        bail!("WEB_ORIGIN must use the http or https scheme");
    }
    if app_env == "production" && !is_https {
        bail!("WEB_ORIGIN must use https in production");
    }
    raw_origin
        .parse()
        .context("WEB_ORIGIN must be a valid HTTP header value")
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
    use super::validate_web_origin;

    #[test]
    fn production_rejects_plain_http_origins() {
        assert!(validate_web_origin("production", "http://localhost:3000").is_err());
    }

    #[test]
    fn local_accepts_http_but_rejects_non_http_schemes() {
        assert!(validate_web_origin("local", "http://localhost:3000").is_ok());
        assert!(validate_web_origin("local", "javascript://unsafe").is_err());
    }
}
