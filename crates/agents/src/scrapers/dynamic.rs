use async_trait::async_trait;
use job_hunter_core::*;
use headless_chrome::{Browser, LaunchOptions};
use std::sync::Arc;
use std::time::Duration;
use governor::{Quota, RateLimiter};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use nonzero_ext::nonzero;
use tracing::{info, error};
use uuid::Uuid;
use chrono::Utc;

type AgentRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub struct DynamicScraperAgent {
    limiter: Arc<AgentRateLimiter>,
}

impl DynamicScraperAgent {
    pub fn new() -> Self {
        // Máximo 5 peticiones cada 60s
        let quota = Quota::with_period(Duration::from_secs(60)).unwrap().allow_burst(nonzero!(5u32));
        Self {
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    async fn scrape_spa_site(&self, url: &str, wait_selector: &str) -> anyhow::Result<String> {
        self.limiter.until_ready().await;

        info!("🌐 [Dynamic] Lanzando navegador headless para: {}", url);

        let options = LaunchOptions {
            headless: true,
            sandbox: false,
            idle_browser_timeout: Duration::from_secs(30),
            ..Default::default()
        };

        let browser = Browser::new(options)?;
        let tab = browser.new_tab()?;

        tab.navigate_to(url)?;
        
        info!("⏳ [Dynamic] Esperando selector: {}", wait_selector);
        // Espera máxima 10s por defecto en headless_chrome
        tab.wait_for_element(wait_selector)?;

        // Scroll
        tab.evaluate("window.scrollTo(0, document.body.scrollHeight)", false)?;
        std::thread::sleep(Duration::from_millis(1000)); 

        let content = tab.get_content()?;
        
        Ok(content)
    }
}

#[async_trait]
impl Agent for DynamicScraperAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        match msg {
            AgentMessage::StartScraping(_criteria) => {
                // Configura aquí URLs de prueba
                let target_url = "https://example.com/jobs"; 
                
                match self.scrape_spa_site(target_url, "body").await {
                    Ok(html) => {
                        let posting = RawJobPosting {
                            id: Uuid::new_v4().to_string(),
                            source: JobSource::Custom("DynamicSPA".to_string()),
                            url: target_url.to_string(),
                            html_content: html,
                            scraped_at: Utc::now(),
                        };
                        Ok(AgentMessage::RawJobsScraped(vec![posting]))
                    },
                    Err(e) => {
                        error!("❌ [Dynamic] Error: {}", e);
                        Ok(AgentMessage::RawJobsScraped(vec![]))
                    }
                }
            }
            _ => Err(AgentError::Scraping("Mensaje no soportado".into())),
        }
    }

    fn name(&self) -> &str {
        "scraper_dynamic_headless"
    }
}