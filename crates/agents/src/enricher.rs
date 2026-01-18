use async_trait::async_trait;
use job_hunter_core::*;
use reqwest::Client;
use tracing::info;

pub struct EnricherAgent {
    _client: Client, // El guion bajo silencia el warning de "unused field"
}

impl EnricherAgent {
    pub fn new() -> Self {
        Self {
            _client: Client::new(),
        }
    }
}

#[async_trait]
impl Agent for EnricherAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::JobsAnalyzed(mut jobs) = msg {
            info!("Enriqueciendo {} ofertas...", jobs.len());
            for job in &mut jobs {
                if let Some(ref mut company) = job.company {
                    // Simulación de enriquecimiento
                    company.website = Some(format!(
                        "https://www.{}.com",
                        company.name.to_lowercase().replace(" ", "")
                    ));
                }
            }
            Ok(AgentMessage::JobsEnriched(jobs))
        } else {
            Err(AgentError::Enrichment("Msg esperado".into()))
        }
    }
    fn name(&self) -> &str {
        "enricher"
    }
}
