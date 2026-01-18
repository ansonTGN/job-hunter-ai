use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use job_hunter_core::{Agent, AgentMessage, AnalyzedJobPosting, SearchCriteria};

pub struct Orchestrator {
    agents: HashMap<String, Arc<dyn Agent>>,
    message_tx: mpsc::Sender<(String, AgentMessage)>,
    message_rx: mpsc::Receiver<(String, AgentMessage)>,
    result_tx: mpsc::Sender<Vec<AnalyzedJobPosting>>,
    current_criteria: Option<Arc<SearchCriteria>>,
}

impl Orchestrator {
    pub fn new() -> (Self, mpsc::Receiver<Vec<AnalyzedJobPosting>>) {
        let (message_tx, message_rx) = mpsc::channel(100);
        let (result_tx, result_rx) = mpsc::channel(10);

        (
            Self {
                agents: HashMap::new(),
                message_tx,
                message_rx,
                result_tx,
                current_criteria: None,
            },
            result_rx,
        )
    }

    pub fn register_agent(&mut self, agent: Arc<dyn Agent>) {
        info!("📝 Registrando agente: {}", agent.name());
        self.agents.insert(agent.name().to_string(), agent);
    }

    pub async fn start_search(&mut self, criteria: SearchCriteria) -> anyhow::Result<()> {
        let criteria_arc = Arc::new(criteria);
        self.current_criteria = Some(criteria_arc.clone());

        let scrapers: Vec<String> = self
            .agents
            .keys()
            .filter(|k| k.starts_with("scraper_"))
            .cloned()
            .collect();

        if scrapers.is_empty() {
            warn!("⚠️ No hay scrapers registrados. La búsqueda no hará nada.");
            // Emitimos fin “vacío” para que la UI no quede esperando eternamente.
            let _ = self.result_tx.send(vec![]).await;
            let _ = self
                .message_tx
                .send(("__orchestrator__".into(), AgentMessage::Shutdown))
                .await;
            return Ok(());
        }

        for name in scrapers {
            info!("🛰️ Activando scraper: {}", name);
            let _ = self
                .message_tx
                .send((name, AgentMessage::StartScraping(criteria_arc.clone())))
                .await;
        }

        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("🚀 Orquestador iniciado y esperando mensajes...");

        while let Some((target, msg)) = self.message_rx.recv().await {
            if let AgentMessage::Shutdown = msg {
                break;
            }

            if let Some(agent) = self.agents.get(&target) {
                let agent = agent.clone();
                let tx = self.message_tx.clone();
                let res_tx = self.result_tx.clone();
                let criteria_ref = self.current_criteria.clone();

                tokio::spawn(async move {
                    debug!("📬 Procesando mensaje para agente: {}", agent.name());
                    match agent.process(msg).await {
                        Ok(response) => {
                            Self::route(response, tx, res_tx, criteria_ref).await;
                        }
                        Err(e) => error!("❌ Error en agente {}: {}", agent.name(), e),
                    }
                });
            } else {
                warn!("❓ Mensaje dirigido a agente desconocido: {}", target);
            }
        }

        Ok(())
    }

    async fn route(
        msg: AgentMessage,
        tx: mpsc::Sender<(String, AgentMessage)>,
        res_tx: mpsc::Sender<Vec<AnalyzedJobPosting>>,
        criteria: Option<Arc<SearchCriteria>>,
    ) {
        match msg {
            AgentMessage::RawJobsScraped(jobs) => {
                info!("📡 Scraper finalizado: {} ofertas encontradas.", jobs.len());

                // --- CORRECCIÓN CRÍTICA ---
                // Si jobs está vacío (scraper desactivado o sin resultados),
                // NO enviamos Shutdown. Simplemente retornamos para permitir
                // que otros scrapers sigan procesando y enviando sus ofertas.
                if jobs.is_empty() {
                    warn!("⚠️ Un scraper finalizó con 0 resultados (o estaba desactivado). Continuando...");
                    return; 
                }
                // --------------------------

                if let Some(c) = criteria {
                    info!("🧠 Enviando {} ofertas al Analyzer (IA)...", jobs.len());
                    let _ = tx
                        .send(("analyzer".into(), AgentMessage::AnalyzeJobs(jobs, c)))
                        .await;
                } else {
                    warn!("⚠️ No hay criteria en memoria. Cerrando run.");
                    let _ = res_tx.send(vec![]).await;
                    let _ = tx
                        .send(("__orchestrator__".into(), AgentMessage::Shutdown))
                        .await;
                }
            }
            AgentMessage::JobsAnalyzed(jobs) => {
                info!(
                    "✨ Análisis de IA completado para {} ofertas. Enriqueciendo...",
                    jobs.len()
                );
                let _ = tx
                    .send(("enricher".into(), AgentMessage::JobsAnalyzed(jobs)))
                    .await;
            }
            AgentMessage::JobsEnriched(jobs) => {
                info!(
                    "✅ Proceso completado. Enviando {} resultados a la UI.",
                    jobs.len()
                );
                let _ = res_tx.send(jobs).await;
                // Aquí SÍ cerramos, porque el enriquecedor es el último paso
                // de un lote de ofertas exitoso.
                let _ = tx
                    .send(("__orchestrator__".into(), AgentMessage::Shutdown))
                    .await;
            }
            AgentMessage::Error(e) => {
                error!("❌ Error recibido en ruta: {}", e);
                // En caso de error explícito, quizás sí queramos cerrar o solo loguear.
                // Dejamos shutdown por seguridad ante fallos fatales.
                let _ = res_tx.send(vec![]).await;
                let _ = tx
                    .send(("__orchestrator__".into(), AgentMessage::Shutdown))
                    .await;
            }
            _ => {}
        }
    }
}
