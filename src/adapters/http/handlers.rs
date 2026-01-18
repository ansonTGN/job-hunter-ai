use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

// Importamos la fábrica de scrapers y los agentes necesarios
use job_hunter_agents::{
    get_all_scrapers, AnalyzerAgent, EnricherAgent, UseCase
};
use job_hunter_core::*;
use job_hunter_orchestrator::Orchestrator;

use super::{
    dto::*,
    error::ApiError,
    ws::{send_log, WsEvent},
};

pub struct AppStateConfig {
    pub web_dir: String,
}

pub struct AppState {
    pub ws_tx: broadcast::Sender<String>,
    pub http_client: reqwest::Client,
    pub web_dir: String,
}

impl AppState {
    pub fn new(cfg: AppStateConfig) -> anyhow::Result<Self> {
        let (ws_tx, _) = broadcast::channel(256);
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()?;

        Ok(Self {
            ws_tx,
            http_client,
            web_dir: cfg.web_dir,
        })
    }

    pub fn web_assets_dir(&self) -> String {
        format!("{}/assets", self.web_dir.trim_end_matches('/'))
    }

    fn index_path(&self) -> String {
        format!("{}/index.html", self.web_dir.trim_end_matches('/'))
    }
}

//
// UI index
//

pub async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match tokio::fs::read_to_string(state.index_path()).await {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("index.html no encontrado o error leyendo: {}", e),
        )
            .into_response(),
    }
}

//
// Health (v1)
//

pub async fn health_v1() -> Result<Json<ApiOk<HealthResponse>>, ApiError> {
    Ok(Json(ApiOk {
        ok: true,
        data: HealthResponse {
            status: "up".to_string(),
        },
    }))
}

//
// Start search (V1 tipado)
//

pub async fn start_search_v1(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartSearchRequestV1>,
) -> Result<Json<ApiOk<StartSearchResponseV1>>, ApiError> {
    if req.criteria.keywords.is_empty() {
        return Err(ApiError::bad_request(
            "validation_error",
            "criteria.keywords no puede estar vacío",
        ));
    }

    let run_id = Uuid::new_v4();

    // Lanzamos ejecución asíncrona
    let state_bg = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_search_from_v1(state_bg, req, run_id).await {
            error!("Error run_search_from_v1: {:#}", e);
        }
    });

    Ok(Json(ApiOk {
        ok: true,
        data: StartSearchResponseV1 { run_id },
    }))
}

async fn run_search_from_v1(
    state: Arc<AppState>,
    req: StartSearchRequestV1,
    run_id: Uuid,
) -> anyhow::Result<()> {
    let (mut orch, mut result_rx) = Orchestrator::new();

    // 1. REGISTRO AUTOMÁTICO DE SCRAPERS
    // Usamos la fábrica centralizada para cargar todos los scrapers disponibles
    let all_scrapers = get_all_scrapers();
    for agent in all_scrapers {
        orch.register_agent(agent);
    }

    // 2. Agentes de IA y Enriquecimiento
    let analyzer = build_analyzer_agent(&req.llm, state.ws_tx.clone())?;
    let enricher = Arc::new(EnricherAgent::new());

    orch.register_agent(analyzer);
    orch.register_agent(enricher);

    // 3. Mapeo de Criteria
    let criteria = SearchCriteria {
        keywords: req.criteria.keywords.clone(),
        experience_level: map_experience(req.criteria.experience_level.clone()),
        sources_config: req
            .criteria
            .sources_config
            .iter()
            .map(|s| SourceSettings {
                source: map_source(s.source.clone()),
                enabled: s.enabled,
                delay_ms: s.delay_ms,
                user_agent: s.user_agent.clone(),
                use_proxy: s.use_proxy,
            })
            .collect(),
        user_cv: req.criteria.user_cv.clone(),
    };

    send_log(
        &state,
        "info",
        format!("run_id={} arrancando búsqueda", run_id),
    );

    // 4. Ejecución del Orquestador
    orch.start_search(criteria).await?;

    // run() consume self, así que lo ejecutamos en background
    let run_task = tokio::spawn(async move { orch.run().await });

    // 5. Recogida de resultados
    if let Some(results) = result_rx.recv().await {
        send_log(
            &state,
            "info",
            format!("run_id={} finalizado. {} resultados", run_id, results.len()),
        );

        for job in results.iter().take(200) {
            let _ = state
                .ws_tx
                .send(serde_json::to_string(&WsEvent::JobFound(job.clone())).unwrap_or_default());
        }
    } else {
        send_log(
            &state,
            "warn",
            format!("run_id={} finalizado sin resultados (o timeout)", run_id),
        );
    }

    // Asegura que el task del orquestador finalice
    let _ = run_task.await;

    Ok(())
}

