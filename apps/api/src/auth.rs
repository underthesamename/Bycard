use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    Json, Router,
    extract::{Extension, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{Postgres, Transaction};
use subtle::ConstantTimeEq;
use tracing::error;
use uuid::Uuid;

use crate::{app::AppState, request_context::RequestId};

const PASSWORD_MIN_CHARS: usize = 15;
const PASSWORD_MAX_CHARS: usize = 128;
const DISPLAY_NAME_MAX_CHARS: usize = 60;
const SESSION_BYTES: usize = 32;
const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;

#[derive(Clone)]
pub struct AuthSettings {
    pub secure_cookie: bool,
    pub hmac_key: Vec<u8>,
    pub session_ttl: Duration,
    pub idle_ttl: Duration,
    pub touch_interval: Duration,
    pub csrf_ttl: Duration,
}

#[derive(Clone)]
pub struct AuthService {
    settings: AuthSettings,
    allowed_origin: HeaderValue,
    dummy_password_hash: Arc<String>,
    limiter: Arc<RateLimiter>,
}

impl AuthService {
    pub fn new(settings: AuthSettings, allowed_origin: HeaderValue) -> anyhow::Result<Self> {
        if settings.hmac_key.len() < 32 {
            anyhow::bail!("SESSION_HMAC_KEY must contain at least 32 bytes");
        }
        if settings.idle_ttl > settings.session_ttl
            || settings.touch_interval >= settings.idle_ttl
            || settings.csrf_ttl.is_zero()
        {
            anyhow::bail!("session and CSRF durations are inconsistent");
        }
        let dummy_password_hash = hash_password_sync("bycard-dummy-password-never-authenticates")?;
        Ok(Self {
            settings,
            allowed_origin,
            dummy_password_hash: Arc::new(dummy_password_hash),
            limiter: Arc::new(RateLimiter::default()),
        })
    }

    fn token_hash(&self, context: &[u8], token: &str) -> [u8; 32] {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.settings.hmac_key)
            .expect("HMAC accepts keys of any size");
        mac.update(context);
        mac.update(token.as_bytes());
        mac.finalize().into_bytes().into()
    }

    fn validate_origin(&self, headers: &HeaderMap) -> Result<(), AuthError> {
        let origin = headers.get(header::ORIGIN);
        if origin == Some(&self.allowed_origin) {
            Ok(())
        } else {
            Err(AuthError::forbidden("invalid_origin", "Origem inválida."))
        }
    }

    fn session_cookie(&self, token: &str) -> Result<HeaderValue, AuthError> {
        let name = self.cookie_name();
        let secure = if self.settings.secure_cookie {
            "; Secure"
        } else {
            ""
        };
        let value = format!(
            "{name}={token}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{secure}",
            self.settings.session_ttl.as_secs()
        );
        HeaderValue::from_str(&value).map_err(|_| AuthError::internal())
    }

    fn clear_cookie(&self) -> HeaderValue {
        let secure = if self.settings.secure_cookie {
            "; Secure"
        } else {
            ""
        };
        HeaderValue::from_str(&format!(
            "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{secure}",
            self.cookie_name()
        ))
        .expect("static cookie attributes are valid")
    }

    fn cookie_name(&self) -> &'static str {
        if self.settings.secure_cookie {
            "__Host-bycard_session"
        } else {
            "bycard_session"
        }
    }

    fn session_token<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
        let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
        cookies.split(';').find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            (name == self.cookie_name() && is_safe_token(value)).then_some(value)
        })
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/csrf", get(csrf))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RegisterRequest {
    display_name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    user: UserDto,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserDto {
    id: Uuid,
    display_name: String,
    email: String,
}

#[derive(sqlx::FromRow)]
pub(crate) struct SessionRow {
    session_id: Uuid,
    pub(crate) user_id: Uuid,
    display_name: String,
    email: String,
    expires_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
}

