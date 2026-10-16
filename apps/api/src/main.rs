use blog_api::db::Store;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let token = std::env::var("BLOG_API_TOKEN").unwrap_or_default();
    if token.len() < 24 {
        eprintln!("BLOG_API_TOKEN must be set to at least 24 characters");
        std::process::exit(2);
    }
    let path = std::env::var("BLOG_API_DB").unwrap_or_else(|_| "blog.db".into());
    let bind = std::env::var("BLOG_API_BIND").unwrap_or_else(|_| "127.0.0.1:7841".into());
    let origins = std::env::var("BLOG_API_ORIGINS")
        .unwrap_or_else(|_| "https://blog.life2harsh.xyz,https://life2harsh.xyz".into())
        .split(',')
        .map(|origin| origin.trim().to_string())
        .filter(|origin| !origin.is_empty())
        .collect::<Vec<_>>();

    let store = match Store::open(&path) {
        Ok(store) => store,
        Err(cause) => {
            eprintln!("cannot open database {path}: {cause}");
            std::process::exit(1);
        }
    };
    let media = std::env::var("BLOG_API_MEDIA").map_or_else(
        |_| {
            std::path::Path::new(&path)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("media")
        },
        std::path::PathBuf::from,
    );
    if let Err(cause) = std::fs::create_dir_all(&media) {
        eprintln!("cannot create media directory {}: {cause}", media.display());
        std::process::exit(1);
    }
    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(cause) => {
            eprintln!("cannot bind {bind}: {cause}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "blog-api listening on {bind}, database {path}, media {}",
        media.display()
    );
    let app = blog_api::router(store, token, &origins, media);
    if let Err(cause) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
    {
        eprintln!("server error: {cause}");
        std::process::exit(1);
    }
}

async fn shutdown() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = interrupt => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = interrupt.await;
}
