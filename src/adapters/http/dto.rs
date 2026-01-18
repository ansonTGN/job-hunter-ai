use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

//
// Respuestas estándar
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiOk<T> {
    pub ok: bool,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErr {
    pub ok: bool,
    pub error: ApiErrorBody,
}

//
// Health
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String, // "up"
}

//
// V1: Search
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSearchResponseV1 {
    pub run_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSearchRequestV1 {
    pub criteria: CriteriaV1,
    pub llm: LlmConfigV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaV1 {
    pub keywords: Vec<String>,
    pub experience_level: ApiExperienceLevel,
    pub sources_config: Vec<SourceSettingsV1>,
    #[serde(default)]
    pub user_cv: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSettingsV1 {
    pub source: ApiJobSource,
    pub enabled: bool,
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default)]
    pub use_proxy: bool,
}

fn default_delay_ms() -> u64 {
    1200
}
fn default_user_agent() -> String {
    "Mozilla/5.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiExperienceLevel {
    Entry,
    Junior,
    Mid,
    Senior,
    Lead,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiJobSource {
    Remoteok,
    Wwr,
    Arbeitnow,
    Himalayas,
    Jobspresso,
}

//
// V1: LLM
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfigV1 {
    pub provider: ApiLlmProvider,
    #[serde(default)]
    pub use_case: ApiUseCase,
    #[serde(default)]
    pub local: Option<LlmLocalV1>,
    #[serde(default)]
    pub cloud: Option<LlmCloudV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiLlmProvider {
    Local,
    Openai,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiUseCase {
    Fast,
    Balanced,
    Deep,
    LongContext,
}

impl Default for ApiUseCase {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmLocalV1 {
    pub endpoint: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCloudV1 {
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

//
// V1: CV extract
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvExtractResponseV1 {
    pub text: String,
    // --- NUEVO CAMPO AGREGADO ---
    #[serde(default)]
    pub keywords: Vec<String>,
}

//
// V1: Models
//

/// Query string para /api/v1/models/ollama?endpoint=http://...
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaModelsQueryV1 {
    #[serde(default)]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelsResponseV1 {
    pub endpoint: String,
    pub models: Vec<OllamaModelTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelTag {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudModelsRequestV1 {
    pub provider: ApiLlmProvider,
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudModelsResponseV1 {
    pub provider: ApiLlmProvider,
    pub base_url: String,
    pub models: Vec<String>,
}

//
// Legacy: compatibilidad con UI actual
//

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LegacyStartSearchRequest {
    pub keywords: String,
    pub experience: String,
    #[serde(default)]
    pub user_cv: Option<String>,

    #[serde(rename = "sourceConfigs")]
    pub source_configs: HashMap<String, LegacySourceCfg>,

    pub llm_provider: String,
    #[serde(default)]
    pub llm_use_case: String,

    // local
    #[serde(default)]
    pub local_endpoint: Option<String>,
    #[serde(default)]
    pub local_model: Option<String>,

    // cloud
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub cloud_model: Option<String>,
    #[serde(default)]
    pub openai_base_url: Option<String>,
    #[serde(default)]
    pub anthropic_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LegacySourceCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub use_proxy: bool,
}
