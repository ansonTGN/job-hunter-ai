pub mod types;
pub mod tools;
pub mod providers;
pub mod rlm;

use async_trait::async_trait;
use job_hunter_core::*;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::warn;

pub use self::types::{LlmProvider, UseCase, LlmAnalysis};
use self::tools::{truncate_chars, parse_llm_json};

pub struct AnalyzerAgent {
    llm: LlmProvider,
    http: reqwest::Client,
    ws_tx: Option<broadcast::Sender<String>>,
    max_html_chars: usize,
}

impl AnalyzerAgent {
    pub fn new_openai(api_key: String, base_url: String, model: Option<String>, use_case: UseCase) -> Self {
        Self {
            llm: LlmProvider::OpenAI { api_key, base_url, model, use_case },
            http: reqwest::Client::builder().timeout(Duration::from_secs(90)).build().unwrap(),
            ws_tx: None, 
            max_html_chars: 12_000,
        }
    }

    pub fn new_anthropic(api_key: String, base_url: String, model: Option<String>, use_case: UseCase) -> Self {
        Self {
            llm: LlmProvider::Anthropic { api_key, base_url, model, use_case, version: "2023-06-01".into() },
            http: reqwest::Client::builder().timeout(Duration::from_secs(90)).build().unwrap(),
            ws_tx: None, 
            max_html_chars: 12_000,
        }
    }

    pub fn new_local(endpoint: String, model: String) -> Self {
        Self {
            llm: LlmProvider::Local { endpoint, model },
            // --- CAMBIO CRÍTICO: Timeout 900s (15 min) para aguantar colas de inferencia ---
            http: reqwest::Client::builder().timeout(Duration::from_secs(900)).build().unwrap(),
            ws_tx: None, 
            max_html_chars: 4_000, 
        }
    }

    pub fn with_ws_tx(mut self, tx: broadcast::Sender<String>) -> Self {
        self.ws_tx = Some(tx);
        self
    }

    pub(crate) fn emit_log(&self, level: &str, msg: impl Into<String>) {
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(serde_json::json!({"type":"log", "payload": {"level": level, "msg": msg.into()}}).to_string());
        }
    }

    pub(crate) fn emit_job_analyzed(&self, job: &AnalyzedJobPosting) {
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(serde_json::json!({"type":"job_analyzed", "payload": job}).to_string());
        }
    }

    pub async fn extract_keywords_from_cv(&self, cv_text: &str) -> Result<Vec<String>, AgentError> {
        let snippet = truncate_chars(cv_text, 6000); 

        let prompt = format!(
            r#"ACT AS: Senior Tech Recruiter.
TASK: Extract key technical skills from CV.
OUTPUT: JSON ONLY. `{{ "keywords": ["rust", "python", ...] }}`.
NO INTRO. NO OUTRO.

CV TEXT:
{snippet}"#,
            snippet = snippet
        );

        self.emit_log("info", "🧠 [Analyzer] Extrayendo keywords del CV con IA...");
        
        let response = self.call_llm(&prompt).await?;
        let json = parse_llm_json(&response)
            .map_err(|e| AgentError::Analysis(format!("Error parsing LLM JSON: {}", e)))?;
        
        let keywords: Vec<String> = json
            .get("keywords")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        Ok(keywords)
    }

    async fn analyze_job(&self, raw: &RawJobPosting, criteria: &SearchCriteria) -> Result<AnalyzedJobPosting, AgentError> {
        let use_recursive = match &self.llm {
            LlmProvider::OpenAI { use_case, .. } | LlmProvider::Anthropic { use_case, .. } => 
                matches!(use_case, UseCase::Deep | UseCase::LongContext),
            _ => false,
        };

        if use_recursive {
            return self.analyze_job_recursive(raw, criteria).await;
        }

        let html_snip = truncate_chars(&raw.html_content, self.max_html_chars);
        let cv = criteria.user_cv.as_deref().map(|s| truncate_chars(s, 3000)).unwrap_or_else(|| "No CV".to_string());
        
        // Prompt reforzado para evitar texto basura fuera del JSON
        let prompt = format!(
            r#"Role: Recruiter.
Task: Evaluate Candidate vs Job.
Output: Strict JSON. No text outside braces.

Candidate:
{cv}

Job (Snippet):
{html}

JSON Structure:
{{
  "title": "Clean Job Title",
  "company_name": "Company Name",
  "match_score": 0.0 to 1.0,
  "match_reasons": ["reason1", "reason2"],
  "red_flags": ["flag1"],
  "skills_analysis": {{ "matching": ["skill1"], "missing": ["skill2"] }},
  "description": "Short summary",
  "location": "City or Remote",
  "is_remote": true/false
}}"#,
            cv = cv,
            html = html_snip
        );

        self.emit_log("info", format!("🤖 [Lineal] Analizando: {}", raw.url));
        let text = self.call_llm(&prompt).await?;
        
        let json = parse_llm_json(&text).map_err(|e| {
            warn!("JSON Error: {}. Resp: {:.100}...", e, text);
            AgentError::Analysis(e)
        })?;
        
        let analysis: LlmAnalysis = serde_json::from_value(json).map_err(|e| AgentError::Analysis(e.to_string()))?;
        
        let res = analysis.into_analyzed(raw, criteria);
        self.emit_job_analyzed(&res);
        Ok(res)
    }
}

#[async_trait]
impl Agent for AnalyzerAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::AnalyzeJobs(jobs, criteria) = msg {
            let mut analyzed = Vec::new();
            for job in jobs {
                match self.analyze_job(&job, &criteria).await {
                    Ok(res) => analyzed.push(res),
                    Err(e) => {
                        warn!("Error analizando {}: {}", job.url, e);
                        self.emit_log("error", format!("Fallo en {}: {}", job.url, e));
                    }
                }
            }
            Ok(AgentMessage::JobsAnalyzed(analyzed))
        } else {
            Err(AgentError::Analysis("Msg incorrecto".into()))
        }
    }
    fn name(&self) -> &str { "analyzer" }
}