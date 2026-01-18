use super::types::{LlmAnalysis};
use super::tools::{find_snippets, parse_llm_json};
use super::AnalyzerAgent;
use job_hunter_core::{AgentError, AnalyzedJobPosting, RawJobPosting, SearchCriteria};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct RlmAction {
    action: String, // "search", "read_cv", "finalize"
    query: Option<String>,
    analysis: Option<Value>,
}

impl AnalyzerAgent {
    /// Bucle de Razonamiento Recursivo (RLM / RIPPLE)
    pub(crate) async fn analyze_job_recursive(
        &self,
        raw: &RawJobPosting,
        criteria: &SearchCriteria,
    ) -> Result<AnalyzedJobPosting, AgentError> {
        
        let doc_text = &raw.html_content;
        let cv_text = criteria.user_cv.as_deref().unwrap_or("");
        
        // Estado del Grafo de Conocimiento (Knowledge Graph en texto)
        let mut context = String::from("ESTADO INICIAL: URL disponible. HTML cargado en memoria (no visible aún).\n");
        let objective = format!(
            "OBJETIVO: Evaluar match CV vs Oferta. Keywords: {}. Nivel: {:?}.", 
            criteria.keywords.join(", "), criteria.experience_level
        );

        let max_steps = 4; // Guardarraíl para evitar recursión infinita

        for i in 1..=max_steps {
            self.emit_log("info", format!("🔄 [RLM] Ciclo {}/{}: Razonando...", i, max_steps));

            // 1. READ & PLAN (Prompt Estratégico)
            let prompt = format!(
                r#"ACTÚA COMO: Agente de Investigación Recursiva (RLM).
{objective}

CONOCIMIENTO ACUMULADO:
{context}

HERRAMIENTAS:
- "search": Busca keywords exactas en la oferta (ej: "salary", "remote", "visa").
- "read_cv": Busca keywords en el CV (ej: "experience", "education").
- "finalize": Detener investigación y generar JSON final.

ESTRATEGIA REQUERIDA:
1. Prioriza buscar "Red Flags" (salario bajo, presencial oculto, tecnologías legacy).
2. Valida requisitos técnicos "Deal Breakers".
3. Solo finaliza cuando tengas evidencia suficiente para un Score preciso.

RESPONDE SOLO JSON:
{{ "action": "search"|"read_cv"|"finalize", "query": "...", "analysis": {{...}} }}
"#,
                objective=objective,
                context=context
            );

            let resp = self.call_llm(&prompt).await?;
            
            // 2. EVAL
            let step: RlmAction = match parse_llm_json(&resp) {
                Ok(v) => serde_json::from_value(v).unwrap_or(RlmAction { action: "search".into(), query: Some("requirements".into()), analysis: None }),
                Err(_) => RlmAction { action: "search".into(), query: Some("skills".into()), analysis: None },
            };

            // 3. LOOP (Execution)
            match step.action.as_str() {
                "search" => {
                    let q = step.query.unwrap_or_default();
                    self.emit_log("info", format!("🔎 [RLM] Buscando en oferta: '{}'", q));
                    // Usamos window_hint 0 porque ahora tools.rs usa lógica de líneas
                    let res = find_snippets(doc_text, &q, 0); 
                    context.push_str(&format!("\n[RESULTADO OFERTA '{}']:\n{}\n", q, res));
                }
                "read_cv" => {
                    let q = step.query.unwrap_or_default();
                    self.emit_log("info", format!("📄 [RLM] Consultando CV: '{}'", q));
                    let res = find_snippets(cv_text, &q, 0);
                    context.push_str(&format!("\n[RESULTADO CV '{}']:\n{}\n", q, res));
                }
                "finalize" => {
                    self.emit_log("success", "✅ [RLM] Finalizando investigación.");
                    if let Some(val) = step.analysis {
                         let analysis: LlmAnalysis = serde_json::from_value(val)
                            .map_err(|e| AgentError::Analysis(format!("JSON final invalido: {}", e)))?;
                         let res = analysis.into_analyzed(raw, criteria);
                         self.emit_job_analyzed(&res);
                         return Ok(res);
                    }
                    break; // Salir y forzar síntesis
                }
                _ => { context.push_str("\n[SYSTEM] Acción desconocida. Intenta 'search'.\n"); }
            }
        }

        // 4. PRINT (Síntesis final de fallback)
        self.emit_log("info", "📝 [RLM] Sintetizando reporte final...");
        let final_prompt = format!(
            "Genera el JSON final basado EXCLUSIVAMENTE en la evidencia recolectada:\n{}\nSCHEMA: {{title, company_name, description, match_score, match_reasons, ...}}", 
            context
        );
        let txt = self.call_llm(&final_prompt).await?;
        let json = parse_llm_json(&txt).map_err(|e| AgentError::Analysis(e))?;
        let analysis: LlmAnalysis = serde_json::from_value(json).map_err(|e| AgentError::Analysis(e.to_string()))?;
        
        let res = analysis.into_analyzed(raw, criteria);
        self.emit_job_analyzed(&res);
        Ok(res)
    }
}