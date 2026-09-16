use axum::{Router, http::StatusCode, routing::any};
use clap::Parser;
use scribewatch::{api, config::Config, quick, state::AppState, workflows};
use tokio_util::sync::CancellationToken;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "scribewatch=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::parse();
    let state = AppState::load(config).await?;
    let shutdown = CancellationToken::new();
    tokio::spawn(workflows::run_supervisor(state.clone(), shutdown.clone()));
    let cleanup_state = state.clone();
    let cleanup_shutdown = shutdown.clone();
    tokio::spawn(async move {
        loop {
            if let Err(error) = quick::cleanup_stale(&cleanup_state) {
                tracing::warn!(%error, "Quick Transcribe cleanup failed");
            }
            tokio::select! {
                _ = cleanup_shutdown.cancelled() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(3600)) => {}
            }
        }
    });

    let mut app = Router::new()
        .merge(api::router())
        .route("/api", any(api_not_found))
        .route("/api/", any(api_not_found))
        .route("/api/{*path}", any(api_not_found))
        .layer(TraceLayer::new_for_http());
    if state.config.dist_dir.is_dir() {
        let index = state.config.dist_dir.join("index.html");
        app = app.fallback_service(
            ServeDir::new(&state.config.dist_dir)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(index)),
        );
    }

    let app = app.with_state(state.clone());
    let address = format!("{}:{}", state.config.host, state.config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(%address, "ScribeWatch started");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown))
        .await?;
    Ok(())
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn shutdown_signal(token: CancellationToken) {
    #[cfg(unix)]
    {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = async {
                if let Some(signal) = term.as_mut() { signal.recv().await; }
            } => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    token.cancel();
}
