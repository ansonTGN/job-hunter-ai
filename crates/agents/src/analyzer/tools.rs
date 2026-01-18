use regex::RegexBuilder;
use tracing::warn;

/// Busca fragmentos de texto relevantes usando Regex case-insensitive.
/// Útil para RLM (Recursive Logic Module) cuando el agente busca "salary", "remote", etc.
pub fn find_snippets(text: &str, keyword: &str, _window_hint: usize) -> String {
    if keyword.trim().is_empty() {
        return "Consulta vacía.".to_string();
    }

    // Construir regex case-insensitive
    let pattern = format!(r"(?i){}", regex::escape(keyword));
    let re = match RegexBuilder::new(&pattern).build() {
        Ok(r) => r,
        Err(_) => return "Error construyendo expresión regular.".to_string(),
    };

    let mut hits = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    
    // Iteramos buscando coincidencias
    // Se extrae la línea completa donde aparece la keyword
    for line in lines {
        if re.is_match(line) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Evitamos duplicados exactos consecutivos
                if hits.last() != Some(&trimmed) {
                    hits.push(trimmed);
                }
            }
        }
        // Limitamos a 6 coincidencias para no saturar el contexto del LLM
        if hits.len() >= 6 { break; }
    }

    if hits.is_empty() {
        return "No se encontraron coincidencias relevantes.".to_string();
    }

    // Formateamos como lista
    hits.iter().map(|s| format!("• \"{}\"", s)).collect::<Vec<_>>().join("\n")
}

/// Trunca un string a un número máximo de caracteres de forma segura (UTF-8).
pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("\n[...TRUNCATED...]");
    out
}

/// Intenta parsear la respuesta del LLM como JSON, aplicando varias estrategias de limpieza y reparación.
pub fn parse_llm_json(text: &str) -> Result<serde_json::Value, String> {
    // 1. Limpieza de bloques de código Markdown (común en Llama 3 / GPT-4)
    // Ej: ```json { ... } ``` -> { ... }
    let clean_text = text
        .replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string();

    // 2. Extracción precisa del bloque JSON más externo
    // Busca el primer '{' y el último '}'
    let candidate = extract_json_block(&clean_text).unwrap_or_else(|| clean_text.clone());
    
    // 3. Intento directo de parseo
    match serde_json::from_str::<serde_json::Value>(&candidate) {
        Ok(v) => Ok(v),
        Err(e) => {
            // 4. Si falla, intentamos sanitizar caracteres de escape (ej: \n reales dentro de strings)
            let sanitized = sanitize_json_string(&candidate);
            match serde_json::from_str::<serde_json::Value>(&sanitized) {
                Ok(v) => {
                    warn!("JSON reparado (escapes inválidos).");
                    Ok(v)
                },
                Err(e_san) => {
                    // 5. Si el error sugiere EOF (corte abrupto), intentamos cerrar estructuras
                    if e_san.to_string().contains("EOF") {
                        let closed = try_close_json(&sanitized);
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&closed) {
                            warn!("JSON reparado (EOF - cierre forzado).");
                            return Ok(v);
                        }
                    }
                    
                    // Si todo falla, devolvemos el error original del candidato más limpio
                    Err(format!("JSON malformado: {}. Inicio respuesta: {:.50}...", e, candidate))
                }
            }
        }
    }
}

/// Extrae el substring entre el primer '{' y el último '}'.
fn extract_json_block(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?; // Usamos rfind para buscar desde el final
    
    if start >= end { 
        return None; 
    } 
    
    // Incluimos el '}' final en el slice
    Some(text[start..=end].to_string())
}

/// Arregla escapes inválidos comunes que los LLMs generan.
/// Ej: Caracteres de control reales (tabs, newlines) dentro de strings JSON.
fn sanitize_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    match next {
                        // Escapes válidos en JSON
                        '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u' => {
                            out.push('\\'); 
                        },
                        _ => {
                            // Escape inválido (ej: path de windows C:\Users), escapamos el backslash
                            out.push('\\');
                            out.push('\\');
                        }
                    }
                } else {
                    // Backslash al final del string
                    out.push('\\');
                    out.push('\\');
                }
            },
            // Caracteres de control reales rompen el parser de JSON estándar
            // Los reemplazamos por su versión escapada
            '\n' => { out.push('\\'); out.push('n'); },
            '\r' => { }, // Ignorar carriage return
            '\t' => { out.push('\\'); out.push('t'); },
            _ => out.push(c),
        }
    }
    out
}

/// Intenta cerrar estructuras JSON abiertas si el LLM se quedó sin tokens (timeout/length limit).
fn try_close_json(s: &str) -> String {
    let mut out = s.to_string();
    
    // Balance básico de comillas (si es impar, falta una al final)
    if out.matches('"').count() % 2 != 0 {
        out.push('"');
    }
    
    // Balance de llaves y corchetes simplificado
    // Contamos cuántos faltan por cerrar
    let open_braces = out.chars().filter(|&c| c == '{').count();
    let close_braces = out.chars().filter(|&c| c == '}').count();
    for _ in 0..open_braces.saturating_sub(close_braces) {
        out.push('}');
    }
    
    let open_brackets = out.chars().filter(|&c| c == '[').count();
    let close_brackets = out.chars().filter(|&c| c == ']').count();
    for _ in 0..open_brackets.saturating_sub(close_brackets) {
        out.push(']');
    }
    
    // Asegurar cierre del objeto raíz si quedó colgando (doble check)
    if !out.trim_end().ends_with('}') {
         out.push('}');
    }

    out
}