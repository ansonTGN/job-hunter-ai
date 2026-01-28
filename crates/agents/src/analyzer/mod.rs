pub mod types;
pub mod tools;
pub mod providers;
pub mod rlm;

use async_trait::async_trait;
use job_hunter_core::*;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::warn; // CORREGIDO: Eliminados info y error que no se usaban
use std::sync::atomic::AtomicUsize;

pub use self::types::{LlmProvider, UseCase, LlmAnalysis};
use self::tools::{truncate_chars, parse_llm_json};

/// Lista de keywords técnicas para el Fallback (Extracción de emergencia)
const COMMON_TECH_KEYWORDS: &[&str] = &[
    "rust", "python", "javascript", "typescript", "react", "vue", "angular", "node", "java", "c++", "c#", "go", "golang",
    "docker", "kubernetes", "aws", "azure", "gcp", "linux", "sql", "postgresql", "mysql", "mongodb", "redis", "elasticsearch",
    "git", "ci/cd", "jenkins", "github actions", "gitlab", "terraform", "ansible", "graphql", "rest api", "grpc",
    "microservices", "distributed systems", "machine learning", "ai", "llm", "nlp", "opencv", "pytorch", "tensorflow",
    "pandas", "numpy", "scikit-learn", "data science", "etl", "big data", "hadoop", "spark", "kafka", "rabbitmq",
    "blockchain", "solidity", "web3", "security", "cybersecurity", "penetration testing", "devops", "sre",
    "agile", "scrum", "kanban", "jira", "tdd", "bdd", "testing", "selenium", "cypress", "playwright"
];

pub struct AnalyzerAgent {
    llm: LlmProvider,
    http: reqwest::Client,
    ws_tx: Option<broadcast::Sender<String>>,
    max_html_chars: usize,
    pub usage_count: AtomicUsize,
}

impl AnalyzerAgent {
    pub fn new_openai(api_key: String, base_url: String, model: Option<String>, use_case: UseCase) -> Self {
        Self {
            llm: LlmProvider::OpenAI { api_key, base_url, model, use_case },
            http: reqwest::Client::builder().timeout(Duration::from_secs(90)).build().unwrap(),
            ws_tx: None, 
            max_html_chars: 12_000,
            usage_count: AtomicUsize::new(0),
        }
    }

    pub fn new_anthropic(api_key: String, base_url: String, model: Option<String>, use_case: UseCase) -> Self {
        Self {
            llm: LlmProvider::Anthropic { api_key, base_url, model, use_case, version: "2023-06-01".into() },
            http: reqwest::Client::builder().timeout(Duration::from_secs(90)).build().unwrap(),
            ws_tx: None, 
            max_html_chars: 12_000,
            usage_count: AtomicUsize::new(0),
        }
    }

    pub fn new_local(endpoint: String, model: String) -> Self {
        Self {
            llm: LlmProvider::Local { endpoint, model },
            // Timeout largo para modelos locales lentos
            http: reqwest::Client::builder().timeout(Duration::from_secs(900)).build().unwrap(),
            ws_tx: None, 
            max_html_chars: 4_000, 
            usage_count: AtomicUsize::new(0),
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

    /// Método principal para extraer keywords del CV.
    /// Utiliza una estrategia híbrida: Intenta LLM primero, si falla, usa Regex/Diccionario.
    pub async fn extract_keywords_from_cv(&self, cv_text: &str) -> Result<Vec<String>, AgentError> {
        // 1. Intentar extracción inteligente vía LLM
        match self.extract_keywords_llm(cv_text).await {
            Ok(kws) if !kws.is_empty() => Ok(kws),
            Ok(_) | Err(_) => {
                // 2. Si el LLM devuelve lista vacía o error, activar Fallback
                self.emit_log("warn", "⚠️ Falló extracción IA. Usando extracción estática de emergencia.");
                warn!("Activando fallback regex para keywords del CV.");
                Ok(self.extract_keywords_regex(cv_text))
            }
        }
    }

    /// Lógica privada de llamada al LLM para keywords
    async fn extract_keywords_llm(&self, cv_text: &str) -> Result<Vec<String>, AgentError> {
        let snippet = truncate_chars(cv_text, 4000); 

        let prompt = format!(
            r#"ACT AS: Senior Tech Recruiter.
TASK: Analyze this CV snippet and extract key technical skills (programming languages, frameworks, tools, cloud).
OUTPUT FORMAT: Strict JSON object with a single key "keywords".
EXAMPLE: {{ "keywords": ["Rust", "Python", "Docker", "AWS"] }}
NO INTRO. NO MARKDOWN. NO OUTRO.

CV TEXT:
{snippet}"#,
            snippet = snippet
        );

        self.emit_log("info", "🧠 [Analyzer] Extrayendo keywords del CV con IA...");
        
        // Llamada al proveedor configurado
        let response = self.call_llm(&prompt).await?;
        
        // Parseo robusto
        let json = parse_llm_json(&response)
            .map_err(|e| AgentError::Analysis(format!("Error parsing LLM JSON: {}", e)))?;
        
        let keywords: Vec<String> = json
            .get("keywords")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if keywords.is_empty() {
            return Err(AgentError::Analysis("El LLM devolvió una lista vacía".into()));
        }

        Ok(keywords)
    }

    /// Extracción determinista basada en diccionario (Fallback)
    fn extract_keywords_regex(&self, text: &str) -> Vec<String> {
        let text_lower = text.to_lowercase();
        let mut found = Vec::new();

        for &kw in COMMON_TECH_KEYWORDS {
            if text_lower.contains(kw) {
                // Capitalizamos la primera letra
                let cap = kw.chars().next().unwrap().to_uppercase().to_string() + &kw[1..];
                found.push(cap);
            }
        }
        
        // Eliminar duplicados y ordenar
        found.sort();
        found.dedup();
        
        if found.is_empty() {
            // Si incluso el diccionario falla, devolvemos genéricos si hay texto
            if !text.is_empty() {
                return vec!["Communication".to_string(), "Teamwork".to_string()];
            }
        }
        
        found
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
        
        let prompt = format!(
            r#"Role: Recruiter.
Task: Evaluate Candidate vs Job.
Output: Strict JSON.

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
