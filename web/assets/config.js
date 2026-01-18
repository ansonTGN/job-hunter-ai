import { el, toast } from "./ui.js";

export function collectUiConfigFromForm() {
  const provider = el("llmProvider").value;
  const useCase = el("llmUseCase").value;

  const sources = {
    remoteok: el("srcRemoteOk").checked,
    weworkremotely: el("srcWeWorkRemotely").checked,
    jobspresso: el("srcJobspresso").checked,
    linkedin: el("srcLinkedIn").checked,
  };

  const config = {
    version: 1,
    criteria: {
      keywords: el("keywords").value || "",
      experience_level: el("experience").value || "Senior",
      max_results_per_source: Number(el("maxResults").value || 50),
      delay_ms: Number(el("delayMs").value || 600),
      sources,
    },
    llm: {
      provider,
      use_case: useCase,
      local_model: el("localModel").value || "",
      cloud_model: el("cloudModel").value || "",
      cloud_base_url: el("cloudBaseUrl").value || "",
    },
  };

  return config;
}

export function applyUiConfigToForm(cfg) {
  if (!cfg) return;

  const c = cfg.criteria || {};
  const llm = cfg.llm || {};

  el("keywords").value = c.keywords || "";
  el("experience").value = c.experience_level || "Senior";
  el("maxResults").value = String(c.max_results_per_source ?? 50);
  el("delayMs").value = String(c.delay_ms ?? 600);

  const s = c.sources || {};
  el("srcRemoteOk").checked = s.remoteok ?? true;
  el("srcWeWorkRemotely").checked = s.weworkremotely ?? true;
  el("srcJobspresso").checked = s.jobspresso ?? true;
  el("srcLinkedIn").checked = s.linkedin ?? false;

  el("llmProvider").value = llm.provider || "local";
  el("llmUseCase").value = llm.use_case || "balanced";
  el("cloudBaseUrl").value = llm.cloud_base_url || "";

  // Model selects se rellenan en app.js; aquí solo guardamos valores deseados
  if (llm.local_model) el("localModel").dataset.desired = llm.local_model;
  if (llm.cloud_model) el("cloudModel").dataset.desired = llm.cloud_model;
}

export function updateProviderVisibility() {
  const provider = el("llmProvider").value;

  const localWrap = el("localModelWrap");
  const cloudWrap = el("cloudModelWrap");
  const apiKeyWrap = el("apiKeyWrap");
  const apiKeyLabel = el("apiKeyLabel");

  if (provider === "local") {
    localWrap.style.display = "";
    cloudWrap.style.display = "none";
    apiKeyWrap.style.display = "none";
    el("cloudModelHint").textContent = "—";
    return;
  }

  localWrap.style.display = "none";
  cloudWrap.style.display = "";
  apiKeyWrap.style.display = "";

  apiKeyLabel.textContent = provider === "openai" ? "OpenAI API Key" : "Anthropic API Key";
}

export function desiredModel(selectEl) {
  return selectEl.dataset.desired || "";
}

export function trySelectDesired(selectEl) {
  const desired = desiredModel(selectEl);
  if (!desired) return false;
  const opt = Array.from(selectEl.options).find(o => o.value === desired);
  if (opt) {
    selectEl.value = desired;
    delete selectEl.dataset.desired;
    return true;
  }
  return false;
}

export function setConfigStatus(text, kind = "info") {
  const node = el("configStatus");
  node.textContent = text || "—";
  if (kind === "error") toast("Configuración", text, "error");
  if (kind === "ok") toast("Configuración", text, "ok");
}

