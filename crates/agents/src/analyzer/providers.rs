use super::types::{LlmProvider, UseCase};
use super::AnalyzerAgent;
use job_hunter_core::AgentError;
use serde_json::Value;
use std::sync::atomic::Ordering;

// Coste estimado promedio por llamada (input + output) para protección simple
const EST_COST_PER_CALL_USD: f64 = 0.01; 

impl AnalyzerAgent {
    /// Verifica si hemos excedido el presupuesto de seguridad (hardcoded a $3.00)
    fn check_budget(&self) -> Result<(), AgentError> {
        let count = self.usage_count.load(Ordering::Relaxed);
        let estimated_cost = count as f64 * EST_COST_PER_CALL_USD;
        
        if estimated_cost > 3.0 {
            return Err(AgentError::Llm(format!("PRESUPUESTO EXCEDIDO (Safety Stop): ${:.2}", estimated_cost)));
        }
        Ok(())
    }

    /// Router centralizado para llamadas LLM
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

    pub(crate) async fn call_ollama_with_fallback(&self, endpoint: &str, model: &str, prompt: &str) -> Result<String, AgentError> {
        match self.call_ollama_raw(endpoint, model, prompt).await {
            Ok(out) => Ok(out),
            Err(e) => {
                let msg = format!("{}", e);
                // Si el modelo no existe, intentamos 'llama3' como fallback común
                if msg.to_lowercase().contains("not found") {
                    let fallback = "llama3";
                    self.emit_log("warn", format!("Modelo {} no encontrado en Ollama. Usando fallback {}", model, fallback));
                    return self.call_ollama_raw(endpoint, fallback, prompt).await;
                }
                Err(e)
            }
        }
    }

    async fn call_ollama_raw(&self, endpoint: &str, model: &str, prompt: &str) -> Result<String, AgentError> {
        let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
        
        // JSON MODE: Ollama soporta nativamente 'format: "json"'
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        let res = self.http.post(&url)
            .json(&body)
            .send().await.map_err(|e| AgentError::Llm(e.to_string()))?;
        
        let txt = res.text().await.unwrap_or_default();
        
        // Intentar parsear la respuesta
        let v: Value = serde_json::from_str(&txt).map_err(|_| AgentError::Llm("Ollama devolvió JSON inválido".into()))?;
        
        v.get("response").and_then(|s| s.as_str()).map(|s| s.to_string())
             .ok_or_else(|| AgentError::Llm("Ollama response vacía".into()))
    }

    async fn call_openai(&self, key: &str, base: &str, model: Option<String>, use_case: UseCase, prompt: &str) -> Result<String, AgentError> {
        self.check_budget()?;
        
        let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
        
        // CORRECCIÓN PRINCIPAL:
        // Filtramos si el modelo es "auto", cadena vacía o None.
        let m = model.as_deref()
            .filter(|s| *s != "auto" && !s.trim().is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Auto-selección inteligente basada en el caso de uso
                match use_case {
                    UseCase::Fast => "gpt-4o-mini".to_string(),
                    UseCase::Deep => "gpt-4o".to_string(),
                    _ => "gpt-4o-mini".to_string(), // Balanced por defecto
                }
            });
        
        // JSON MODE: OpenAI requiere 'json_object' y mencionar "JSON" en el system prompt
        let system_msg = "You are a helpful assistant designed to output JSON.";
        
        let body = serde_json::json!({
            "model": m,
            "messages": [
                {"role": "system", "content": system_msg},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.1, // Baja temperatura para consistencia estructural
            "response_format": { "type": "json_object" } 
        });

        let res = self.http.post(&url).bearer_auth(key).json(&body).send().await
            .map_err(|e| AgentError::Llm(e.to_string()))?;
        
        let status = res.status();
        let txt = res.text().await.unwrap_or_default();
        
        if !status.is_success() {
             return Err(AgentError::Llm(format!("OpenAI API Error (HTTP {}): {}", status, txt)));
        }

        let v: Value = serde_json::from_str(&txt).map_err(|_| AgentError::Llm("OpenAI response JSON inválido".into()))?;
        
        // Incrementar uso solo si fue exitoso
        self.usage_count.fetch_add(1, Ordering::Relaxed);

        v["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
            .ok_or_else(|| AgentError::Llm(format!("OpenAI content vacío. Raw: {}", txt)))
    }

    async fn call_anthropic(&self, key: &str, base: &str, model: Option<String>, use_case: UseCase, ver: &str, prompt: &str) -> Result<String, AgentError> {
        self.check_budget()?;
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));
        
        // CORRECCIÓN PRINCIPAL TAMBIÉN AQUÍ:
        let m = model.as_deref()
            .filter(|s| *s != "auto" && !s.trim().is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                 match use_case {
                    UseCase::Fast => "claude-3-haiku-20240307".to_string(), 
                    UseCase::Deep => "claude-3-opus-20240229".to_string(),
                    _ => "claude-3-5-sonnet-20240620".to_string(), 
                }
            });

        // JSON MODE (Hack para Anthropic): Prefill del asistente con "{"
        let body = serde_json::json!({
            "model": m,
            "max_tokens": 4096, 
            "messages": [
                {"role": "user", "content": prompt},
                {"role": "assistant", "content": "{"} // Forzamos inicio de JSON
            ]
        });

        let res = self.http.post(&url)
            .header("x-api-key", key).header("anthropic-version", ver)
            .json(&body).send().await.map_err(|e| AgentError::Llm(e.to_string()))?;

        let status = res.status();
        let txt = res.text().await.unwrap_or_default();
        
        if !status.is_success() {
             return Err(AgentError::Llm(format!("Anthropic API Error (HTTP {}): {}", status, txt)));
        }

        let v: Value = serde_json::from_str(&txt).map_err(|_| AgentError::Llm("Anthropic response JSON inválido".into()))?;
        
        self.usage_count.fetch_add(1, Ordering::Relaxed);

        // Reconstruimos el JSON válido añadiendo la llave de apertura que pre-rellenamos
        v["content"][0]["text"].as_str()
            .map(|s| format!("{{{}", s)) 
            .ok_or_else(|| AgentError::Llm(format!("Anthropic content vacío. Raw: {}", txt)))
    }
}