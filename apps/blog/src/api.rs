use std::fmt;

use content::{Entry, EntryDraft};
use gloo_net::http::{Request, RequestBuilder, Response};

pub const BASE: &str = match option_env!("BLOG_API") {
    Some(base) => base,
    None => "/api",
};
const TOKEN_KEY: &str = "life2harsh.blog-token";

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    Unauthorized,
    NotFound,
    Rejected(String),
    Unreachable,
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized => write!(formatter, "that token was not accepted"),
            Self::NotFound => write!(formatter, "entry not found"),
            Self::Rejected(reason) => write!(formatter, "{reason}"),
            Self::Unreachable => write!(formatter, "the journal api is unreachable"),
        }
    }
}

fn storage() -> Option<web_sys::Storage> {
    gloo_utils::window().local_storage().ok().flatten()
}

pub fn stored_token() -> Option<String> {
    storage()
        .and_then(|storage| storage.get_item(TOKEN_KEY).ok().flatten())
        .filter(|token| !token.is_empty())
}

pub fn store_token(token: Option<&str>) {
    if let Some(storage) = storage() {
        let _ = match token {
            Some(token) => storage.set_item(TOKEN_KEY, token),
            None => storage.remove_item(TOKEN_KEY),
        };
    }
}

fn authed(request: RequestBuilder, token: Option<&str>) -> RequestBuilder {
    match token {
        Some(token) => request.header("Authorization", &format!("Bearer {token}")),
        None => request,
    }
}

async fn checked(response: Result<Response, gloo_net::Error>) -> Result<Response, ApiError> {
    let response = response.map_err(|_| ApiError::Unreachable)?;
    match response.status() {
        200..=299 => Ok(response),
        401 => Err(ApiError::Unauthorized),
        404 => Err(ApiError::NotFound),
        _ => {
            let reason = response
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|body| body["error"].as_str().map(str::to_string))
                .unwrap_or_else(|| format!("request failed ({})", response.status()));
            Err(ApiError::Rejected(reason))
        }
    }
}

pub async fn list(token: Option<&str>) -> Result<Vec<Entry>, ApiError> {
    let url = if token.is_some() {
        format!("{BASE}/entries?drafts=true")
    } else {
        format!("{BASE}/entries")
    };
    let response = checked(authed(Request::get(&url), token).send().await).await?;
    response.json().await.map_err(|_| ApiError::Unreachable)
}

pub async fn save(slug: Option<&str>, draft: &EntryDraft, token: &str) -> Result<Entry, ApiError> {
    let request = match slug {
        Some(slug) => Request::put(&format!("{BASE}/entries/{slug}")),
        None => Request::post(&format!("{BASE}/entries")),
    };
    let request = authed(request, Some(token))
        .json(draft)
        .map_err(|_| ApiError::Rejected("could not encode entry".into()))?;
    let response = checked(request.send().await).await?;
    response.json().await.map_err(|_| ApiError::Unreachable)
}

pub async fn remove(slug: &str, token: &str) -> Result<(), ApiError> {
    checked(
        authed(
            Request::delete(&format!("{BASE}/entries/{slug}")),
            Some(token),
        )
        .send()
        .await,
    )
    .await
    .map(|_| ())
}
