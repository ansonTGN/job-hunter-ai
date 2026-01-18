export async function apiJson(path, body) {
  const res = await fetch(path, {
    method: "POST",
    headers: { "Content-Type":"application/json" },
    body: JSON.stringify(body ?? {})
  });
  return await res.json();
}

// MODIFICADO: Ahora acepta llmConfig
export async function uploadCv(file, llmConfig) {
  const form = new FormData();
  form.append("file", file);

  // Añadir configuración del LLM al form data
  if (llmConfig) {
    form.append("llm_provider", llmConfig.provider || "local");
    form.append("model", llmConfig.model || "");
    form.append("endpoint", llmConfig.endpoint || "");
    form.append("api_key", llmConfig.apiKey || "");
    form.append("base_url", llmConfig.baseUrl || "");
  }

  // v1 recomendado
  const res = await fetch("/api/v1/cv/extract", { method:"POST", body: form });
  if (res.ok) {
    const json = await res.json();
    if (json?.ok && json?.data?.text != null) {
        return { 
            text: json.data.text,
            keywords: json.data.keywords || [] 
        };
    }
  }

  // fallback legacy (sin keywords inteligentes)
  const res2 = await fetch("/api/upload-cv", { method:"POST", body: form });
  return await res2.json();
}

export async function startSearchV1(payloadV1) {
  const res = await fetch("/api/v1/search/start", {
    method:"POST",
    headers: { "Content-Type":"application/json" },
    body: JSON.stringify(payloadV1)
  });
  return await res.json();
}

export async function startSearchLegacy(payloadLegacy) {
  const res = await fetch("/api/start", {
    method:"POST",
    headers: { "Content-Type":"application/json" },
    body: JSON.stringify(payloadLegacy)
  });
  return await res.json();
}

export async function listOllamaModels(endpoint) {
  const url = new URL("/api/v1/models/ollama", location.origin);
  if (endpoint) url.searchParams.set("endpoint", endpoint);

  const res = await fetch(url.toString(), { method: "GET" });
  if (res.ok) {
    const json = await res.json();
    if (json?.ok && json?.data?.models) return { models: json.data.models };
  }
  return apiJson("/api/ollama/models", { endpoint });
}

export async function listCloudModels(provider, apiKey, baseUrl) {
  const res = await fetch("/api/v1/models/cloud", {
    method: "POST",
    headers: { "Content-Type":"application/json" },
    body: JSON.stringify({ provider, api_key: apiKey, base_url: baseUrl })
  });

  const json = await res.json().catch(() => ({}));
  if (json?.ok && json?.data?.models) return { models: json.data.models };
  return apiJson("/api/llm/models", { provider, api_key: apiKey, base_url: baseUrl });
}

export async function pingBackend() {
  const res = await fetch("/api/v1/health", { method: "GET" });
  return { ok: res.ok, status: res.status };
}

