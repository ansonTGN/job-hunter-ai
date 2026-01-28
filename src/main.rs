mod adapters;

use std::{net::SocketAddr, sync::Arc};

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use adapters::http::{AppState, AppStateConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configuración de logs
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "job_hunter=info,job_hunter_orchestrator=info,job_hunter_agents=info,axum=info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 1. Configuración de Host
    // En Render debemos escuchar en 0.0.0.0
    let bind = std::env::var("JOB_HUNTER_BIND").unwrap_or_else(|_| "0.0.0.0".to_string());

    // 2. Configuración de Puerto (CRÍTICO PARA RENDER)
    // Render pasa el puerto en la variable "PORT". 
    // Prioridad: JOB_HUNTER_PORT -> PORT -> 3000
    let port: u16 = std::env::var("JOB_HUNTER_PORT")
        .or_else(|_| std::env::var("PORT")) 
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    // 3. Directorio Web
    let web_dir = std::env::var("JOB_HUNTER_WEB_DIR").unwrap_or_else(|_| "web".to_string());

    let cfg = AppStateConfig { web_dir };
    let state = Arc::new(AppState::new(cfg)?);

    let app = adapters::http::router(state);

    let addr: SocketAddr = format!("{}:{}", bind, port).parse()?;
    info!("🚀 Servidor iniciado en http://{}", addr);
    info!("💡 UI: / | Assets: /assets/* | WS: /ws | Docs: /docs | OpenAPI: /api-docs/openapi.json");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

