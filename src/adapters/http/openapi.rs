use axum::response::{Html, IntoResponse};
use axum::Json;

pub async fn openapi_json() -> impl IntoResponse {
    // OpenAPI 3.0 (manual, estable y explícito)
    // Mantiene v1 como superficie recomendada; legacy marcado como deprecated.
    let spec = serde_json::json!({
      "openapi": "3.0.3",
      "info": {
        "title": "Job Hunter API",
        "version": "1.0.0",
        "description": "API versionada y tipada para Job Hunter. Usa /api/v1/* como superficie estable. /api/* queda como compatibilidad legacy."
      },
      "paths": {
        "/api/v1/health": {
          "get": {
            "summary": "Healthcheck",
            "responses": {
              "200": { "description": "OK" }
            }
          }
        },
        "/api/v1/search/start": {
          "post": {
            "summary": "Start a search run",
            "requestBody": {
              "required": true,
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/StartSearchRequestV1" }
                }
              }
            },
            "responses": {
              "200": {
                "description": "Run started",
                "content": {
                  "application/json": {
                    "schema": { "$ref": "#/components/schemas/ApiOkStartSearchResponseV1" }
                  }
                }
              },
              "400": { "description": "Validation error" },
              "500": { "description": "Internal error" }
            }
          }
        },
        "/api/v1/cv/extract": {
          "post": {
            "summary": "Extract CV text from PDF/TXT",
            "requestBody": {
              "required": true,
              "content": {
                "multipart/form-data": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "file": { "type": "string", "format": "binary" }
                    },
                    "required": ["file"]
                  }
                }
              }
            },
            "responses": {
              "200": {
                "description": "Extracted",
                "content": {
                  "application/json": {
                    "schema": { "$ref": "#/components/schemas/ApiOkCvExtractResponseV1" }
                  }
                }
              }
            }
          }
        },
        "/api/v1/models/ollama": {
          "get": {
            "summary": "List Ollama models",
            "parameters": [
              {
                "name": "endpoint",
                "in": "query",
                "required": false,
                "schema": { "type": "string", "example": "http://localhost:11434" }
              }
            ],
            "responses": {
              "200": {
                "description": "Models",
                "content": {
                  "application/json": {
                    "schema": { "$ref": "#/components/schemas/ApiOkOllamaModelsResponseV1" }
                  }
                }
              }
            }
          }
        },
        "/api/v1/models/cloud": {
          "post": {
            "summary": "List cloud provider models (OpenAI/Anthropic)",
            "requestBody": {
              "required": true,
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/CloudModelsRequestV1" }
                }
              }
            },
            "responses": {
              "200": {
                "description": "Models",
                "content": {
                  "application/json": {
                    "schema": { "$ref": "#/components/schemas/ApiOkCloudModelsResponseV1" }
                  }
                }
              }
            }
          }
        },

        "/ws": {
          "get": {
            "summary": "WebSocket stream (events)",
            "description": "Eventos JSON con formato {type,payload}. type: log|status|job_found."
          }
        },

        "/api/start": {
          "post": {
            "deprecated": true,
            "summary": "LEGACY: start search (UI compatibility)",
            "responses": { "200": { "description": "OK" } }
          }
        },
        "/api/upload-cv": {
          "post": {
            "deprecated": true,
            "summary": "LEGACY: upload CV (UI compatibility)",
            "responses": { "200": { "description": "OK" } }
          }
        },
        "/api/ollama/models": {
          "post": {
            "deprecated": true,
            "summary": "LEGACY: list Ollama models",
            "responses": { "200": { "description": "OK" } }
          }
        },
        "/api/llm/models": {
          "post": {
            "deprecated": true,
            "summary": "LEGACY: list cloud models",
            "responses": { "200": { "description": "OK" } }
          }
        }
      },
      "components": {
        "schemas": {
          "ApiExperienceLevel": {
            "type": "string",
            "enum": ["entry","junior","mid","senior","lead","any"]
          },
          "ApiJobSource": {
            "type": "string",
            "enum": ["remoteok","wwr","arbeitnow","himalayas","jobspresso"]
          },
          "ApiLlmProvider": {
            "type": "string",
            "enum": ["local","openai","anthropic"]
          },
          "ApiUseCase": {
            "type": "string",
            "enum": ["fast","balanced","deep","long_context"]
          },
          "SourceSettingsV1": {
            "type": "object",
            "properties": {
              "source": { "$ref": "#/components/schemas/ApiJobSource" },
              "enabled": { "type": "boolean" },
              "delay_ms": { "type": "integer", "format": "int64" },
              "user_agent": { "type": "string" },
              "use_proxy": { "type": "boolean" }
            },
            "required": ["source","enabled"]
          },
          "CriteriaV1": {
            "type": "object",
            "properties": {
              "keywords": { "type": "array", "items": { "type": "string" } },
              "experience_level": { "$ref": "#/components/schemas/ApiExperienceLevel" },
              "sources_config": { "type": "array", "items": { "$ref": "#/components/schemas/SourceSettingsV1" } },
              "user_cv": { "type": "string", "nullable": true }
            },
            "required": ["keywords","experience_level","sources_config"]
          },
          "LlmLocalV1": {
            "type": "object",
            "properties": {
              "endpoint": { "type": "string" },
              "model": { "type": "string" }
            },
            "required": ["endpoint","model"]
          },
          "LlmCloudV1": {
            "type": "object",
            "properties": {
              "api_key": { "type": "string" },
              "base_url": { "type": "string", "nullable": true },
              "model": { "type": "string", "nullable": true }
            },
            "required": ["api_key"]
          },
          "LlmConfigV1": {
            "type": "object",
            "properties": {
              "provider": { "$ref": "#/components/schemas/ApiLlmProvider" },
              "use_case": { "$ref": "#/components/schemas/ApiUseCase" },
              "local": { "$ref": "#/components/schemas/LlmLocalV1", "nullable": true },
              "cloud": { "$ref": "#/components/schemas/LlmCloudV1", "nullable": true }
            },
            "required": ["provider"]
          },
          "StartSearchRequestV1": {
            "type": "object",
            "properties": {
              "criteria": { "$ref": "#/components/schemas/CriteriaV1" },
              "llm": { "$ref": "#/components/schemas/LlmConfigV1" }
            },
            "required": ["criteria","llm"]
          },
          "StartSearchResponseV1": {
            "type": "object",
            "properties": {
              "run_id": { "type": "string", "format": "uuid" }
            },
            "required": ["run_id"]
          },
          "ApiOkStartSearchResponseV1": {
            "type": "object",
            "properties": {
              "ok": { "type": "boolean" },
              "data": { "$ref": "#/components/schemas/StartSearchResponseV1" }
            },
            "required": ["ok","data"]
          },
          "CvExtractResponseV1": {
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
          },
          "ApiOkCvExtractResponseV1": {
            "type": "object",
            "properties": {
              "ok": { "type": "boolean" },
              "data": { "$ref": "#/components/schemas/CvExtractResponseV1" }
            },
            "required": ["ok","data"]
          },
          "OllamaModelTag": {
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
          },
          "OllamaModelsResponseV1": {
            "type": "object",
            "properties": {
              "endpoint": { "type": "string" },
              "models": { "type": "array", "items": { "$ref": "#/components/schemas/OllamaModelTag" } }
            },
            "required": ["endpoint","models"]
          },
          "ApiOkOllamaModelsResponseV1": {
            "type": "object",
            "properties": {
              "ok": { "type": "boolean" },
              "data": { "$ref": "#/components/schemas/OllamaModelsResponseV1" }
            },
            "required": ["ok","data"]
          },
          "CloudModelsRequestV1": {
            "type": "object",
            "properties": {
              "provider": { "$ref": "#/components/schemas/ApiLlmProvider" },
              "api_key": { "type": "string" },
              "base_url": { "type": "string", "nullable": true }
            },
            "required": ["provider","api_key"]
          },
          "CloudModelsResponseV1": {
            "type": "object",
            "properties": {
              "provider": { "$ref": "#/components/schemas/ApiLlmProvider" },
              "base_url": { "type": "string" },
              "models": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["provider","base_url","models"]
          },
          "ApiOkCloudModelsResponseV1": {
            "type": "object",
            "properties": {
              "ok": { "type": "boolean" },
              "data": { "$ref": "#/components/schemas/CloudModelsResponseV1" }
            },
            "required": ["ok","data"]
          }
        }
      }
    });

    Json(spec)
}

pub async fn docs_page() -> impl IntoResponse {
    Html(
        r#"
<!doctype html>
<html lang="es">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <title>Job Hunter API Docs</title>
  <style>
    body{font-family:system-ui,-apple-system,Segoe UI,Roboto,Ubuntu,Cantarell,Arial,sans-serif;margin:40px;line-height:1.45}
    code{background:#f2f2f2;padding:2px 6px;border-radius:6px}
    a{color:#0b62d6}
  </style>
</head>
<body>
  <h1>Job Hunter API Docs</h1>
  <p>OpenAPI JSON:</p>
  <ul>
    <li><a href="/api-docs/openapi.json">/api-docs/openapi.json</a></li>
  </ul>

  <h2>Superficie recomendada</h2>
  <p>Usa endpoints versionados <code>/api/v1/*</code>. Los endpoints <code>/api/*</code> quedan como compatibilidad legacy.</p>

  <h2>WebSocket</h2>
  <p>Conecta a <code>/ws</code> y recibe eventos JSON: <code>log</code>, <code>status</code>, <code>job_found</code>.</p>

  <p>Si quieres una UI Swagger completa, puedes pegar el OpenAPI JSON en Swagger Editor.</p>
</body>
</html>
"#,
    )
}
