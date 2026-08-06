use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::error;
use uuid::Uuid;

use crate::{app::AppState, request_context::RequestId};

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 100;
const MAX_SEARCH_CHARACTERS: usize = 160;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sets", get(list_sets))
        .route("/sets/{set_id}", get(get_set))
        .route("/sets/{set_id}/cards", get(list_cards))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawCatalogQuery {
    search: Option<String>,
    page: Option<String>,
    page_size: Option<String>,
    sort: Option<String>,
}

#[derive(Clone, Copy)]
struct Pagination {
    page: u32,
    page_size: u32,
}

impl Pagination {
    fn parse(query: &RawCatalogQuery) -> Result<Self, ValidationError> {
        let page = parse_positive_parameter("page", query.page.as_deref(), DEFAULT_PAGE)?;
        let page_size =
            parse_positive_parameter("pageSize", query.page_size.as_deref(), DEFAULT_PAGE_SIZE)?;
        if page_size > MAX_PAGE_SIZE {
            return Err(ValidationError::new(format!(
                "pageSize deve ser no máximo {MAX_PAGE_SIZE}."
            )));
        }
        Ok(Self { page, page_size })
    }

    fn offset(self) -> i64 {
        i64::from(self.page - 1) * i64::from(self.page_size)
    }
}

fn parse_positive_parameter(
    name: &str,
    raw_value: Option<&str>,
    default: u32,
) -> Result<u32, ValidationError> {
    let Some(raw_value) = raw_value else {
        return Ok(default);
    };
    let value = raw_value.parse::<u32>().map_err(|_| {
        ValidationError::new(format!("{name} deve ser um número inteiro positivo."))
    })?;
    if value == 0 {
        return Err(ValidationError::new(format!(
            "{name} deve ser maior que zero."
        )));
    }
    Ok(value)
}

#[derive(Clone, Copy)]
enum SetSort {
    ReleaseDateDesc,
    ReleaseDateAsc,
    NameAsc,
    NameDesc,
}

impl SetSort {
    fn parse(raw_value: Option<&str>) -> Result<Self, ValidationError> {
        match raw_value.unwrap_or("release_date_desc") {
            "release_date_desc" => Ok(Self::ReleaseDateDesc),
            "release_date_asc" => Ok(Self::ReleaseDateAsc),
            "name_asc" => Ok(Self::NameAsc),
            "name_desc" => Ok(Self::NameDesc),
            _ => Err(ValidationError::new(
                "sort inválido para coleções. Use release_date_desc, release_date_asc, name_asc ou name_desc.",
            )),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::ReleaseDateDesc => "release_date_desc",
            Self::ReleaseDateAsc => "release_date_asc",
            Self::NameAsc => "name_asc",
            Self::NameDesc => "name_desc",
        }
    }
}

#[derive(Clone, Copy)]
enum CardSort {
    NumberAsc,
    NumberDesc,
    NameAsc,
    NameDesc,
}

impl CardSort {
    fn parse(raw_value: Option<&str>) -> Result<Self, ValidationError> {
        match raw_value.unwrap_or("number_asc") {
            "number_asc" => Ok(Self::NumberAsc),
            "number_desc" => Ok(Self::NumberDesc),
            "name_asc" => Ok(Self::NameAsc),
            "name_desc" => Ok(Self::NameDesc),
            _ => Err(ValidationError::new(
                "sort inválido para cartas. Use number_asc, number_desc, name_asc ou name_desc.",
            )),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::NumberAsc => "number_asc",
            Self::NumberDesc => "number_desc",
            Self::NameAsc => "name_asc",
            Self::NameDesc => "name_desc",
        }
    }
}

struct SearchTerm(Option<String>);

