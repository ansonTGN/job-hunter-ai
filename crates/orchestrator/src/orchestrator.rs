use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use std::panic::AssertUnwindSafe;
use futures::FutureExt; 

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
        info!("🚀 Orquestador (con Supervisión) iniciado...");

        while let Some((target, msg)) = self.message_rx.recv().await {
            
            if let AgentMessage::Shutdown = msg {
                break;
            }

            if let Some(agent) = self.agents.get(&target) {
                let agent = agent.clone();
                let tx = self.message_tx.clone();
                let res_tx = self.result_tx.clone();
                let criteria_ref = self.current_criteria.clone();

                // --- SUPERVISOR TASK WRAPPER ---
                tokio::spawn(async move {
                    // Usamos AssertUnwindSafe para capturar pánicos (crashes de Rust)
                    let result = AssertUnwindSafe(async {
                        debug!("🔎 [Supervisor] Ejecutando agente: {}", agent.name());
                        agent.process(msg).await
                    }).catch_unwind().await;

                    match result {
                        // El agente terminó "bien" (Ok o Err controlado)
                        Ok(process_result) => {
                            match process_result {
                                Ok(response) => {
                                    Self::route(response, tx, res_tx, criteria_ref).await;
                                }
                                Err(e) => {
                                    error!("⚠️ [Supervisor] Agente '{}' reportó error: {}", agent.name(), e);
                                    // Notificamos error interno pero no cerramos el sistema completo
                                    let _ = tx.send(("__orchestrator__".into(), AgentMessage::Error(e.to_string()))).await;
                                }
                            }
                        },
                        // El agente entró en PÁNICO (Crash real)
                        Err(panic_cause) => {
                            let cause_str = if let Some(s) = panic_cause.downcast_ref::<&str>() {
                                s.to_string()
                            } else {
                                "Unknown panic".to_string()
                            };
                            error!("🚨 [SUPERVISOR] CRITICAL: Agente '{}' CRASHED! Causa: {}", agent.name(), cause_str);
                            let _ = tx.send(("__orchestrator__".into(), AgentMessage::Error(format!("PANIC: {}", cause_str)))).await;
                        }
                    }
                });
            } else if target == "__orchestrator__" {
                if let AgentMessage::Error(e) = msg {
                    warn!("⚙️ [Orchestrator Logic] Error recibido de subsistema: {}", e);
                }
            } else {
                warn!("❓ Mensaje a agente desconocido: {}", target);
            }
        }
        
        info!("🛑 Orquestador detenido.");
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
                info!("📡 Scraper finalizado. {} ofertas encontradas.", jobs.len());
                // Si jobs viene vacío, NO matamos el flujo, permitimos que otros scrapers sigan.
                if jobs.is_empty() { return; }
                
                if let Some(c) = criteria {
                    let _ = tx.send(("analyzer".into(), AgentMessage::AnalyzeJobs(jobs, c))).await;
                }
            },
            AgentMessage::JobsAnalyzed(jobs) => {
                info!("✨ Análisis completado para {} ofertas.", jobs.len());
                let _ = tx.send(("enricher".into(), AgentMessage::JobsAnalyzed(jobs))).await;
            }
            AgentMessage::JobsEnriched(jobs) => {
                info!("✅ Proceso completado. Enviando {} resultados.", jobs.len());
                let _ = res_tx.send(jobs).await;
                // Señal de finalización del ciclo
                let _ = tx.send(("__orchestrator__".into(), AgentMessage::Shutdown)).await;
            }
            _ => {}
        }
    }
}