async fn register(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<Response, AuthError> {
    state.auth.validate_origin(&headers)?;
    let Json(request) = parse_json(body)?;
    let display_name = validate_display_name(&request.display_name)?;
    let email = normalize_email(&request.email)?;
    validate_password(&request.password)?;
    if !state
        .auth
        .limiter
        .allow("register", &email, 3, Duration::from_secs(3600))
    {
        return Err(AuthError::too_many_requests());
    }

    let password_hash = hash_password(request.password).await?;
    let session = new_session(&state.auth)?;
    let user_id = Uuid::now_v7();
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(log_database(&request_id))?;
    let insert_user = sqlx::query(
        "INSERT INTO users (id, display_name, email, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(&display_name)
    .bind(&email)
    .bind(password_hash)
    .execute(&mut *transaction)
    .await;
    if let Err(error) = insert_user {
        if error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref()
            == Some("23505")
        {
            return Err(AuthError::conflict(
                "email_already_registered",
                "Já existe uma conta com este e-mail.",
            ));
        }
        log_database(&request_id)(error);
        return Err(AuthError::internal());
    }
    insert_session(&mut transaction, user_id, &session, &state.auth)
        .await
        .map_err(log_database(&request_id))?;
    transaction
        .commit()
        .await
        .map_err(log_database(&request_id))?;

    session_response(
        StatusCode::CREATED,
        UserDto {
            id: user_id,
            display_name,
            email,
        },
        session.expires_at,
        state.auth.session_cookie(&session.token)?,
    )
}

async fn login(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Response, AuthError> {
    state.auth.validate_origin(&headers)?;
    let Json(request) = parse_json(body)?;
    if request.password.chars().count() > PASSWORD_MAX_CHARS {
        return Err(AuthError::invalid_credentials());
    }
    let email = normalize_email(&request.email).map_err(|_| AuthError::invalid_credentials())?;
    if !state
        .auth
        .limiter
        .allow("login", &email, 5, Duration::from_secs(15 * 60))
    {
        return Err(AuthError::too_many_requests());
    }
    let credentials = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT id, display_name, email, password_hash FROM users WHERE email = $1 AND deleted_at IS NULL",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    .map_err(log_database(&request_id))?;
    let stored_hash = credentials
        .as_ref()
        .map_or(state.auth.dummy_password_hash.as_str(), |value| {
            value.3.as_str()
        });
    let password_matches = verify_password(request.password, stored_hash.to_owned()).await?;
    let Some((user_id, display_name, email, _)) = credentials.filter(|_| password_matches) else {
        return Err(AuthError::invalid_credentials());
    };

    let session = new_session(&state.auth)?;
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(log_database(&request_id))?;
    if let Some(previous_token) = state.auth.session_token(&headers) {
        let previous_hash = state.auth.token_hash(b"session", previous_token);
        sqlx::query(
            "UPDATE sessions SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(previous_hash.as_slice())
        .execute(&mut *transaction)
        .await
        .map_err(log_database(&request_id))?;
    }
    insert_session(&mut transaction, user_id, &session, &state.auth)
        .await
        .map_err(log_database(&request_id))?;
    transaction
        .commit()
        .await
        .map_err(log_database(&request_id))?;
    session_response(
        StatusCode::OK,
        UserDto {
            id: user_id,
            display_name,
            email,
        },
        session.expires_at,
        state.auth.session_cookie(&session.token)?,
    )
}

async fn me(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AuthError> {
    let session = active_session(&state, &headers, &request_id).await?;
    let touch_before = Utc::now()
        - TimeDelta::from_std(state.auth.settings.touch_interval)
            .map_err(|_| AuthError::internal())?;
    if session.last_seen_at <= touch_before {
        sqlx::query(
            "UPDATE sessions SET last_seen_at = NOW() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(session.session_id)
        .execute(&state.pool)
        .await
        .map_err(log_database(&request_id))?;
    }
    Ok(Json(SessionResponse {
        user: UserDto {
            id: session.user_id,
            display_name: session.display_name,
            email: session.email,
        },
        expires_at: session.expires_at,
    }))
}

async fn csrf(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AuthError> {
    let session = active_session(&state, &headers, &request_id).await?;
    let token = generate_token()?;
    let hash = state.auth.token_hash(session.session_id.as_bytes(), &token);
    let ttl =
        TimeDelta::from_std(state.auth.settings.csrf_ttl).map_err(|_| AuthError::internal())?;
    sqlx::query("UPDATE sessions SET csrf_token_hash = $2, csrf_expires_at = NOW() + $3::interval WHERE id = $1 AND revoked_at IS NULL")
        .bind(session.session_id)
        .bind(hash.as_slice())
        .bind(format!("{} seconds", ttl.num_seconds()))
        .execute(&state.pool)
        .await
        .map_err(log_database(&request_id))?;
    Ok(Json(serde_json::json!({ "csrfToken": token })))
}

async fn logout(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Response, AuthError> {
    state.auth.validate_origin(&headers)?;
    let session_token = state
        .auth
        .session_token(&headers)
        .ok_or_else(AuthError::unauthorized)?;
    let csrf_token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_safe_token(value))
        .ok_or_else(|| AuthError::forbidden("csrf_rejected", "Proteção CSRF inválida."))?;
    let session_hash = state.auth.token_hash(b"session", session_token);
    let session_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM sessions WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > NOW() AND last_seen_at > NOW() - $2::interval",
    )
    .bind(session_hash.as_slice())
    .bind(duration_interval(state.auth.settings.idle_ttl))
    .fetch_optional(&state.pool)
    .await
    .map_err(log_database(&request_id))?;
    let session_id = session_id.ok_or_else(AuthError::unauthorized)?;
    let supplied_hash = state.auth.token_hash(session_id.as_bytes(), csrf_token);
    let stored_hash: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT csrf_token_hash FROM sessions WHERE id = $1 AND csrf_expires_at > NOW()",
    )
    .bind(session_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(log_database(&request_id))?
    .flatten();
    let csrf_matches = stored_hash
        .as_deref()
        .is_some_and(|stored| stored.ct_eq(supplied_hash.as_slice()).into());
    if !csrf_matches {
        return Err(AuthError::forbidden(
            "csrf_rejected",
            "Proteção CSRF inválida.",
        ));
    }
    sqlx::query("UPDATE sessions SET revoked_at = NOW(), csrf_token_hash = NULL, csrf_expires_at = NULL WHERE id = $1")
        .bind(session_id)
        .execute(&state.pool)
        .await
        .map_err(log_database(&request_id))?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, state.auth.clear_cookie());
    Ok(response)
}

pub(crate) async fn active_session(
    state: &AppState,
    headers: &HeaderMap,
    request_id: &RequestId,
) -> Result<SessionRow, AuthError> {
    let token = state
        .auth
        .session_token(headers)
        .ok_or_else(AuthError::unauthorized)?;
    let token_hash = state.auth.token_hash(b"session", token);
    sqlx::query_as::<_, SessionRow>(
        "SELECT s.id AS session_id, u.id AS user_id, u.display_name, u.email, s.expires_at, s.last_seen_at FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > NOW() AND s.last_seen_at > NOW() - $2::interval AND u.deleted_at IS NULL",
    )
    .bind(token_hash.as_slice())
    .bind(duration_interval(state.auth.settings.idle_ttl))
    .fetch_optional(&state.pool)
    .await
    .map_err(log_database(request_id))?
    .ok_or_else(AuthError::unauthorized)
}

pub(crate) async fn authorize_mutation(
    state: &AppState,
    headers: &HeaderMap,
    request_id: &RequestId,
) -> Result<SessionRow, AuthError> {
    state.auth.validate_origin(headers)?;
    let session = active_session(state, headers, request_id).await?;
    let csrf_token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_safe_token(value))
        .ok_or_else(|| AuthError::forbidden("csrf_rejected", "Proteção CSRF inválida."))?;
    let supplied_hash = state
        .auth
        .token_hash(session.session_id.as_bytes(), csrf_token);
    let stored_hash: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT csrf_token_hash FROM sessions WHERE id = $1 AND csrf_expires_at > NOW()",
    )
    .bind(session.session_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(log_database(request_id))?
    .flatten();
    let csrf_matches = stored_hash
        .as_deref()
        .is_some_and(|stored| stored.ct_eq(supplied_hash.as_slice()).into());
    if !csrf_matches {
        return Err(AuthError::forbidden(
            "csrf_rejected",
            "Proteção CSRF inválida.",
        ));
    }
    Ok(session)
}

struct NewSession {
    id: Uuid,
    token: String,
    expires_at: DateTime<Utc>,
}

fn new_session(auth: &AuthService) -> Result<NewSession, AuthError> {
    let ttl = TimeDelta::from_std(auth.settings.session_ttl).map_err(|_| AuthError::internal())?;
    Ok(NewSession {
        id: Uuid::now_v7(),
        token: generate_token()?,
        expires_at: Utc::now() + ttl,
    })
}

async fn insert_session(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    session: &NewSession,
    auth: &AuthService,
) -> Result<(), sqlx::Error> {
    let token_hash = auth.token_hash(b"session", &session.token);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(session.id)
    .bind(user_id)
    .bind(token_hash.as_slice())
    .bind(session.expires_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn session_response(
    status: StatusCode,
    user: UserDto,
    expires_at: DateTime<Utc>,
    cookie: HeaderValue,
) -> Result<Response, AuthError> {
    let mut response = (status, Json(SessionResponse { user, expires_at })).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

fn normalize_email(raw: &str) -> Result<String, AuthError> {
    let email = raw.trim().to_ascii_lowercase();
    let valid = email.len() <= 254
        && email.is_ascii()
        && !email.contains(char::is_whitespace)
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && local.len() <= 64
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        });
    if valid {
        Ok(email)
    } else {
        Err(AuthError::bad_request(
            "invalid_email",
            "Informe um e-mail válido.",
        ))
    }
}

fn validate_display_name(raw: &str) -> Result<String, AuthError> {
    let display_name = raw.trim();
    let length = display_name.chars().count();
    if (2..=DISPLAY_NAME_MAX_CHARS).contains(&length) && !display_name.chars().any(char::is_control)
    {
        Ok(display_name.to_owned())
    } else {
        Err(AuthError::bad_request(
            "invalid_display_name",
            "O nome deve ter entre 2 e 60 caracteres.",
        ))
    }
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    let length = password.chars().count();
    if (PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&length) {
        Ok(())
    } else {
        Err(AuthError::bad_request(
            "invalid_password",
            "A senha deve ter entre 15 e 128 caracteres.",
        ))
    }
}

fn argon2() -> Result<Argon2<'static>, AuthError> {
    let parameters = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|_| AuthError::internal())?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters))
}

fn hash_password_sync(password: &str) -> anyhow::Result<String> {
    let parameters = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let algorithm = Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters);
    let salt = random_salt().map_err(|error| anyhow::anyhow!(error.message))?;
    Ok(algorithm
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .to_string())
}

