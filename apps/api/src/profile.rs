use std::io::Cursor;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Extension, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, put},
};
use image::{ImageFormat, ImageReader, Limits, Rgb, RgbImage, codecs::jpeg::JpegEncoder};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::{AuthError, active_session, authorize_mutation, validate_display_name},
    request_context::RequestId,
};

pub(crate) const MAX_AVATAR_UPLOAD_BYTES: usize = 2 * 1024 * 1024;
const MAX_AVATAR_DIMENSION: u32 = 2_048;
const MAX_AVATAR_DECODE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_STORED_AVATAR_BYTES: usize = 128 * 1024;
const AVATAR_DIMENSION: u32 = 256;
const AVATAR_JPEG_QUALITY: u8 = 82;
const AVATAR_CONTENT_TYPE: &str = "image/jpeg";

pub fn router() -> Router<AppState> {
    Router::new().route("/me/profile", put(update_profile))
}

pub fn avatar_router() -> Router<AppState> {
    Router::new().route(
        "/me/avatar",
        get(get_avatar).put(upload_avatar).delete(delete_avatar),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateProfileRequest {
    display_name: String,
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ProfileUser {
    id: Uuid,
    display_name: String,
    username: String,
    email: String,
    avatar_version: Option<Uuid>,
}

#[derive(Serialize)]
struct ProfileResponse {
    user: ProfileUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarResponse {
    avatar_version: Uuid,
}

async fn update_profile(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Result<Json<UpdateProfileRequest>, JsonRejection>,
) -> Result<Json<ProfileResponse>, ProfileError> {
    let session = authorize_mutation(&state, &headers, &request_id)
        .await
        .map_err(|error| ProfileError::auth(error, &request_id))?;
    let Json(request) = body.map_err(|_| ProfileError::invalid_json(&request_id))?;
    let display_name = validate_display_name(&request.display_name)
        .map_err(|error| ProfileError::auth(error, &request_id))?;
    let user = sqlx::query_as::<_, ProfileUser>(
        "UPDATE users SET display_name = $2, updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL RETURNING id, display_name, username, email, avatar_version",
    )
    .bind(session.user_id)
    .bind(display_name)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| ProfileError::database(error, &request_id))?
    .ok_or_else(|| ProfileError::not_found(&request_id))?;

    Ok(Json(ProfileResponse { user }))
}

async fn upload_avatar(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AvatarResponse>, ProfileError> {
    let session = authorize_mutation(&state, &headers, &request_id)
        .await
        .map_err(|error| ProfileError::auth(error, &request_id))?;
    let declared_format = declared_image_format(&headers, &request_id)?;
    let raw_avatar = body.to_vec();
    let normalized_avatar = tokio::task::spawn_blocking(move || {
        normalize_avatar(&raw_avatar, declared_format)
    })
    .await
    .map_err(|join_error| {
        error!(request_id = %request_id.0, error = %join_error, "avatar processing task failed");
        ProfileError::internal(&request_id)
    })?
    .map_err(|processing_error| {
        warn!(request_id = %request_id.0, error = ?processing_error, "avatar upload was rejected");
        ProfileError::invalid_avatar(&request_id)
    })?;
    let avatar_version = Uuid::now_v7();
    let updated = sqlx::query(
        "UPDATE users SET avatar_data = $2, avatar_content_type = $3, avatar_version = $4, updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(session.user_id)
    .bind(normalized_avatar)
    .bind(AVATAR_CONTENT_TYPE)
    .bind(avatar_version)
    .execute(&state.pool)
    .await
    .map_err(|error| ProfileError::database(error, &request_id))?;
    if updated.rows_affected() == 0 {
        return Err(ProfileError::not_found(&request_id));
    }

    Ok(Json(AvatarResponse { avatar_version }))
}

async fn get_avatar(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Response, ProfileError> {
    let session = active_session(&state, &headers, &request_id)
        .await
        .map_err(|error| ProfileError::auth(error, &request_id))?;
    let stored_avatar = sqlx::query_as::<_, (Option<Vec<u8>>, Option<String>, Option<Uuid>)>(
        "SELECT avatar_data, avatar_content_type, avatar_version FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(session.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| ProfileError::database(error, &request_id))?
    .ok_or_else(|| ProfileError::not_found(&request_id))?;
    let (Some(data), Some(content_type), Some(version)) = stored_avatar else {
        return Err(ProfileError::avatar_not_found(&request_id));
    };
    let etag = format!("\"{version}\"");
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        == Some(etag.as_str())
    {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        apply_avatar_headers(&mut response, &etag, None, &request_id)?;
        return Ok(response);
    }

    let mut response = data.into_response();
    apply_avatar_headers(&mut response, &etag, Some(&content_type), &request_id)?;
    Ok(response)
}

async fn delete_avatar(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<StatusCode, ProfileError> {
    let session = authorize_mutation(&state, &headers, &request_id)
        .await
        .map_err(|error| ProfileError::auth(error, &request_id))?;
    let updated = sqlx::query(
        "UPDATE users SET avatar_data = NULL, avatar_content_type = NULL, avatar_version = NULL, updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(session.user_id)
    .execute(&state.pool)
    .await
    .map_err(|error| ProfileError::database(error, &request_id))?;
    if updated.rows_affected() == 0 {
        return Err(ProfileError::not_found(&request_id));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn declared_image_format(
    headers: &HeaderMap,
    request_id: &RequestId,
) -> Result<ImageFormat, ProfileError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    match content_type {
        Some("image/jpeg") => Ok(ImageFormat::Jpeg),
        Some("image/png") => Ok(ImageFormat::Png),
        Some("image/webp") => Ok(ImageFormat::WebP),
        _ => Err(ProfileError::invalid_avatar(request_id)),
    }
}

fn normalize_avatar(
    raw_avatar: &[u8],
    declared_format: ImageFormat,
) -> Result<Vec<u8>, AvatarProcessingError> {
    if raw_avatar.is_empty() || raw_avatar.len() > MAX_AVATAR_UPLOAD_BYTES {
        return Err(AvatarProcessingError::InvalidInput);
    }
    let mut reader = ImageReader::new(Cursor::new(raw_avatar))
        .with_guessed_format()
        .map_err(|_| AvatarProcessingError::InvalidInput)?;
    if reader.format() != Some(declared_format) {
        return Err(AvatarProcessingError::InvalidInput);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_AVATAR_DIMENSION);
    limits.max_image_height = Some(MAX_AVATAR_DIMENSION);
    limits.max_alloc = Some(MAX_AVATAR_DECODE_BYTES);
    reader.limits(limits);
    let resized = reader
        .decode()
        .map_err(|_| AvatarProcessingError::InvalidInput)?
        .resize_to_fill(
            AVATAR_DIMENSION,
            AVATAR_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
        .to_rgba8();
    let mut rgb = RgbImage::from_pixel(AVATAR_DIMENSION, AVATAR_DIMENSION, Rgb([246, 247, 251]));
    for (target, source) in rgb.pixels_mut().zip(resized.pixels()) {
        let alpha = u16::from(source[3]);
        let inverse_alpha = 255 - alpha;
        for channel in 0..3 {
            target[channel] = ((u16::from(source[channel]) * alpha
                + u16::from(target[channel]) * inverse_alpha
                + 127)
                / 255) as u8;
        }
    }
    let mut encoded = Vec::new();
    JpegEncoder::new_with_quality(&mut encoded, AVATAR_JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|_| AvatarProcessingError::EncodingFailed)?;
    if encoded.len() > MAX_STORED_AVATAR_BYTES {
        return Err(AvatarProcessingError::EncodedImageTooLarge);
    }
    Ok(encoded)
}

fn apply_avatar_headers(
    response: &mut Response,
    etag: &str,
    content_type: Option<&str>,
    request_id: &RequestId,
) -> Result<(), ProfileError> {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=300, no-transform"),
    );
    response.headers_mut().insert(
        header::ETAG,
        HeaderValue::from_str(etag).map_err(|_| ProfileError::internal(request_id))?,
    );
    if let Some(content_type) = content_type {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(content_type).map_err(|_| ProfileError::internal(request_id))?,
        );
    }
    Ok(())
}

#[derive(Debug)]
enum AvatarProcessingError {
    InvalidInput,
    EncodingFailed,
    EncodedImageTooLarge,
}

struct ProfileError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    request_id: String,
}

impl ProfileError {
    fn invalid_json(request_id: &RequestId) -> Self {
        Self::bad_request(
            "invalid_json",
            "Os dados enviados são inválidos.",
            request_id,
        )
    }

    fn invalid_avatar(request_id: &RequestId) -> Self {
        Self::bad_request(
            "invalid_avatar",
            "Envie uma imagem JPEG, PNG ou WebP válida de até 2 MB.",
            request_id,
        )
    }

    fn bad_request(code: &'static str, message: &'static str, request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message,
            request_id: request_id.0.clone(),
        }
    }

    fn avatar_not_found(request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "avatar_not_found",
            message: "Este perfil ainda não possui foto.",
            request_id: request_id.0.clone(),
        }
    }

    fn not_found(request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "profile_not_found",
            message: "Perfil não encontrado.",
            request_id: request_id.0.clone(),
        }
    }

    fn database(database_error: sqlx::Error, request_id: &RequestId) -> Self {
        error!(request_id = %request_id.0, error = %database_error, "profile database operation failed");
        Self::internal(request_id)
    }

    fn auth(error: AuthError, request_id: &RequestId) -> Self {
        Self {
            status: error.status,
            code: error.code,
            message: error.message,
            request_id: request_id.0.clone(),
        }
    }

    fn internal(request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "Não foi possível atualizar o perfil agora.",
            request_id: request_id.0.clone(),
        }
    }
}

impl IntoResponse for ProfileError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": {
                    "code": self.code,
                    "message": self.message,
                    "requestId": self.request_id,
                }
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat};

    use super::{AVATAR_DIMENSION, MAX_STORED_AVATAR_BYTES, normalize_avatar};

    #[test]
    fn avatar_is_resized_and_reencoded_without_source_metadata() {
        let source = DynamicImage::new_rgba8(640, 320);
        let mut bytes = Cursor::new(Vec::new());
        source
            .write_to(&mut bytes, ImageFormat::Png)
            .expect("test PNG should encode");

        let normalized = normalize_avatar(bytes.get_ref(), ImageFormat::Png)
            .expect("valid PNG should normalize");
        let normalized_image = image::load_from_memory_with_format(&normalized, ImageFormat::Jpeg)
            .expect("normalized avatar should be a JPEG");

        assert_eq!(normalized_image.width(), AVATAR_DIMENSION);
        assert_eq!(normalized_image.height(), AVATAR_DIMENSION);
        assert!(normalized.len() <= MAX_STORED_AVATAR_BYTES);
    }

    #[test]
    fn avatar_rejects_a_mismatched_declared_format() {
        let source = DynamicImage::new_rgb8(8, 8);
        let mut bytes = Cursor::new(Vec::new());
        source
            .write_to(&mut bytes, ImageFormat::Png)
            .expect("test PNG should encode");

        assert!(normalize_avatar(bytes.get_ref(), ImageFormat::Jpeg).is_err());
    }
}
