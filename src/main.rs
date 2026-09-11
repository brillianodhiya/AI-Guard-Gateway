mod config;
mod handlers;
mod models;
mod services;

use axum::{
    routing::{get, post},
    Router,
};
use config::AppConfig;
use handlers::proxy::{handle_chat_completion, AppState};
use reqwest::Client;
use services::sanitizer::SanitizerService;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_guard_gateway=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let port = config.port;

    info!("==================================================================");
    info!("🛡️  STARTING AI GUARD GATEWAY (Rust 🦀 High-Performance Edition)");
    info!("==================================================================");
    info!("📍 Listening Port       : {}", port);
    info!("🧠 LLM Provider         : {}", config.llm_provider);
    info!("🔗 LLM Target Base URL  : {}", config.llm_base_url);
    info!("🛡️  Prompt Sanitizer     : {}", config.enable_sanitizer);
    info!("✂️  Smart Truncator     : {}", config.enable_truncator);
    info!("🔐 Scope Header Name    : {}", config.scope_header);
    info!("==================================================================");

    let state = Arc::new(AppState {
        config,
        http_client: Client::builder().build().expect("Failed to build reqwest client"),
        sanitizer: SanitizerService::new(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/v1/chat/completions", post(handle_chat_completion))
        .route("/chat/completions", post(handle_chat_completion))
        .route("/v1/models", get(handle_models))
        .route("/models", get(handle_models))
        .fallback(fallback_handler)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "🛡️ AI Guard Gateway is healthy and running! 🦀"
}

async fn handle_models() -> &'static str {
    r#"{"object":"list","data":[{"id":"gemini-3.6-flash","object":"model","created":1700000000,"owned_by":"google"}]}"#
}

async fn fallback_handler(uri: axum::http::Uri) -> (axum::http::StatusCode, String) {
    tracing::warn!("⚠️ Unhandled Route Requested: {}", uri);
    (axum::http::StatusCode::NOT_FOUND, format!("AI Guard Gateway: Route '{}' Not Found", uri))
}
