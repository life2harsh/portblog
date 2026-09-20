pub mod db;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use content::{EntryDraft, EntryKind};
use serde::Deserialize;
use tower_http::cors::{AllowOrigin, CorsLayer};

use db::{Store, StoreError};

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
    token: Arc<String>,
}

impl AppState {
    fn store(&self) -> MutexGuard<'_, Store> {
        self.store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        let Some(given) = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
        else {
            return false;
        };
        constant_time_eq(given.as_bytes(), self.token.as_bytes())
    }
}

pub fn router(store: Store, token: String, origins: &[String]) -> Router {
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        token: Arc::new(token),
    };
    let origins = origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(3600));
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/entries", get(list).post(create))
        .route("/entries/{slug}", get(read).put(update).delete(remove))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(cors)
        .with_state(state)
}

#[derive(Deserialize)]
struct ListQuery {
    kind: Option<String>,
    drafts: Option<bool>,
}

async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let drafts = query.drafts.unwrap_or(false);
    if drafts && !state.authorized(&headers) {
        return unauthorized().await;
    }
    let kind = match query.kind.as_deref() {
        None | Some("") | Some("all") => None,
        Some(value) => match EntryKind::parse(value) {
            Some(kind) => Some(kind),
            None => return error(StatusCode::BAD_REQUEST, "unknown kind"),
        },
    };
    respond(state.store().list(drafts, kind))
}

async fn read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    let drafts = state.authorized(&headers);
    respond(state.store().get(&slug, drafts))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(draft): Json<EntryDraft>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized().await;
    }
    match state.store().create(draft, now()) {
        Ok(entry) => (StatusCode::CREATED, Json(entry)).into_response(),
        Err(failure) => store_error(failure),
    }
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(draft): Json<EntryDraft>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized().await;
    }
    respond(state.store().update(&slug, draft, now()))
}

async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized().await;
    }
    match state.store().delete(&slug) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(failure) => store_error(failure),
    }
}

fn respond<T: serde::Serialize>(result: Result<T, StoreError>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(failure) => store_error(failure),
    }
}

fn store_error(failure: StoreError) -> Response {
    match failure {
        StoreError::NotFound => error(StatusCode::NOT_FOUND, "not found"),
        StoreError::Invalid(reason) => error(StatusCode::BAD_REQUEST, reason),
        StoreError::Database(cause) => {
            eprintln!("database error: {cause}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "database error")
        }
    }
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

async fn unauthorized() -> Response {
    tokio::time::sleep(Duration::from_millis(250)).await;
    error(StatusCode::UNAUTHORIZED, "unauthorized")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}