fn build_analyzer_agent(
    llm: &LlmConfigV1,
    ws_tx: broadcast::Sender<String>,
) -> anyhow::Result<Arc<AnalyzerAgent>> {
    let use_case = match llm.use_case {
        ApiUseCase::Fast => UseCase::Fast,
        ApiUseCase::Balanced => UseCase::Balanced,
        ApiUseCase::Deep => UseCase::Deep,
        ApiUseCase::LongContext => UseCase::LongContext,
    };

    let agent = match llm.provider {
        ApiLlmProvider::Local => {
            let endpoint = llm
                .local
                .as_ref()
                .map(|l| l.endpoint.clone())
                .unwrap_or_else(|| "http://localhost:11434".to_string());

            let model = llm
                .local
                .as_ref()
                .map(|l| l.model.clone())
                .unwrap_or_else(|| "llama3.2:3b".to_string());

            AnalyzerAgent::new_local(endpoint, model)
        }
        ApiLlmProvider::Openai => {
            let cloud = llm.cloud.as_ref().ok_or_else(|| {
                anyhow::anyhow!("provider=openai requiere llm.cloud (api_key, base_url?, model?)")
            })?;

            let api_key = cloud.api_key.trim().to_string();
            if api_key.is_empty() {
                anyhow::bail!("OpenAI api_key vacío");
            }

            let base_url = cloud
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com".to_string());

            AnalyzerAgent::new_openai(api_key, base_url, cloud.model.clone(), use_case)
        }
        ApiLlmProvider::Anthropic => {
            let cloud = llm.cloud.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "provider=anthropic requiere llm.cloud (api_key, base_url?, model?)"
                )
            })?;

            let api_key = cloud.api_key.trim().to_string();
            if api_key.is_empty() {
                anyhow::bail!("Anthropic api_key vacío");
            }

            let base_url = cloud
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".to_string());

            AnalyzerAgent::new_anthropic(api_key, base_url, cloud.model.clone(), use_case)
        }
    };

    Ok(Arc::new(agent.with_ws_tx(ws_tx)))
}

fn map_experience(level: ApiExperienceLevel) -> ExperienceLevel {
    match level {
        ApiExperienceLevel::Entry => ExperienceLevel::Entry,
        ApiExperienceLevel::Junior => ExperienceLevel::Junior,
        ApiExperienceLevel::Mid => ExperienceLevel::Mid,
        ApiExperienceLevel::Senior => ExperienceLevel::Senior,
        ApiExperienceLevel::Lead => ExperienceLevel::Lead,
        ApiExperienceLevel::Any => ExperienceLevel::Any,
    }
}

