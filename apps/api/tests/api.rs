use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use blog_api::db::Store;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "test-token-that-is-long-enough";

fn app() -> Router {
    blog_api::router(
        Store::open(":memory:").unwrap(),
        TOKEN.to_string(),
        &["https://blog.life2harsh.xyz".to_string()],
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
