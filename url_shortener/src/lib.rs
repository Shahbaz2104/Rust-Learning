//! url_shortener — a tiny REST API that turns long URLs into short codes.
//!
//! Library crate: all the logic lives here so it can be unit-tested.
//! The binary (`main.rs`) only starts the server.
//!
//! Endpoints:
//!   POST /shorten     body: {"url": "https://example.com/very/long"}  -> {"short": "0"}
//!   GET /{code}       307-redirects to the stored URL
//!   GET /stats/{code} -> {"short": "0", "clicks": 3}
//!
//! Links are persisted in SQLite, so they survive server restarts.

use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Everything the handlers need: a pool of connections to the SQLite database.
/// `SqlitePool` is cheap to clone and handles concurrency, so no `Mutex` needed.
#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
}

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS links (
    code TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    clicks INTEGER NOT NULL DEFAULT 0
);";

impl AppState {
    /// Connect to the database at `db_url` and make sure the schema exists.
    /// The DB file is created if it doesn't exist yet.
    pub async fn new(db_url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(db_url)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        Self::with_pool(pool).await
    }

    /// A throwaway in-memory database for tests. Locked to one connection,
    /// because an in-memory SQLite DB only lives as long as its connection.
    pub async fn new_in_memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        Self::with_pool(pool).await
    }

    async fn with_pool(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(Self { pool })
    }
}

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

#[derive(Serialize)]
pub struct StatsResponse {
    pub short: String,
    pub clicks: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Every error the API can return, converted into an HTTP response.
pub enum AppError {
    BadRequest,
    NotFound,
    Internal(Box<dyn std::error::Error>),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::BadRequest => (StatusCode::BAD_REQUEST, "bad request"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found"),
            AppError::Internal(e) => {
                eprintln!("internal error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };
        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(Box::new(e))
    }
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
    State(state): State<AppState>,
    Json(payload): Json<ShortenRequest>,
) -> Result<Json<ShortenResponse>, AppError> {
    let url = payload.url.trim().to_string();
    if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::BadRequest);
    }

    // Next code = how many links already exist (we never delete, so no collisions).
    let next_id: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM links")
        .fetch_one(&state.pool)
        .await?;
    let code = encode(next_id as u64);

    sqlx::query("INSERT INTO links (code, url) VALUES (?, ?)")
        .bind(&code)
        .bind(&url)
        .execute(&state.pool)
        .await?;

    Ok(Json(ShortenResponse { short: code }))
}

/// GET /{code} — count the click, look up the URL, and redirect.
pub async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Redirect, AppError> {
    // Increment first; `rows_affected()` tells us whether the code existed.
    let updated = sqlx::query("UPDATE links SET clicks = clicks + 1 WHERE code = ?")
        .bind(&code)
        .execute(&state.pool)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let url: String = sqlx::query_scalar("SELECT url FROM links WHERE code = ?")
        .bind(&code)
        .fetch_one(&state.pool)
        .await?;

    Ok(Redirect::temporary(&url))
}

/// GET /stats/{code} — report how many times a link has been followed.
pub async fn stats(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<StatsResponse>, AppError> {
    let clicks: Option<i64> = sqlx::query_scalar("SELECT clicks FROM links WHERE code = ?")
        .bind(&code)
        .fetch_optional(&state.pool)
        .await?;

    match clicks {
        Some(clicks) => Ok(Json(StatsResponse {
            short: code,
            clicks: clicks as u64,
        })),
        None => Err(AppError::NotFound),
    }
}

// ---------------------------------------------------------------------------
// App builder
// ---------------------------------------------------------------------------

/// Build the router with the given shared state. Tests use this too.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/shorten", axum::routing::post(shorten))
        .route("/stats/{code}", get(stats))
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
}