// Mapeo exhaustivo de fuentes para incluir los nuevos scrapers
fn map_source(s: ApiJobSource) -> JobSource {
    match s {
        // Clásicos
        ApiJobSource::Remoteok => JobSource::RemoteOk,
        ApiJobSource::Arbeitnow => JobSource::Arbeitnow,
        ApiJobSource::Himalayas => JobSource::Himalayas,
        ApiJobSource::Wwr => JobSource::WeWorkRemotely,
        ApiJobSource::Jobspresso => JobSource::Jobspresso,
        // Nuevos
        ApiJobSource::Remotive => JobSource::Remotive,
        ApiJobSource::Jobicy => JobSource::Jobicy,
        ApiJobSource::FindWork => JobSource::FindWork,
        ApiJobSource::WorkingNomads => JobSource::WorkingNomads,
        ApiJobSource::VueJobs => JobSource::VueJobs,
        ApiJobSource::CryptoJobs => JobSource::CryptoJobs,
        ApiJobSource::RemoteCo => JobSource::RemoteCo,
        ApiJobSource::DevItJobs => JobSource::DevItJobs,
        ApiJobSource::PythonOrg => JobSource::PythonOrg,
        ApiJobSource::GolangProjects => JobSource::GolangProjects,
    }
}

//
// Extract CV (V1 tipado + IA)
//

