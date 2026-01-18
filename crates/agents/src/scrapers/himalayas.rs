use async_trait::async_trait;
use chrono::Utc;
use job_hunter_core::*;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info; // CORREGIDO: Eliminado 'warn'
use uuid::Uuid;

pub struct HimalayasAgent;

impl HimalayasAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for HimalayasAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::StartScraping(criteria) = msg {
            // 1. Verificar configuración
            let default_cfg = SourceSettings {
                source: JobSource::Himalayas,
                enabled: true,
                delay_ms: 1200,
                user_agent: "Mozilla/5.0".to_string(),
                use_proxy: false,
            };

            let my_cfg = criteria
                .sources_config
                .iter()
                .find(|s| s.source == JobSource::Himalayas)
                .unwrap_or(&default_cfg);

            if !my_cfg.enabled {
                info!("ℹ️ [Himalayas] Agente desactivado por configuración.");
                return Ok(AgentMessage::RawJobsScraped(vec![]));
            }

            if my_cfg.delay_ms > 0 {
                info!("⏳ [Himalayas] Esperando {}ms (Anti-bot)...", my_cfg.delay_ms);
                sleep(Duration::from_millis(my_cfg.delay_ms)).await;
            }

            let client = reqwest::Client::builder()
                .user_agent(&my_cfg.user_agent)
                .timeout(Duration::from_secs(30))
                .build()?;

            let res = client
                .get("https://himalayas.app/jobs/api?limit=10")
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            let mut postings = Vec::new();
            if let Some(jobs) = res["jobs"].as_array() {
                for job in jobs {
                    postings.push(RawJobPosting {
                        id: Uuid::new_v4().to_string(),
                        source: JobSource::Himalayas,
                        url: job["url"].as_str().unwrap_or_default().to_string(),
                        html_content: job.to_string(),
                        scraped_at: Utc::now(),
                    });
                }
            }
            
            info!("📂 [Himalayas] Éxito: {} ofertas extraídas.", postings.len());
            Ok(AgentMessage::RawJobsScraped(postings))
        } else {
            Err(AgentError::Scraping("Msg inválido".into()))
        }
    }
    fn name(&self) -> &str {
        "scraper_himalayas"
    }
}
