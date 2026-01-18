use super::types::{LlmProvider, UseCase};
use super::AnalyzerAgent;
use job_hunter_core::AgentError;
use serde_json::Value;

// CORRECCIÓN: Eliminados imports no usados (Duration, sleep, warn)

impl AnalyzerAgent {
    pub(crate) async fn call_llm(&self, prompt: &str) -> Result<String, AgentError> {
        match &self.llm {
            LlmProvider::Local { endpoint, model } => {
                self.call_ollama_with_fallback(endpoint, model, prompt).await
            }
            LlmProvider::OpenAI { api_key, base_url, model, use_case } => {
                self.call_openai(api_key, base_url, model.clone(), *use_case, prompt).await
            }
            LlmProvider::Anthropic { api_key, base_url, model, use_case, version } => {
                self.call_anthropic(api_key, base_url, model.clone(), *use_case, version, prompt).await
            }
        }
    }

    // --- OLLAMA ---
    pub(crate) async fn call_ollama_with_fallback(&self, endpoint: &str, model: &str, prompt: &str) -> Result<String, AgentError> {
        match self.call_ollama_raw(endpoint, model, prompt).await {
            Ok(out) => Ok(out),
            Err(e) => {
                let msg = format!("{}", e);
                if msg.to_lowercase().contains("not found") {
                    let fallback = "llama3"; // Simplificación
                    self.emit_log("warn", format!("Modelo {} no encontrado. Usando fallback {}", model, fallback));
                    return self.call_ollama_raw(endpoint, fallback, prompt).await;
                }
                Err(e)
            }
        }
    }

    async fn call_ollama_raw(&self, endpoint: &str, model: &str, prompt: &str) -> Result<String, AgentError> {
        let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
        let res = self.http.post(&url)
            .json(&serde_json::json!({ "model": model, "prompt": prompt, "stream": false }))
            .send().await.map_err(|e| AgentError::Llm(e.to_string()))?;
        
        let txt = res.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&txt).map_err(|_| AgentError::Llm("Ollama bad json".into()))?;
        v.get("response").and_then(|s| s.as_str()).map(|s| s.to_string())
            .ok_or_else(|| AgentError::Llm("Ollama no response".into()))
    }

    // --- OPENAI ---
    async fn call_openai(&self, key: &str, base: &str, model: Option<String>, use_case: UseCase, prompt: &str) -> Result<String, AgentError> {
        let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
        // Selección simple de modelo si es None
        let m = model.unwrap_or_else(|| match use_case {
            UseCase::Fast => "gpt-4o-mini".to_string(),
            _ => "gpt-4o".to_string(),
        });

        let body = serde_json::json!({
            "model": m,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.2
        });

        let res = self.http.post(&url).bearer_auth(key).json(&body).send().await
            .map_err(|e| AgentError::Llm(e.to_string()))?;
        
        let txt = res.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&txt).map_err(|_| AgentError::Llm("OpenAI bad json".into()))?;
        
        v["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
            .ok_or_else(|| AgentError::Llm(format!("OpenAI err: {}", txt)))
    }

    // --- ANTHROPIC ---
    async fn call_anthropic(&self, key: &str, base: &str, model: Option<String>, use_case: UseCase, ver: &str, prompt: &str) -> Result<String, AgentError> {
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));
        let m = model.unwrap_or_else(|| match use_case {
            UseCase::Fast => "claude-3-5-haiku-latest".to_string(),
            _ => "claude-3-5-sonnet-latest".to_string(),
        });

        let body = serde_json::json!({
            "model": m,
            "max_tokens": 1500,
            "messages": [{"role": "user", "content": prompt}]
        });

        let res = self.http.post(&url)
            .header("x-api-key", key).header("anthropic-version", ver)
            .json(&body).send().await.map_err(|e| AgentError::Llm(e.to_string()))?;

        let txt = res.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&txt).map_err(|_| AgentError::Llm("Anthropic bad json".into()))?;
        
        v["content"][0]["text"].as_str().map(|s| s.to_string())
            .ok_or_else(|| AgentError::Llm(format!("Anthropic err: {}", txt)))
    }
}