pub async fn extract_cv_v1(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiOk<CvExtractResponseV1>>, ApiError> {
    let mut file_text = String::new();
    let mut provider = "local".to_string();
    let mut model = "llama3".to_string();
    let mut endpoint = "http://localhost:11434".to_string();
    let mut api_key = "".to_string();
    let mut base_url = "".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let file_name = field.file_name().unwrap_or("").to_string();
            if let Ok(bytes) = field.bytes().await {
                if file_name.to_lowercase().ends_with(".pdf") {
                    info!("Procesando PDF: {}", file_name);
                    
                    // --- Extracción Robusta PDF ---
                    match lopdf::Document::load_mem(&bytes) {
                        Ok(doc) => {
                            let pages = doc.get_pages();
                            let mut page_numbers: Vec<_> = pages.keys().collect();
                            page_numbers.sort();

                            for &page_id in page_numbers {
                                if let Ok(t) = doc.extract_text(&[page_id]) {
                                    if !t.trim().is_empty() {
                                        file_text.push_str(&t);
                                        file_text.push('\n');
                                    }
                                }
                            }
                            
                            // Si lopdf extrae cadenas vacías (PDF escaneado/protegido)
                            if file_text.trim().is_empty() {
                                warn!("PDF cargado pero sin texto estructurado. Intentando modo raw...");
                                // Fallback a buscar strings crudos
                                for object in doc.objects.values() {
                                    if let lopdf::Object::String(ref bytes, _) = object {
                                        let s = String::from_utf8_lossy(bytes);
                                        if !s.trim().is_empty() {
                                            file_text.push_str(&s);
                                            file_text.push(' ');
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error al parsear PDF con lopdf: {:?}", e);
                            // Último recurso: tratar bytes como texto
                            if file_text.is_empty() {
                                file_text = String::from_utf8_lossy(&bytes).to_string();
                            }
                        }
                    }
                } else {
                    // Texto plano o similar
                    file_text = String::from_utf8_lossy(&bytes).to_string();
                }
            }
        } else {
            // Leer configuración del LLM del formulario
            if let Ok(val) = field.text().await {
                match name.as_str() {
                    "llm_provider" => provider = val,
                    "model" => model = val,
                    "endpoint" => endpoint = val,
                    "api_key" => api_key = val,
                    "base_url" => base_url = val,
                    _ => {}
                }
            }
        }
    }

    // --- MANEJO DE TEXTO VACÍO (NO RETORNAR ERROR 400) ---
    if file_text.trim().is_empty() {
        warn!("No se pudo extraer texto. El archivo puede estar vacío o ser una imagen. Devolviendo placeholder.");
        file_text = "No se pudo extraer texto automáticamente. Por favor copia y pega tu CV aquí.".to_string();
    }

    // Construir agente temporal con la config recibida para extraer skills
    let agent = match provider.as_str() {
        "openai" => {
            let url = if base_url.is_empty() { "https://api.openai.com".to_string() } else { base_url };
            AnalyzerAgent::new_openai(api_key, url, Some(model), UseCase::Balanced)
        }
        "anthropic" => {
            let url = if base_url.is_empty() { "https://api.anthropic.com".to_string() } else { base_url };
            AnalyzerAgent::new_anthropic(api_key, url, Some(model), UseCase::Balanced)
        }
        _ => AnalyzerAgent::new_local(endpoint, model),
    };

    let agent_arc = agent.with_ws_tx(state.ws_tx.clone());

    // IA: Extraer keywords
    let keywords = match agent_arc.extract_keywords_from_cv(&file_text).await {
        Ok(kws) => kws,
        Err(e) => {
            warn!("Fallo extracción keywords LLM: {}", e);
            send_log(
                &state,
                "warn",
                format!("No se pudieron extraer keywords con IA: {}", e),
            );
            vec![]
        }
    };

    Ok(Json(ApiOk {
        ok: true,
        data: CvExtractResponseV1 {
            text: file_text,
            keywords,
        },
    }))
}

//
// Ollama models (V1 tipado) - GET
//

pub async fn ollama_models_v1(
    Query(q): Query<OllamaModelsQueryV1>,
) -> Result<Json<ApiOk<OllamaModelsResponseV1>>, ApiError> {
    let endpoint = q
        .endpoint
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    let res = reqwest::get(&url).await.map_err(|e| {
        ApiError::bad_request(
            "ollama_network_error",
            format!("No se pudo conectar con Ollama: {}", e),
        )
    })?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(ApiError::bad_request(
            "ollama_http_error",
            format!(
                "Ollama HTTP {}: {}",
                status,
                body.chars().take(220).collect::<String>()
            ),
        ));
    }

    let v: serde_json::Value = res.json().await.unwrap_or(serde_json::json!({}));
    let models: Vec<OllamaModelTag> = v
        .get("models")
        .and_then(|x| x.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .map(|name| OllamaModelTag { name })
        .collect();

    Ok(Json(ApiOk {
        ok: true,
        data: OllamaModelsResponseV1 { endpoint, models },
    }))
}

//
// Cloud models (V1 tipado) - POST
//

pub async fn cloud_models_v1(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CloudModelsRequestV1>,
) -> Result<Json<ApiOk<CloudModelsResponseV1>>, ApiError> {
    let provider_enum = req.provider.clone();

    let provider = match req.provider {
        ApiLlmProvider::Openai => "openai",
        ApiLlmProvider::Anthropic => "anthropic",
        ApiLlmProvider::Local => {
            return Err(ApiError::bad_request(
                "validation_error",
                "provider inválido (openai|anthropic)",
            ));
        }
    };

    let api_key = req.api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(ApiError::bad_request("validation_error", "api_key vacío"));
    }

    let base_url = req.base_url.unwrap_or_else(|| match provider {
        "openai" => "https://api.openai.com".to_string(),
        "anthropic" => "https://api.anthropic.com".to_string(),
        _ => "".to_string(),
    });

    let url = match provider {
        "openai" => format!("{}/v1/models", base_url.trim_end_matches('/')),
        "anthropic" => format!("{}/v1/models", base_url.trim_end_matches('/')),
        _ => {
            return Err(ApiError::bad_request(
                "validation_error",
                "provider inválido (openai|anthropic)",
            ));
        }
    };

    let res = match provider {
        "openai" => state
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| ApiError::bad_request("network_error", format!("Network error: {}", e)))?,
        "anthropic" => state
            .http_client
            .get(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| ApiError::bad_request("network_error", format!("Network error: {}", e)))?,
        _ => unreachable!(),
    };

    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ApiError::bad_request(
            "http_error",
            format!(
                "{} HTTP {}: {}",
                provider,
                status,
                body.chars().take(220).collect::<String>()
            ),
        ));
    }

    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
    let models: Vec<String> = v
        .get("data")
        .and_then(|x| x.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            m.get("id")
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    Ok(Json(ApiOk {
        ok: true,
        data: CloudModelsResponseV1 {
            provider: provider_enum,
            base_url,
            models,
        },
    }))
}

