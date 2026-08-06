use axum::{
    Json, Router,
    extract::{Extension, Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use tracing::error;
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::{active_session, authorize_mutation},
    request_context::RequestId,
};

const MAX_QUANTITY: i32 = 999;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/me/collections",
            get(list_collections).post(add_collection),
        )
        .route(
            "/me/collections/{set_id}",
            get(get_collection).delete(remove_collection),
        )
        .route(
            "/me/collections/{set_id}/cards/{card_id}",
            put(set_quantity),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AddCollectionRequest {
    set_id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetQuantityRequest {
    quantity: i32,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct PersonalCollection {
    id: Uuid,
    set_id: Uuid,
    slug: String,
    name: String,
    cover_image_url: Option<String>,
    total_unique: i64,
    owned_unique: i64,
    missing_unique: i64,
    total_copies: i64,
    duplicate_copies: i64,
    completion_percentage: f64,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct PersonalCard {
    id: Uuid,
    set_id: Uuid,
    local_number: String,
    printed_number: String,
    name: String,
    rarity: Option<String>,
    artist: Option<String>,
    image_small_url: Option<String>,
    image_large_url: Option<String>,
    sort_order: i32,
    quantity: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionDetail {
    collection: PersonalCollection,
    cards: Vec<PersonalCard>,
}

#[derive(Serialize)]
struct DataResponse<T> {
    data: T,
}

async fn list_collections(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Json<DataResponse<Vec<PersonalCollection>>>, CollectionError> {
    let session = active_session(&state, &headers, &request_id)
        .await
        .map_err(|error| CollectionError::auth(error, &request_id))?;
    let collections = query_collections(&state, session.user_id, None, &request_id).await?;
    Ok(Json(DataResponse { data: collections }))
}

async fn add_collection(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Result<Json<AddCollectionRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DataResponse<PersonalCollection>>), CollectionError> {
    let session = authorize_mutation(&state, &headers, &request_id)
        .await
        .map_err(|error| CollectionError::auth(error, &request_id))?;
    let Json(request) = body.map_err(|_| CollectionError::invalid_json(&request_id))?;
    let collection_id = Uuid::now_v7();
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO user_collections (id, user_id, set_id) SELECT $1, $2, s.id FROM sets s WHERE s.id = $3 AND s.is_published = TRUE ON CONFLICT (user_id, set_id) DO NOTHING RETURNING id",
    )
    .bind(collection_id)
    .bind(session.user_id)
    .bind(request.set_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| CollectionError::database(error, &request_id))?;
    let collection = query_collections(&state, session.user_id, Some(request.set_id), &request_id)
        .await?
        .pop()
        .ok_or_else(|| {
            CollectionError::not_found(
                "collection_not_found",
                "Coleção não encontrada.",
                &request_id,
            )
        })?;
    Ok((
        if inserted.is_some() {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(DataResponse { data: collection }),
    ))
}

async fn get_collection(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Path(set_id): Path<Uuid>,
) -> Result<Json<DataResponse<CollectionDetail>>, CollectionError> {
    let session = active_session(&state, &headers, &request_id)
        .await
        .map_err(|error| CollectionError::auth(error, &request_id))?;
    let detail = load_collection_detail(&state, session.user_id, set_id, &request_id).await?;
    Ok(Json(DataResponse { data: detail }))
}

async fn load_collection_detail(
    state: &AppState,
    user_id: Uuid,
    set_id: Uuid,
    request_id: &RequestId,
) -> Result<CollectionDetail, CollectionError> {
    let collection = query_collections(state, user_id, Some(set_id), request_id)
        .await?
        .pop()
        .ok_or_else(|| {
            CollectionError::not_found(
                "personal_collection_not_found",
                "Coleção pessoal não encontrada.",
                request_id,
            )
        })?;
    let cards = sqlx::query_as::<_, PersonalCard>(
        "SELECT c.id, c.set_id, c.local_number, c.printed_number, c.name, c.rarity, c.artist, c.image_small_url, c.image_large_url, c.sort_order, COALESCE(h.quantity, 0)::int AS quantity FROM cards c LEFT JOIN user_card_holdings h ON h.card_id = c.id AND h.user_id = $1 WHERE c.set_id = $2 AND c.is_published = TRUE ORDER BY c.sort_order ASC",
    )
    .bind(user_id)
    .bind(set_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| CollectionError::database(error, request_id))?;
    Ok(CollectionDetail { collection, cards })
}

async fn remove_collection(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Path(set_id): Path<Uuid>,
) -> Result<StatusCode, CollectionError> {
    let session = authorize_mutation(&state, &headers, &request_id)
        .await
        .map_err(|error| CollectionError::auth(error, &request_id))?;
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|error| CollectionError::database(error, &request_id))?;
    sqlx::query("DELETE FROM user_card_holdings h USING cards c, user_collections uc WHERE h.card_id = c.id AND c.set_id = $1 AND h.user_id = $2 AND uc.user_id = $2 AND uc.set_id = $1")
        .bind(set_id).bind(session.user_id).execute(&mut *transaction).await
        .map_err(|error| CollectionError::database(error, &request_id))?;
    let deleted = sqlx::query("DELETE FROM user_collections WHERE user_id = $1 AND set_id = $2")
        .bind(session.user_id)
        .bind(set_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| CollectionError::database(error, &request_id))?;
    if deleted.rows_affected() == 0 {
        return Err(CollectionError::not_found(
            "personal_collection_not_found",
            "Coleção pessoal não encontrada.",
            &request_id,
        ));
    }
    transaction
        .commit()
        .await
        .map_err(|error| CollectionError::database(error, &request_id))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_quantity(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Path((set_id, card_id)): Path<(Uuid, Uuid)>,
    body: Result<Json<SetQuantityRequest>, JsonRejection>,
) -> Result<Json<DataResponse<CollectionDetail>>, CollectionError> {
    let session = authorize_mutation(&state, &headers, &request_id)
        .await
        .map_err(|error| CollectionError::auth(error, &request_id))?;
    let Json(request) = body.map_err(|_| CollectionError::invalid_json(&request_id))?;
    if !(0..=MAX_QUANTITY).contains(&request.quantity) {
        return Err(CollectionError::bad_request(
            "invalid_quantity",
            "A quantidade deve estar entre 0 e 999.",
            &request_id,
        ));
    }
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|error| CollectionError::database(error, &request_id))?;
    lock_personal_card(
        &mut transaction,
        session.user_id,
        set_id,
        card_id,
        &request_id,
    )
    .await?;
    if request.quantity == 0 {
        sqlx::query("DELETE FROM user_card_holdings WHERE user_id = $1 AND card_id = $2")
            .bind(session.user_id)
            .bind(card_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CollectionError::database(error, &request_id))?;
    } else {
        sqlx::query("INSERT INTO user_card_holdings (id, user_id, card_id, quantity) VALUES ($1, $2, $3, $4) ON CONFLICT (user_id, card_id) DO UPDATE SET quantity = EXCLUDED.quantity, updated_at = NOW()")
            .bind(Uuid::now_v7()).bind(session.user_id).bind(card_id).bind(request.quantity)
            .execute(&mut *transaction).await.map_err(|error| CollectionError::database(error, &request_id))?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| CollectionError::database(error, &request_id))?;
    let detail = load_collection_detail(&state, session.user_id, set_id, &request_id).await?;
    Ok(Json(DataResponse { data: detail }))
}

async fn lock_personal_card(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    set_id: Uuid,
    card_id: Uuid,
    request_id: &RequestId,
) -> Result<(), CollectionError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_collections uc JOIN sets s ON s.id = uc.set_id JOIN cards c ON c.set_id = s.id WHERE uc.user_id = $1 AND uc.set_id = $2 AND c.id = $3 AND s.is_published = TRUE AND c.is_published = TRUE FOR UPDATE OF uc)",
    ).bind(user_id).bind(set_id).bind(card_id).fetch_one(&mut **transaction).await
      .map_err(|error| CollectionError::database(error, request_id))?;
    if !exists {
        return Err(CollectionError::not_found(
            "personal_card_not_found",
            "Carta ou coleção pessoal não encontrada.",
            request_id,
        ));
    }
    Ok(())
}

async fn query_collections(
    state: &AppState,
    user_id: Uuid,
    set_id: Option<Uuid>,
    request_id: &RequestId,
) -> Result<Vec<PersonalCollection>, CollectionError> {
    let mut collections = sqlx::query_as::<_, PersonalCollection>(
        "SELECT uc.id, s.id AS set_id, s.slug, s.name, s.cover_image_url, COUNT(c.id)::bigint AS total_unique, COUNT(h.card_id)::bigint AS owned_unique, (COUNT(c.id) - COUNT(h.card_id))::bigint AS missing_unique, COALESCE(SUM(h.quantity), 0)::bigint AS total_copies, COALESCE(SUM(GREATEST(h.quantity - 1, 0)), 0)::bigint AS duplicate_copies, 0::float8 AS completion_percentage FROM user_collections uc JOIN sets s ON s.id = uc.set_id LEFT JOIN cards c ON c.set_id = s.id AND c.is_published = TRUE LEFT JOIN user_card_holdings h ON h.user_id = uc.user_id AND h.card_id = c.id WHERE uc.user_id = $1 AND s.is_published = TRUE AND ($2::uuid IS NULL OR s.id = $2) GROUP BY uc.id, s.id ORDER BY uc.created_at DESC",
    ).bind(user_id).bind(set_id).fetch_all(&state.pool).await
      .map_err(|error| CollectionError::database(error, request_id))?;
    for collection in &mut collections {
        collection.completion_percentage =
            completion_percentage(collection.total_unique, collection.owned_unique);
    }
    Ok(collections)
}

fn completion_percentage(total_unique: i64, owned_unique: i64) -> f64 {
    if total_unique == 0 {
        0.0
    } else {
        owned_unique as f64 * 100.0 / total_unique as f64
    }
}

struct CollectionError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    request_id: String,
}

impl CollectionError {
    fn bad_request(code: &'static str, message: &'static str, request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message,
            request_id: request_id.0.clone(),
        }
    }
    fn invalid_json(request_id: &RequestId) -> Self {
        Self::bad_request(
            "invalid_json",
            "Os dados enviados são inválidos.",
            request_id,
        )
    }
    fn not_found(code: &'static str, message: &'static str, request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code,
            message,
            request_id: request_id.0.clone(),
        }
    }
    fn database(database_error: sqlx::Error, request_id: &RequestId) -> Self {
        error!(request_id = %request_id.0, error = %database_error, "personal collection database operation failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "Não foi possível concluir a operação agora.",
            request_id: request_id.0.clone(),
        }
    }
    fn auth(error: crate::auth::AuthError, request_id: &RequestId) -> Self {
        Self {
            status: error.status,
            code: error.code,
            message: error.message,
            request_id: request_id.0.clone(),
        }
    }
}

impl IntoResponse for CollectionError {
    fn into_response(self) -> Response {
        (self.status, Json(serde_json::json!({ "error": { "code": self.code, "message": self.message, "requestId": self.request_id } }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::completion_percentage;

    #[test]
    fn completion_is_zero_for_an_empty_collection() {
        assert_eq!(completion_percentage(0, 0), 0.0);
    }

    #[test]
    fn completion_keeps_precision_for_presentation_to_round() {
        assert!((completion_percentage(18, 1) - 5.555_555_555_555_555).abs() < f64::EPSILON);
        assert_eq!(completion_percentage(18, 18), 100.0);
    }
}
