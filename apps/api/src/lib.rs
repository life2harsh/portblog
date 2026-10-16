pub mod db;

use std::hash::{DefaultHasher, Hasher};
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use content::{EntryDraft, EntryKind};
use http_body_util::BodyExt;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

use db::{Store, StoreError};

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
    token: Arc<String>,
    media: Arc<PathBuf>,
}

pub const MEDIA_LIMIT: usize = 48 * 1024 * 1024;

const MEDIA_TYPES: [(&str, &str); 8] = [
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
    ("image/avif", "avif"),
    ("video/mp4", "mp4"),
    ("video/webm", "webm"),
    ("video/quicktime", "mov"),
];

static UPLOADS: AtomicU64 = AtomicU64::new(0);

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

pub fn router(store: Store, token: String, origins: &[String], media: PathBuf) -> Router {
    let files = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .service(ServeDir::new(&media).append_index_html_on_directories(false));
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        token: Arc::new(token),
        media: Arc::new(media),
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
        .route("/uploads", post(upload))
        .nest_service("/media", files)
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

async fn upload(State(state): State<AppState>, headers: HeaderMap, body: Body) -> Response {
    if !state.authorized(&headers) {
        return unauthorized().await;
    }
    let Some(extension) = media_extension(&headers) else {
        return error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "only images and mp4, webm or mov video",
        );
    };
    let partial = state.media.join(format!(
        ".upload-{}-{}",
        now(),
        UPLOADS.fetch_add(1, Ordering::Relaxed)
    ));
    let outcome = receive(body, &partial).await;
    let digest = match outcome {
        Ok(digest) => digest,
        Err(failure) => {
            let _ = tokio::fs::remove_file(&partial).await;
            return match failure {
                Received::TooLarge => {
                    error(StatusCode::PAYLOAD_TOO_LARGE, "files can be up to 48 MB")
                }
                Received::Empty => error(StatusCode::BAD_REQUEST, "that file is empty"),
                Received::Broken(cause) => {
                    eprintln!("upload failed: {cause}");
                    error(StatusCode::INTERNAL_SERVER_ERROR, "upload failed")
                }
            };
        }
    };
    let name = format!("{digest:016x}.{extension}");
    if let Err(cause) = tokio::fs::rename(&partial, state.media.join(&name)).await {
        eprintln!("upload failed: {cause}");
        let _ = tokio::fs::remove_file(&partial).await;
        return error(StatusCode::INTERNAL_SERVER_ERROR, "upload failed");
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "url": format!("/media/{name}") })),
    )
        .into_response()
}

fn media_extension(headers: &HeaderMap) -> Option<&'static str> {
    let kind = headers
        .get(header::CONTENT_TYPE)?
        .to_str()
        .ok()?
        .split(';')
        .next()?
        .trim()
        .to_ascii_lowercase();
    MEDIA_TYPES
        .iter()
        .find(|(known, _)| *known == kind)
        .map(|(_, extension)| *extension)
}

enum Received {
    TooLarge,
    Empty,
    Broken(String),
}

async fn receive(mut body: Body, path: &FsPath) -> Result<u64, Received> {
    let broken = |cause: std::io::Error| Received::Broken(cause.to_string());
    let mut file = tokio::fs::File::create(path).await.map_err(broken)?;
    let mut hasher = DefaultHasher::new();
    let mut size = 0;
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|cause| Received::Broken(cause.to_string()))?;
        let Ok(chunk) = frame.into_data() else {
            continue;
        };
        size += chunk.len();
        if size > MEDIA_LIMIT {
            return Err(Received::TooLarge);
        }
        hasher.write(&chunk);
        file.write_all(&chunk).await.map_err(broken)?;
    }
    if size == 0 {
        return Err(Received::Empty);
    }
    file.flush().await.map_err(broken)?;
    file.sync_all().await.map_err(broken)?;
    hasher.write_usize(size);
    Ok(hasher.finish())
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
