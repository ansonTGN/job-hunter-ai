pub mod dto;
pub mod error;
pub mod handlers;
pub mod openapi;
pub mod ws;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

pub use handlers::{AppState, AppStateConfig};

pub fn router(state: Arc<AppState>) -> Router {
    // Importante:
    // - /assets/* -> {web_dir}/assets/*
    // - / y fallback -> index() (lee {web_dir}/index.html)
    let web_assets_dir = state.web_assets_dir();

    Router::new()
        // -------------------------
        // V1 (API estable / tipada)
        // -------------------------
        .route("/api/v1/health", get(handlers::health_v1))
        .route("/api/v1/search/start", post(handlers::start_search_v1))
        .route("/api/v1/cv/extract", post(handlers::extract_cv_v1))
        .route("/api/v1/models/ollama", get(handlers::ollama_models_v1))
        .route("/api/v1/models/cloud", post(handlers::cloud_models_v1))
        // OpenAPI
        .route("/api-docs/openapi.json", get(openapi::openapi_json))
        .route("/docs", get(openapi::docs_page))
        // -------------------------
        // Legacy (compatibilidad UI actual)
        // -------------------------
        .route("/api/start", post(handlers::start_search_legacy))
        .route("/api/upload-cv", post(handlers::upload_cv_legacy))
        .route("/api/ollama/models", post(handlers::ollama_models_legacy))
        .route("/api/llm/models", post(handlers::cloud_models_legacy))
        // Alias ping (para tooling)
        .route("/api/ping", get(handlers::health_v1))
        // WS (contrato tipado)
        .route("/ws", get(ws::ws_handler))
        // Static assets
        .nest_service("/assets", ServeDir::new(web_assets_dir))
        // Index + SPA fallback
        .route("/", get(handlers::index))
        .fallback(get(handlers::index))
        .with_state(state)
}
