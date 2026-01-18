use async_trait::async_trait;
use chrono::Utc;
use job_hunter_core::*;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info; // CORREGIDO: Eliminado 'warn'
use uuid::Uuid;

pub struct WwrAgent;

impl WwrAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for WwrAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::StartScraping(criteria) = msg {
            // 1. Verificar configuración
            let default_cfg = SourceSettings {
                source: JobSource::WeWorkRemotely,
                enabled: true,
                delay_ms: 1200,
                user_agent: "Mozilla/5.0".to_string(),
                use_proxy: false,
            };

            let my_cfg = criteria
                .sources_config
                .iter()
                .find(|s| s.source == JobSource::WeWorkRemotely)
                .unwrap_or(&default_cfg);

            if !my_cfg.enabled {
                info!("ℹ️ [WWR] Agente desactivado por configuración.");
                return Ok(AgentMessage::RawJobsScraped(vec![]));
            }

            if my_cfg.delay_ms > 0 {
                info!("⏳ [WWR] Esperando {}ms (Anti-bot)...", my_cfg.delay_ms);
                sleep(Duration::from_millis(my_cfg.delay_ms)).await;
            }

            let client = reqwest::Client::builder()
                .user_agent(&my_cfg.user_agent)
                .timeout(Duration::from_secs(30))
                .build()?;

            let res = client
                .get("https://weworkremotely.com/remote-jobs.rss")
                .send()
                .await?
                .text()
                .await?;

            let mut postings = Vec::new();
            // Parsing simple de RSS para la demo
            for cap in res.split("<item>").skip(1).take(10) {
                if let Some(url) = cap
                    .split("<link>")
                    .nth(1)
                    .and_then(|s| s.split("</link>").next())
                {
                    postings.push(RawJobPosting {
                        id: Uuid::new_v4().to_string(),
                        source: JobSource::WeWorkRemotely,
                        url: url.trim().to_string(),
                        html_content: cap.to_string(),
                        scraped_at: Utc::now(),
                    });
                }
            }
            
            info!("📂 [WWR] Éxito: {} ofertas extraídas.", postings.len());
            Ok(AgentMessage::RawJobsScraped(postings))
        } else {
            Err(AgentError::Scraping("Msg inválido".into()))
        }
    }
    fn name(&self) -> &str {
        "scraper_wwr"
    }
}
