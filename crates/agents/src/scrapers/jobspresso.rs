use async_trait::async_trait;
use chrono::Utc;
use job_hunter_core::*;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

pub struct JobspressoAgent;

impl JobspressoAgent {
    pub fn new() -> Self {
        Self
    }

    // Devuelve (enabled, delay_ms, user_agent, use_proxy)
    fn settings_from_criteria(criteria: &SearchCriteria) -> (bool, u64, String, bool) {
        if let Some(s) = criteria
            .sources_config
            .iter()
            .find(|s| s.source == JobSource::Jobspresso)
        {
            (s.enabled, s.delay_ms, s.user_agent.clone(), s.use_proxy)
        } else {
            // Default
            (true, 1200, "Mozilla/5.0".to_string(), false)
        }
    }

    async fn fetch_html(client: &reqwest::Client, url: &str) -> Result<String, AgentError> {
        let res = client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .await
            .map_err(AgentError::Network)?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AgentError::Scraping(format!(
                "Jobspresso HTTP {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            )));
        }
        res.text().await.map_err(AgentError::Network)
    }

    fn extract_job_links(base: &Url, html: &str, max: usize) -> Vec<String> {
        let doc = Html::parse_document(html);
        let a_sel = Selector::parse("a").unwrap();

        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<String> = Vec::new();

        for a in doc.select(&a_sel) {
            if let Some(href) = a.value().attr("href") {
                if !href.contains("/job/") {
                    continue;
                }
                let abs = match base.join(href) {
                    Ok(u) => u,
                    Err(_) => continue,
                };
                if abs.path().is_empty() {
                    continue;
                }
                let s = abs.to_string();
                if seen.insert(s.clone()) {
                    out.push(s);
                    if out.len() >= max {
                        break;
                    }
                }
            }
        }
        out
    }

    async fn scrape_latest(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<RawJobPosting>, AgentError> {
        let (enabled, delay_ms, user_agent, _use_proxy) = Self::settings_from_criteria(criteria);

        if !enabled {
            info!("ℹ️ [Jobspresso] Agente desactivado por configuración.");
            return Ok(vec![]);
        }

        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(AgentError::Network)?;

        if delay_ms > 0 {
            info!("⏳ [Jobspresso] Esperando {}ms (Anti-bot)...", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;
        }

        let base = Url::parse("https://jobspresso.co/").expect("base url");
        info!("🌐 [Jobspresso] Descargando listados (home, sin JS)...");
        
        let home_html = match Self::fetch_html(&client, base.as_str()).await {
            Ok(h) => h,
            Err(e) => {
                error!("❌ [Jobspresso] Error conectando a home: {}", e);
                return Ok(vec![]);
            }
        };

        let links = Self::extract_job_links(&base, &home_html, 12);
        if links.is_empty() {
            warn!("⚠️ [Jobspresso] No se encontraron links /job/. Posible cambio de markup o bloqueo.");
            return Ok(vec![]);
        }

        let mut postings = Vec::new();
        for (i, job_url) in links.into_iter().enumerate() {
            if i > 0 {
                sleep(Duration::from_millis(300)).await;
            }

            match Self::fetch_html(&client, &job_url).await {
                Ok(detail_html) => postings.push(RawJobPosting {
                    id: Uuid::new_v4().to_string(),
                    source: JobSource::Jobspresso,
                    url: job_url,
                    html_content: detail_html,
                    scraped_at: Utc::now(),
                }),
                Err(e) => {
                    warn!("⚠️ [Jobspresso] Saltando detalle por error: {}", e);
                    continue;
                }
            }

            if postings.len() >= 10 {
                break;
            }
        }

        Ok(postings)
    }
}

#[async_trait]
impl Agent for JobspressoAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        match msg {
            AgentMessage::StartScraping(criteria) => match self.scrape_latest(&criteria).await {
                Ok(postings) => {
                    if !postings.is_empty() {
                        info!("📂 [Jobspresso] Encontradas {} ofertas.", postings.len());
                    }
                    Ok(AgentMessage::RawJobsScraped(postings))
                }
                Err(e) => {
                    error!("❌ [Jobspresso] Error fatal: {}", e);
                    Ok(AgentMessage::RawJobsScraped(vec![]))
                }
            },
            _ => Err(AgentError::Scraping("Msg inválido".into())),
        }
    }

    fn name(&self) -> &str {
        "scraper_jobspresso"
    }
}
