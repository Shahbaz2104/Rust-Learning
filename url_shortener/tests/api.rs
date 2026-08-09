use std::sync::{Arc, Mutex};

use axum::{body::Body, http::{Request, StatusCode}, Router};
use http_body_util::BodyExt;
use tower::ServiceExt;

use url_shortener::{app, AppState};

/// Build a fresh router on its own state, so tests don't share data.
fn test_app() -> Router {
    app(Arc::new(Mutex::new(AppState::new())))
}

/// POST a JSON body to /shorten and read back the response as (status, text).
async fn post_shorten(body: &str) -> (StatusCode, String) {
    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/shorten")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn shorten_returns_a_code() {
    let (status, body) = post_shorten(r#"{"url":"https://www.rust-lang.org/learn"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["short"], "0");
}

#[tokio::test]
async fn shorten_rejects_bad_url() {
    let (status, _) = post_shorten(r#"{"url":"not-a-url"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn shorten_rejects_empty_url() {
    let (status, _) = post_shorten(r#"{"url":""}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn redirect_returns_the_original_url() {
    let state = Arc::new(Mutex::new(AppState::new()));

    // 1. create a short link
    let shorten_response = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/shorten")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url":"https://example.com/a/very/long/path"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = shorten_response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let code = json["short"].as_str().unwrap();

    // 2. follow it
    let redirect_response = app(state)
        .oneshot(
            Request::builder()
                .uri(format!("/{code}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(redirect_response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        redirect_response.headers()["location"],
        "https://example.com/a/very/long/path"
    );
}

#[tokio::test]
async fn redirect_unknown_code_is_404() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Shorten a URL and return the assigned short code, reusing one shared state.
async fn shorten_on(state: &Arc<Mutex<AppState>>, url: &str) -> String {
    let response = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/shorten")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"url":"{url}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["short"].as_str().unwrap().to_string()
}

async fn get_clicks(state: &Arc<Mutex<AppState>>, code: &str) -> u64 {
    let response = app(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/stats/{code}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["clicks"].as_u64().unwrap()
}

#[tokio::test]
async fn stats_starts_at_zero() {
    let state = Arc::new(Mutex::new(AppState::new()));
    let code = shorten_on(&state, "https://example.com").await;
    assert_eq!(get_clicks(&state, &code).await, 0);
}

#[tokio::test]
async fn stats_counts_redirects() {
    let state = Arc::new(Mutex::new(AppState::new()));
    let code = shorten_on(&state, "https://example.com").await;

    for _ in 0..3 {
        let response = app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/{code}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    }

    assert_eq!(get_clicks(&state, &code).await, 3);
}

#[tokio::test]
async fn stats_unknown_code_is_404() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/stats/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