//
// -------------------------
// Legacy handlers (compat UI actual)
// -------------------------
//

pub async fn start_search_legacy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let tx = state.ws_tx.clone();

    tokio::spawn(async move {
        let (mut orch, mut result_rx) = Orchestrator::new();

        // Carga automática de TODOS los scrapers (Legacy también se beneficia)
        let all_scrapers = get_all_scrapers();
        for agent in all_scrapers {
            orch.register_agent(agent);
        }

        // LLM config (legacy)
        let llm = req.get("llm").cloned().unwrap_or(serde_json::json!({}));
        let provider = llm
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("local");
        let model = llm
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let endpoint = llm
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("http://localhost:11434")
            .to_string();
        let base_url = llm
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com")
            .to_string();
        let api_key = llm
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let use_case = llm
            .get("use_case")
            .and_then(|v| v.as_str())
            .unwrap_or("balanced")
            .to_string();

        let analyzer = match provider {
            "openai" => {
                AnalyzerAgent::new_openai(api_key, base_url, model, UseCase::from_str(&use_case))
            }
            "anthropic" => {
                let base_url = llm
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://api.anthropic.com")
                    .to_string();
                AnalyzerAgent::new_anthropic(api_key, base_url, model, UseCase::from_str(&use_case))
            }
            _ => {
                let model = model.unwrap_or_else(|| "llama3.2:3b".to_string());
                AnalyzerAgent::new_local(endpoint, model)
            }
        }
        .with_ws_tx(tx.clone());

        orch.register_agent(Arc::new(analyzer));
        orch.register_agent(Arc::new(EnricherAgent::new()));

        // Criteria (legacy)
        let criteria = req
            .get("criteria")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let keywords: Vec<String> = criteria
            .get("keywords")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect();

        let experience = criteria
            .get("experience_level")
            .and_then(|v| v.as_str())
            .unwrap_or("any");

        let experience_level = match experience {
            "entry" => ExperienceLevel::Entry,
            "junior" => ExperienceLevel::Junior,
            "mid" => ExperienceLevel::Mid,
            "senior" => ExperienceLevel::Senior,
            "lead" => ExperienceLevel::Lead,
            _ => ExperienceLevel::Any,
        };

        // Note: Legacy source config parsing is simplified here.
        // It will only enable classic sources unless updated explicitly.
        // For new sources, users should use V1 API.
        let sources_config: Vec<SourceSettings> = criteria
            .get("sources_config")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|s| {
                let source = s.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let enabled = s.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let delay_ms = s.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(1200);
                let user_agent = s
                    .get("user_agent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Mozilla/5.0")
                    .to_string();
                let use_proxy = s
                    .get("use_proxy")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let source = match source {
                    "remote_ok" => JobSource::RemoteOk,
                    "arbeitnow" => JobSource::Arbeitnow,
                    "himalayas" => JobSource::Himalayas,
                    "wwr" => JobSource::WeWorkRemotely,
                    "jobspresso" => JobSource::Jobspresso,
                     // Legacy mapping fallback
                    _ => return None,
                };

                Some(SourceSettings {
                    source,
                    enabled,
                    delay_ms,
                    user_agent,
                    use_proxy,
                })
            })
            .collect();

        let user_cv = criteria
            .get("user_cv")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let criteria = SearchCriteria {
            keywords,
            experience_level,
            sources_config,
            user_cv,
        };

        let _ = tx.send(serde_json::json!({"type":"status","payload":"starting"}).to_string());

        // API nueva del orquestador
        if let Err(e) = orch.start_search(criteria).await {
            let _ = tx.send(
                serde_json::json!({"type":"status","payload":format!("error: {}", e)}).to_string(),
            );
            return;
        }

        let run_task = tokio::spawn(async move { orch.run().await });

        if let Some(results) = result_rx.recv().await {
            let _ = tx.send(serde_json::json!({"type":"status","payload":"done"}).to_string());
            for job in results.iter().take(200) {
                let _ = tx.send(serde_json::json!({"type":"job_found","payload":job}).to_string());
            }
        } else {
            let _ =
                tx.send(serde_json::json!({"type":"status","payload":"done_empty"}).to_string());
        }

        let _ = run_task.await;
    });

    Json(serde_json::json!({"ok": true}))
}

