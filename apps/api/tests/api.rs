use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use blog_api::db::Store;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "test-token-that-is-long-enough";

fn media() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "blog-api-media-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn app() -> Router {
    app_with(media())
}

fn app_with(media: std::path::PathBuf) -> Router {
    blog_api::router(
        Store::open(":memory:").unwrap(),
        TOKEN.to_string(),
        &["https://blog.life2harsh.xyz".to_string()],
        media,
    )
}

async fn upload(app: &Router, kind: &str, auth: bool, body: Vec<u8>) -> (StatusCode, Value) {
    let mut request = Request::post("/uploads").header(header::CONTENT_TYPE, kind);
    if auth {
        request = request.header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    auth: bool,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if auth {
        request = request.header(header::AUTHORIZATION, format!("Bearer {TOKEN}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => request.body(Body::empty()).unwrap(),
    };
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn health() {
    let app = app();
    let response = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn writes_need_the_token() {
    let app = app();
    let (status, _) = send(&app, "POST", "/entries", false, Some(json!({"body": "hi"}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let request = Request::post("/entries")
        .header(
            header::AUTHORIZATION,
            "Bearer wrong-token-of-similar-length!",
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"body": "hi"}).to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let (status, _) = send(&app, "GET", "/entries?drafts=true", false, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn note_and_essay_lifecycle() {
    let app = app();
    let (status, note) = send(
        &app,
        "POST",
        "/entries",
        true,
        Some(json!({"body": "a quick thought about rings and math", "tags": ["Math", "math", "Rings!"]})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(note["kind"], "note");
    assert_eq!(note["slug"], "a-quick-thought-about-rings-and");
    assert_eq!(note["tags"], json!(["math", "rings"]));
    assert_eq!(note["published"], true);

    let (_, essay) = send(
        &app,
        "POST",
        "/entries",
        true,
        Some(json!({"kind": "essay", "title": "On Rings", "summary": "  why rings  ", "body": "# long\n\nbody", "published": false})),
    )
    .await;
    assert_eq!(essay["slug"], "on-rings");
    assert_eq!(essay["summary"], "why rings");

    let (_, duplicate) = send(
        &app,
        "POST",
        "/entries",
        true,
        Some(json!({"kind": "essay", "title": "On Rings", "body": "again"})),
    )
    .await;
    assert_eq!(duplicate["slug"], "on-rings-2");

    let (_, public) = send(&app, "GET", "/entries", false, None).await;
    let slugs = public
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["slug"].clone())
        .collect::<Vec<_>>();
    assert!(
        !slugs.contains(&json!("on-rings")),
        "drafts must stay private"
    );
    assert!(slugs.contains(&json!("on-rings-2")));

    let (status, _) = send(&app, "GET", "/entries/on-rings", false, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, draft) = send(&app, "GET", "/entries/on-rings", true, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(draft["published"], false);

    let (_, all) = send(&app, "GET", "/entries?drafts=true", true, None).await;
    assert_eq!(all.as_array().unwrap().len(), 3);
    let (_, essays) = send(&app, "GET", "/entries?kind=essay", false, None).await;
    assert_eq!(essays.as_array().unwrap().len(), 1);
    let (status, _) = send(&app, "GET", "/entries?kind=poem", false, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, updated) = send(
        &app,
        "PUT",
        "/entries/on-rings",
        true,
        Some(json!({"published": true, "body": "revised"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["body"], "revised");
    assert_eq!(updated["title"], "On Rings");
    assert_eq!(updated["summary"], "why rings");
    let (_, cleared) = send(
        &app,
        "PUT",
        "/entries/on-rings",
        true,
        Some(json!({"summary": ""})),
    )
    .await;
    assert_eq!(cleared["summary"], Value::Null);
    let (status, _) = send(&app, "GET", "/entries/on-rings", false, None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        &app,
        "PUT",
        "/entries/on-rings",
        true,
        Some(json!({"body": "   "})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(&app, "DELETE", "/entries/on-rings", false, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(&app, "DELETE", "/entries/on-rings", true, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, "DELETE", "/entries/on-rings", true, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rejects_empty_and_oversized_bodies() {
    let app = app();
    let (status, _) = send(
        &app,
        "POST",
        "/entries",
        true,
        Some(json!({"body": "  \n "})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let huge = "x".repeat(300 * 1024);
    let (status, _) = send(&app, "POST", "/entries", true, Some(json!({"body": huge}))).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn cors_only_for_known_origins() {
    let app = app();
    let preflight = |origin: &str| {
        Request::builder()
            .method("OPTIONS")
            .uri("/entries")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap()
    };
    let allowed = app
        .clone()
        .oneshot(preflight("https://blog.life2harsh.xyz"))
        .await
        .unwrap();
    assert_eq!(
        allowed
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "https://blog.life2harsh.xyz"
    );
    let denied = app
        .oneshot(preflight("https://evil.example"))
        .await
        .unwrap();
    assert!(
        denied
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
}

#[test]
fn migrates_databases_without_summary() {
    let path = std::env::temp_dir().join(format!("blog-api-migrate-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE entries (slug TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT, body TEXT NOT NULL,
                 tags TEXT NOT NULL DEFAULT '[]', published INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                 INSERT INTO entries VALUES ('old', 'essay', 'Old', 'body', '[]', 1, 1, 1);",
            )
            .unwrap();
    }
    let store = Store::open(path.to_str().unwrap()).unwrap();
    let entry = store.get("old", false).unwrap();
    assert_eq!(entry.summary, None);
    assert_eq!(entry.title.as_deref(), Some("Old"));
    drop(store);
    let reopened = Store::open(path.to_str().unwrap()).unwrap();
    assert!(reopened.get("old", false).is_ok());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn uploads_are_served_back() {
    let dir = media();
    let app = app_with(dir.clone());
    let picture = b"\x89PNG\r\n\x1a\nnot really a png".to_vec();
    let (status, value) = upload(&app, "image/png", true, picture.clone()).await;
    assert_eq!(status, StatusCode::CREATED);
    let url = value["url"].as_str().unwrap().to_string();
    assert!(url.starts_with("/media/") && url.ends_with(".png"));
    let (_, again) = upload(&app, "image/png", true, picture.clone()).await;
    assert_eq!(again["url"], value["url"]);

    let response = app
        .clone()
        .oneshot(Request::get(&url).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
    assert!(
        response.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("immutable")
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), picture.as_slice());
    let leftovers = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with('.')
        })
        .count();
    assert_eq!(leftovers, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn uploads_are_checked() {
    let dir = media();
    let app = app_with(dir.clone());
    let (status, _) = upload(&app, "image/png", false, b"x".to_vec()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = upload(&app, "image/svg+xml", true, b"<svg/>".to_vec()).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let (status, _) = upload(&app, "text/html", true, b"<p>".to_vec()).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let (status, _) = upload(&app, "image/png", true, Vec::new()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = upload(&app, "video/mp4", true, vec![0; blog_api::MEDIA_LIMIT + 1]).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    let response = app
        .clone()
        .oneshot(
            Request::get("/media/../Cargo.toml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::OK);
    let _ = std::fs::remove_dir_all(&dir);
}
