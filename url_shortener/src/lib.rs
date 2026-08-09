//! url_shortener — a tiny REST API that turns long URLs into short codes.
//!
//! Library crate: all the logic lives here so it can be unit-tested.
//! The binary (`main.rs`) only starts the server.
//!
//! Endpoints:
//!   POST /shorten   body: {"url": "https://example.com/very/long"}  -> {"short": "0"}
//!   GET /{code}     307-redirects to the stored URL

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// What every request handler needs: the map of code -> url, plus a counter
/// so each new link gets a unique code. `Arc<Mutex<..>>` lets many requests
/// touch it at once without data races.
#[derive(Default)]
pub struct AppState {
    links: HashMap<String, String>,
    next_id: u64,
}

impl AppState {
    /// A fresh, empty store (next code will be "0").
    pub fn new() -> Self {
        Self::default()
    }
}

pub type AppStateRef = Arc<Mutex<AppState>>;

// ---------------------------------------------------------------------------
// Request / response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ShortenRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    pub short: String,
}

// ---------------------------------------------------------------------------
// Short code generation
// ---------------------------------------------------------------------------

/// Base-62 digits: 0-9, a-z, A-Z. Short, URL-safe strings.
const ALPHABET: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Convert a number to a base-62 string (same idea as base 10, / 62).
pub fn encode(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut digits = Vec::new();
    let mut n = n;
    while n > 0 {
        digits.push(ALPHABET[(n % 62) as usize]);
        n /= 62;
    }
    digits.reverse();
    digits.iter().map(|&c| c as char).collect()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /shorten — validate the URL, mint a code, store the pair, return it.
pub async fn shorten(
    State(state): State<AppStateRef>,
    Json(payload): Json<ShortenRequest>,
) -> Result<Json<ShortenResponse>, StatusCode> {
    let url = payload.url.trim().to_string();
    if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut state = state.lock().expect("state mutex poisoned");
    let code = encode(state.next_id);
    state.next_id += 1;
    state.links.insert(code.clone(), url);

    Ok(Json(ShortenResponse { short: code }))
}

/// GET /{code} — look up the URL and redirect the browser there.
pub async fn redirect(
    State(state): State<AppStateRef>,
    Path(code): Path<String>,
) -> Result<Redirect, StatusCode> {
    let state = state.lock().expect("state mutex poisoned");
    match state.links.get(&code) {
        Some(url) => Ok(Redirect::temporary(url)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ---------------------------------------------------------------------------
// App builder
// ---------------------------------------------------------------------------

/// Build the router with the given shared state. Tests use this too.
pub fn app(state: AppStateRef) -> Router {
    Router::new()
        .route("/shorten", axum::routing::post(shorten))
        .route("/{code}", get(redirect))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_zero() {
        assert_eq!(encode(0), "0");
    }

    #[test]
    fn encode_single_digits() {
        assert_eq!(encode(1), "1");
        assert_eq!(encode(10), "a");
        assert_eq!(encode(61), "Z");
    }

    #[test]
    fn encode_carries() {
        // 62 in base 10 is "10" in base 62
        assert_eq!(encode(62), "10");
        assert_eq!(encode(62 * 62), "100");
    }

    #[test]
    fn app_state_starts_empty() {
        let state = AppState::new();
        assert!(state.links.is_empty());
        assert_eq!(state.next_id, 0);
    }
}