async fn hash_password(password: String) -> Result<String, AuthError> {
    tokio::task::spawn_blocking(move || {
        let salt = random_salt()?;
        argon2()?
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AuthError::internal())
    })
    .await
    .map_err(|_| AuthError::internal())?
}

fn random_salt() -> Result<SaltString, AuthError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| AuthError::internal())?;
    SaltString::encode_b64(&bytes).map_err(|_| AuthError::internal())
}

async fn verify_password(password: String, encoded_hash: String) -> Result<bool, AuthError> {
    tokio::task::spawn_blocking(move || {
        let hash = PasswordHash::new(&encoded_hash).map_err(|_| AuthError::internal())?;
        Ok(argon2()?
            .verify_password(password.as_bytes(), &hash)
            .is_ok())
    })
    .await
    .map_err(|_| AuthError::internal())?
}

fn generate_token() -> Result<String, AuthError> {
    let mut bytes = [0_u8; SESSION_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| AuthError::internal())?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn is_safe_token(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn duration_interval(duration: Duration) -> String {
    format!("{} seconds", duration.as_secs())
}

fn parse_json<T>(body: Result<Json<T>, JsonRejection>) -> Result<Json<T>, AuthError> {
    body.map_err(|_| AuthError::bad_request("invalid_json", "Os dados enviados são inválidos."))
}

fn log_database(request_id: &RequestId) -> impl Fn(sqlx::Error) -> AuthError + '_ {
    move |database_error| {
        error!(request_id = %request_id.0, error = %database_error, "authentication database operation failed");
        AuthError::internal()
    }
}

#[derive(Default)]
struct RateLimiter {
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    fn allow(&self, operation: &str, key: &str, limit: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let entries = attempts.entry(format!("{operation}:{key}")).or_default();
        entries.retain(|attempt| now.duration_since(*attempt) < window);
        if entries.len() >= limit {
            return false;
        }
        entries.push(now);
        true
    }
}

pub(crate) struct AuthError {
    pub(crate) status: StatusCode,
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

impl AuthError {
    fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message,
        }
    }
    fn conflict(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code,
            message,
        }
    }
    fn forbidden(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code,
            message,
        }
    }
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "authentication_required",
            message: "Sua sessão não é válida.",
        }
    }
    fn invalid_credentials() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "invalid_credentials",
            message: "E-mail ou senha inválidos.",
        }
    }
    fn too_many_requests() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limit_exceeded",
            message: "Muitas tentativas. Aguarde antes de tentar novamente.",
        }
    }
    fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "Não foi possível concluir a autenticação agora.",
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": { "code": self.code, "message": self.message } })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_uses_configured_argon2id_parameters() {
        let hash = hash_password_sync("uma-senha-de-teste-bem-longa").expect("hash should work");
        assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    }

    #[test]
    fn validates_password_boundaries() {
        assert!(validate_password("123456789012345").is_ok());
        assert!(validate_password("curta").is_err());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }
}