pub async fn ollama_models_legacy(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let endpoint = req
        .get("endpoint")
        .and_then(|v| v.as_str())
        .unwrap_or("http://localhost:11434")
        .to_string();

    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    match reqwest::get(&url).await {
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            if !status.is_success() {
                return Json(
                    serde_json::json!({"models": [], "error": format!("Ollama HTTP {}: {}", status, body.chars().take(220).collect::<String>())}),
                );
            }
            let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
            let models: Vec<String> = v
                .get("models")
                .and_then(|x| x.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            Json(serde_json::json!({"models": models}))
        }
        Err(e) => Json(serde_json::json!({"models": [], "error": format!("Network error: {}", e)})),
    }
}

pub async fn cloud_models_legacy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let provider = req.get("provider").and_then(|v| v.as_str()).unwrap_or("");
    let api_key = req
        .get("api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if api_key.is_empty() {
        return Json(serde_json::json!({"models": [], "error": "Falta api_key"}));
    }

    match provider {
        "openai" => {
            let base_url = req
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.openai.com");
            let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

            match state
                .http_client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
            {
                Ok(res) => {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    if !status.is_success() {
                        return Json(
                            serde_json::json!({"models": [], "error": format!("OpenAI HTTP {}: {}", status, body.chars().take(220).collect::<String>())}),
                        );
                    }
                    let v: serde_json::Value =
                        serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
                    let models: Vec<String> = v
                        .get("data")
                        .and_then(|x| x.as_array())
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|m| {
                            m.get("id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                    Json(serde_json::json!({"models": models}))
                }
                Err(e) => Json(
                    serde_json::json!({"models": [], "error": format!("Network error: {}", e)}),
                ),
            }
        }
        "anthropic" => {
            let base_url = req
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.anthropic.com");
            let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

            match state
                .http_client
                .get(&url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
            {
                Ok(res) => {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    if !status.is_success() {
                        return Json(
                            serde_json::json!({"models": [], "error": format!("Anthropic HTTP {}: {}", status, body.chars().take(220).collect::<String>())}),
                        );
                    }
                    let v: serde_json::Value =
                        serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
                    let models: Vec<String> = v
                        .get("data")
                        .and_then(|x| x.as_array())
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|m| {
                            m.get("id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                    Json(serde_json::json!({"models": models}))
                }
                Err(e) => Json(
                    serde_json::json!({"models": [], "error": format!("Network error: {}", e)}),
                ),
            }
        }
        _ => Json(serde_json::json!({"models": [], "error": "provider inválido"})),
    }
}

pub async fn upload_cv_legacy(mut multipart: Multipart) -> Response {
    let mut extracted_text = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("").to_string();
        if let Ok(bytes) = field.bytes().await {
            if file_name.to_lowercase().ends_with(".pdf") {
                if let Ok(doc) = lopdf::Document::load_mem(&bytes) {
                    for page_num in doc.get_pages().keys() {
                        if let Ok(text) = doc.extract_text(&[*page_num]) {
                            extracted_text.push_str(&text);
                            extracted_text.push('\n');
                        }
                    }
                }
            } else {
                extracted_text = String::from_utf8_lossy(&bytes).to_string();
            }
        }
    }

    Json(serde_json::json!({ "text": extracted_text })).into_response()
}