use regex::RegexBuilder;
use tracing::warn;

pub fn find_snippets(text: &str, keyword: &str, _window_hint: usize) -> String {
    if keyword.trim().is_empty() {
        return "Consulta vacía.".to_string();
    }

    let pattern = format!(r"(?i){}", regex::escape(keyword));
    let re = match RegexBuilder::new(&pattern).build() {
        Ok(r) => r,
        Err(_) => return "Error construyendo expresión regular.".to_string(),
    };

    let mut hits = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    
    let mut i = 0;
    while i < lines.len() {
        if re.is_match(lines[i]) {
            let start = i.saturating_sub(1);
            let end = (i + 2).min(lines.len());
            
            let block = lines[start..end]
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join(" ");
            
            if !block.is_empty() {
                let is_duplicate = hits.last().map(|last: &String| last.contains(&block) || block.contains(last)).unwrap_or(false);
                if !is_duplicate {
                    hits.push(format!("• \"...{}...\"", block));
                }
            }
            i = end; 
        } else {
            i += 1;
        }
        
        if hits.len() >= 6 { break; }
    }

    if hits.is_empty() {
        return "No se encontraron coincidencias relevantes.".to_string();
    }

    hits.join("\n")
}

pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("\n[...TRUNCATED...]");
    out
}

pub fn parse_llm_json(text: &str) -> Result<serde_json::Value, String> {
    // 1. Intento directo
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text.trim()) {
        return Ok(v);
    }

    // 2. Extracción de bloque
    let candidate = extract_json_block(text).unwrap_or_else(|| text.to_string());
    
    // 3. Intento parseo del candidato
    match serde_json::from_str::<serde_json::Value>(&candidate) {
        Ok(v) => Ok(v),
        Err(e) => {
            // 4. Si falla, intentamos sanitizar caracteres de escape
            let sanitized = sanitize_json_string(&candidate);
            match serde_json::from_str::<serde_json::Value>(&sanitized) {
                Ok(v) => {
                    warn!("JSON reparado (escapes inválidos).");
                    Ok(v)
                },
                Err(e_san) => {
                    // 5. Si el error es EOF (corte abrupto), intentamos cerrar el JSON
                    if e_san.to_string().contains("EOF") {
                        let closed = try_close_json(&sanitized);
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&closed) {
                            warn!("JSON reparado (EOF - cierre forzado).");
                            return Ok(v);
                        }
                    }
                    
                    Err(format!("JSON malformado: {}", e))
                }
            }
        }
    }
}

fn extract_json_block(text: &str) -> Option<String> {
    let start = text.find('{')?;
    // Buscamos el último cierre, si no existe (EOF), tomamos hasta el final
    let end = text.rfind('}').unwrap_or(text.len()); 
    if start >= end { 
        return Some(text[start..].to_string()); 
    } 
    Some(text[start..=end].to_string())
}

/// Intenta cerrar estructuras JSON abiertas si el LLM se quedó a medias
fn try_close_json(s: &str) -> String {
    let mut out = s.to_string();
    
    // Balance básico de comillas (si es impar, falta una al final)
    if out.matches('"').count() % 2 != 0 {
        out.push('"');
    }
    
    // Balance de llaves y corchetes simplificado
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
    
    // Asegurar cierre del objeto raíz si quedó colgando
    if !out.trim_end().ends_with('}') {
         out.push('}');
    }

    out
}

/// Arregla escapes inválidos (ej: \s en regex o paths de windows)
fn sanitize_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    match next {
                        '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u' => {
                            out.push('\\'); 
                        },
                        _ => {
                            // Escape inválido, escapamos el backslash
                            out.push('\\');
                            out.push('\\');
                        }
                    }
                } else {
                    out.push('\\');
                    out.push('\\');
                }
            },
            // Caracteres de control reales rompen JSON, los escapamos
            '\n' => { out.push('\\'); out.push('n'); },
            '\r' => { }, 
            '\t' => { out.push('\\'); out.push('t'); },
            _ => out.push(c),
        }
    }
    out
}