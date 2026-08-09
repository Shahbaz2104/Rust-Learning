use tokio::net::TcpListener;
use url_shortener::{app, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Database file from the DATABASE_URL env var, default a local links.db.
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://links.db".to_string());
    let state = AppState::new(&db_url).await?;
    let app = app(state);

    // Port from the PORT env var (used by containers/platforms), default 3000.
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