impl SearchTerm {
    fn parse(raw_value: Option<&str>) -> Result<Self, ValidationError> {
        let Some(raw_value) = raw_value else {
            return Ok(Self(None));
        };
        if raw_value.chars().any(char::is_control) {
            return Err(ValidationError::new(
                "search contém caracteres não permitidos.",
            ));
        }
        let normalized = raw_value.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.is_empty() {
            return Ok(Self(None));
        }
        if normalized.chars().count() > MAX_SEARCH_CHARACTERS {
            return Err(ValidationError::new(format!(
                "search deve ter no máximo {MAX_SEARCH_CHARACTERS} caracteres."
            )));
        }

        let escaped = normalized
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        Ok(Self(Some(escaped)))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionDto {
    id: Uuid,
    slug: String,
    name: String,
    series_name: Option<String>,
    release_date: String,
    total_cards: i32,
    cover_image_url: Option<String>,
    language: String,
}

#[derive(FromRow)]
struct CollectionRow {
    id: Uuid,
    slug: String,
    name: String,
    series_name: Option<String>,
    release_date: String,
    total_cards: i32,
    cover_image_url: Option<String>,
    language: String,
}

impl From<CollectionRow> for CollectionDto {
    fn from(row: CollectionRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            name: row.name,
            series_name: row.series_name,
            release_date: row.release_date,
            total_cards: row.total_cards,
            cover_image_url: row.cover_image_url,
            language: row.language,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardDto {
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
}

#[derive(FromRow)]
struct CardRow {
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
}

impl From<CardRow> for CardDto {
    fn from(row: CardRow) -> Self {
        Self {
            id: row.id,
            set_id: row.set_id,
            local_number: row.local_number,
            printed_number: row.printed_number,
            name: row.name,
            rarity: row.rarity,
            artist: row.artist,
            image_small_url: row.image_small_url,
            image_large_url: row.image_large_url,
            sort_order: row.sort_order,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PaginationDto {
    page: u32,
    page_size: u32,
    total_items: i64,
    total_pages: i64,
}

impl PaginationDto {
    fn new(pagination: Pagination, total_items: i64) -> Self {
        let total_pages = if total_items == 0 {
            0
        } else {
            (total_items + i64::from(pagination.page_size) - 1) / i64::from(pagination.page_size)
        };
        Self {
            page: pagination.page,
            page_size: pagination.page_size,
            total_items,
            total_pages,
        }
    }
}

#[derive(Serialize)]
struct ResourceResponse<T> {
    data: T,
}

async fn list_sets(
    State(pool): State<PgPool>,
    Extension(request_id): Extension<RequestId>,
    query: Result<Query<RawCatalogQuery>, QueryRejection>,
) -> Result<Json<PaginatedResponse<CollectionDto>>, ApiError> {
    let query = parse_query(query, &request_id)?;
    let pagination =
        Pagination::parse(&query).map_err(|error| ApiError::validation(error, &request_id))?;
    let search = SearchTerm::parse(query.search.as_deref())
        .map_err(|error| ApiError::validation(error, &request_id))?;
    let sort = SetSort::parse(query.sort.as_deref())
        .map_err(|error| ApiError::validation(error, &request_id))?;

    let (rows, total_items) = query_sets(&pool, &search, pagination, sort)
        .await
        .map_err(|error| ApiError::database(error, &request_id))?;
    Ok(Json(PaginatedResponse {
        data: rows.into_iter().map(CollectionDto::from).collect(),
        pagination: PaginationDto::new(pagination, total_items),
    }))
}

async fn get_set(
    State(pool): State<PgPool>,
    Extension(request_id): Extension<RequestId>,
    Path(raw_set_id): Path<String>,
) -> Result<Json<ResourceResponse<CollectionDto>>, ApiError> {
    let set_id = parse_id(&raw_set_id, &request_id)?;
    let collection = query_set(&pool, set_id)
        .await
        .map_err(|error| ApiError::database(error, &request_id))?
        .ok_or_else(|| ApiError::not_found("Coleção não encontrada.", &request_id))?;
    Ok(Json(ResourceResponse {
        data: collection.into(),
    }))
}

async fn list_cards(
    State(pool): State<PgPool>,
    Extension(request_id): Extension<RequestId>,
    Path(raw_set_id): Path<String>,
    query: Result<Query<RawCatalogQuery>, QueryRejection>,
) -> Result<Json<PaginatedResponse<CardDto>>, ApiError> {
    let set_id = parse_id(&raw_set_id, &request_id)?;
    if query_set(&pool, set_id)
        .await
        .map_err(|error| ApiError::database(error, &request_id))?
        .is_none()
    {
        return Err(ApiError::not_found("Coleção não encontrada.", &request_id));
    }

    let query = parse_query(query, &request_id)?;
    let pagination =
        Pagination::parse(&query).map_err(|error| ApiError::validation(error, &request_id))?;
    let search = SearchTerm::parse(query.search.as_deref())
        .map_err(|error| ApiError::validation(error, &request_id))?;
    let sort = CardSort::parse(query.sort.as_deref())
        .map_err(|error| ApiError::validation(error, &request_id))?;
    let (rows, total_items) = query_cards(&pool, set_id, &search, pagination, sort)
        .await
        .map_err(|error| ApiError::database(error, &request_id))?;

    Ok(Json(PaginatedResponse {
        data: rows.into_iter().map(CardDto::from).collect(),
        pagination: PaginationDto::new(pagination, total_items),
    }))
}

fn parse_query(
    query: Result<Query<RawCatalogQuery>, QueryRejection>,
    request_id: &RequestId,
) -> Result<RawCatalogQuery, ApiError> {
    query.map(|Query(query)| query).map_err(|_| {
        ApiError::bad_request(
            "invalid_query",
            "Os parâmetros da consulta são inválidos.",
            request_id,
        )
    })
}

fn parse_id(raw_id: &str, request_id: &RequestId) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw_id).map_err(|_| {
        ApiError::bad_request(
            "invalid_id",
            "O identificador da coleção é inválido.",
            request_id,
        )
    })
}

async fn query_sets(
    pool: &PgPool,
    search: &SearchTerm,
    pagination: Pagination,
    sort: SetSort,
) -> Result<(Vec<CollectionRow>, i64), sqlx::Error> {
    let total_items = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sets WHERE is_published = TRUE AND ($1::text IS NULL OR name ILIKE '%' || $1 || '%' ESCAPE '\\')",
    )
    .bind(&search.0)
    .fetch_one(pool)
    .await?;
    let rows = sqlx::query_as::<_, CollectionRow>(
        "SELECT id, slug, name, series_name, release_date::text, total_cards, cover_image_url, language FROM sets WHERE is_published = TRUE AND ($1::text IS NULL OR name ILIKE '%' || $1 || '%' ESCAPE '\\') ORDER BY CASE WHEN $2 = 'release_date_asc' THEN release_date END ASC, CASE WHEN $2 = 'release_date_desc' THEN release_date END DESC, CASE WHEN $2 = 'name_asc' THEN lower(name) END ASC, CASE WHEN $2 = 'name_desc' THEN lower(name) END DESC, id ASC LIMIT $3 OFFSET $4",
    )
    .bind(&search.0)
    .bind(sort.key())
    .bind(i64::from(pagination.page_size))
    .bind(pagination.offset())
    .fetch_all(pool)
    .await?;
    Ok((rows, total_items))
}

async fn query_set(pool: &PgPool, set_id: Uuid) -> Result<Option<CollectionRow>, sqlx::Error> {
    sqlx::query_as::<_, CollectionRow>(
        "SELECT id, slug, name, series_name, release_date::text, total_cards, cover_image_url, language FROM sets WHERE id = $1 AND is_published = TRUE",
    )
    .bind(set_id)
    .fetch_optional(pool)
    .await
}

async fn query_cards(
    pool: &PgPool,
    set_id: Uuid,
    search: &SearchTerm,
    pagination: Pagination,
    sort: CardSort,
) -> Result<(Vec<CardRow>, i64), sqlx::Error> {
    let total_items = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cards WHERE set_id = $1 AND is_published = TRUE AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%' ESCAPE '\\' OR local_number ILIKE '%' || $2 || '%' ESCAPE '\\' OR printed_number ILIKE '%' || $2 || '%' ESCAPE '\\')",
    )
    .bind(set_id)
    .bind(&search.0)
    .fetch_one(pool)
    .await?;
    let rows = sqlx::query_as::<_, CardRow>(
        "SELECT id, set_id, local_number, printed_number, name, rarity, artist, image_small_url, image_large_url, sort_order FROM cards WHERE set_id = $1 AND is_published = TRUE AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%' ESCAPE '\\' OR local_number ILIKE '%' || $2 || '%' ESCAPE '\\' OR printed_number ILIKE '%' || $2 || '%' ESCAPE '\\') ORDER BY CASE WHEN $3 = 'number_asc' THEN sort_order END ASC, CASE WHEN $3 = 'number_desc' THEN sort_order END DESC, CASE WHEN $3 = 'name_asc' THEN lower(name) END ASC, CASE WHEN $3 = 'name_desc' THEN lower(name) END DESC, id ASC LIMIT $4 OFFSET $5",
    )
    .bind(set_id)
    .bind(&search.0)
    .bind(sort.key())
    .bind(i64::from(pagination.page_size))
    .bind(pagination.offset())
    .fetch_all(pool)
    .await?;
    Ok((rows, total_items))
}

#[derive(Debug)]
struct ValidationError {
    message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    request_id: String,
}

impl ApiError {
    fn bad_request(code: &'static str, message: impl Into<String>, request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
            request_id: request_id.0.clone(),
        }
    }

    fn validation(error: ValidationError, request_id: &RequestId) -> Self {
        Self::bad_request("invalid_parameter", error.message, request_id)
    }

    fn not_found(message: impl Into<String>, request_id: &RequestId) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "catalog_not_found",
            message: message.into(),
            request_id: request_id.0.clone(),
        }
    }

    fn database(error: sqlx::Error, request_id: &RequestId) -> Self {
        error!(request_id = %request_id.0, error = %error, "catalog query failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "Não foi possível consultar o catálogo agora.".to_owned(),
            request_id: request_id.0.clone(),
        }
    }
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: &'static str,
    message: String,
    request_id: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code: self.code,
                    message: self.message,
                    request_id: self.request_id,
                },
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{Pagination, RawCatalogQuery, SearchTerm};

    #[test]
    fn search_normalizes_spaces_and_escapes_like_metacharacters() {
        let search = SearchTerm::parse(Some("  aura   100%_  "))
            .expect("search should be valid")
            .0;
        assert_eq!(search.as_deref(), Some("aura 100\\%\\_"));
    }

    #[test]
    fn pagination_applies_defaults_and_rejects_oversized_pages() {
        let default = Pagination::parse(&RawCatalogQuery::default())
            .expect("default pagination should be valid");
        assert_eq!((default.page, default.page_size), (1, 20));

        let oversized = RawCatalogQuery {
            page_size: Some("101".to_owned()),
            ..RawCatalogQuery::default()
        };
        assert!(Pagination::parse(&oversized).is_err());
    }
}
