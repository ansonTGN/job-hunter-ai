import { state } from "./state.js";
import { $, providerDefaults } from "./utils.js";
import { connectWs } from "./ws.js";
import { uploadCv, startSearchV1, startSearchLegacy, listOllamaModels, listCloudModels, pingBackend } from "./api.js";
import {
  renderSources, buildSourceConfigs, addLog, clearUi,
  setProviderUi, renderCloudModelSelect, setCloudError,
  showToast, setupTabs, setBackendStatus, setLastEvent,
  renderCvKeywords, exportResults // <-- Importamos la nueva función
} from "./ui.js";

// ... (Resto de funciones: setCvStatus, refreshLocalModels, etc. se mantienen igual) ...
function setCvStatus(text) { const el = $("cvStatus"); if (el) el.textContent = text; }
async function refreshLocalModels() { /* ... igual que antes ... */ 
  const endpoint = $("localEndpoint").value;
  try {
    const json = await listOllamaModels(endpoint);
    const models = (json.models || []).map(m => m.name).filter(Boolean);
    const select = $("localModel");
    select.innerHTML = "";
    if (models.length === 0) { const opt = document.createElement("option"); opt.value = "llama3"; opt.textContent = "llama3 (fallback)"; select.appendChild(opt); showToast("Ollama", "No se detectaron modelos."); return; }
    for (const m of models.sort((a,b)=>a.localeCompare(b))) { const opt = document.createElement("option"); opt.value = m; opt.textContent = m; select.appendChild(opt); }
  } catch (e) { console.warn("Error Ollama:", e); }
}
async function refreshCloudModels() { /* ... igual que antes ... */ 
  const provider = $("llmProvider").value; const apiKey = $("apiKey").value; const baseUrl = $("cloudBaseUrl").value || providerDefaults(provider).baseUrl;
  setCloudError(""); addLog("info", `Listando modelos (${provider})...`);
  try {
    const json = await listCloudModels(provider, apiKey, baseUrl);
    if (json.error) { state.cloudModels = []; renderCloudModelSelect([]); setCloudError(`Error: ${json.error}`); addLog("warn", `Error listando: ${json.error}`); return; }
    state.cloudModels = json.models || []; renderCloudModelSelect(state.cloudModels); addLog("success", `Modelos cargados: ${state.cloudModels.length}`);
  } catch (e) { addLog("error", "Error red."); }
}
async function extractCv() { /* ... igual que antes ... */ 
  const fileInput = $("cvFile");
  if (!fileInput || !fileInput.files[0]) { showToast("CV", "Selecciona archivo."); return; }
  const file = fileInput.files[0];
  const btn = $("btnUploadCv");
  if(btn) { btn.disabled = true; btn.textContent = "Procesando..."; }
  setCvStatus("Analizando CV...");
  try {
    const provider = $("llmProvider").value;
    const llmConfig = { provider, model: provider === "local" ? $("localModel").value : $("cloudModel").value, endpoint: $("localEndpoint").value, apiKey: $("apiKey").value, baseUrl: $("cloudBaseUrl").value };
    addLog("info", `Analizando CV: ${file.name}...`);
    const json = await uploadCv(file, llmConfig);
    if (!json || (!json.text && !json.error)) throw new Error("Respuesta vacía.");
    if (json.error) throw new Error(json.error);
    $("cvText").value = json.text || "";
    if (json.keywords && json.keywords.length > 0) { renderCvKeywords(json.keywords); addLog("success", `CV analizado: ${json.keywords.length} skills.`); setCvStatus(`Análisis completo.`); }
    else { renderCvKeywords([]); addLog("warn", "Sin skills detectadas."); setCvStatus("Texto extraído."); }
    setLastEvent("cv_loaded");
  } catch (e) { console.error(e); addLog("error", `Fallo CV: ${e.message}`); setCvStatus("Error al procesar."); showToast("Error", e.message); }
  finally { if(btn) { btn.disabled = false; btn.textContent = "Cargar y Analizar CV"; } }
}

function splitKeywords(s) { return String(s || "").split(",").map(x => x.trim()).filter(Boolean); }
function mapExperience(exp) { const e = String(exp || "").toLowerCase(); if (["entry","junior","senior","lead","any"].includes(e)) return e; return "mid"; }
function mapSourceKeyToV1(k) {
  const map = {
    "remoteok":"remoteok", "wwr":"wwr", "arbeitnow":"arbeitnow", "himalayas":"himalayas", "jobspresso":"jobspresso",
    "remotive":"remotive", "jobicy":"jobicy", "findwork":"find_work", "workingnomads":"working_nomads", "vuejobs":"vue_jobs",
    "cryptojobs":"crypto_jobs", "devitjobs":"dev_it_jobs", "golangprojects":"golang_projects", "pythonorg":"python_org", "remoteco":"remote_co"
  };
  return map[k] || "remoteok";
}

function buildStartPayloadV1() {
  const provider = $("llmProvider").value;
  const legacySources = buildSourceConfigs(); // <-- Esto ahora lee el Delay global
  
  const sources_config = Object.entries(legacySources || {}).map(([k,v]) => ({
    source: mapSourceKeyToV1(k),
    enabled: Boolean(v?.enabled),
    delay_ms: Number(v?.delay_ms || 1200), // <-- Se envía al backend
    user_agent: String(v?.user_agent || "Mozilla/5.0"),
    use_proxy: Boolean(v?.use_proxy),
  }));

  const criteria = {
    keywords: splitKeywords($("keywords").value),
    experience_level: mapExperience($("experience").value),
    sources_config,
    user_cv: $("cvText").value || null,
  };

  const llm = {
      provider: provider,
      use_case: provider === "local" ? "balanced" : ($("llmUseCase").value || "balanced"),
      local: provider === "local" ? { endpoint: $("localEndpoint").value, model: $("localModel").value } : null,
      cloud: provider !== "local" ? { api_key: $("apiKey").value, base_url: $("cloudBaseUrl").value, model: $("cloudModel").value } : null
  };
  return { criteria, llm };
}

async function doStart() {
  addLog("info", "Iniciando búsqueda...");
  setLastEvent("start");
  try {
      const payload = buildStartPayloadV1();
      const json = await startSearchV1(payload);
      if (json?.ok) { addLog("success", `Búsqueda iniciada (ID: ${json.data.run_id})`); showToast("Job Hunter", "Búsqueda iniciada."); setLastEvent("started"); }
      else throw new Error(json?.error?.message || "Error desconocido");
  } catch (e) { addLog("error", e.message); showToast("Error", "No se pudo iniciar"); }
}

async function doPing() {
  try { const json = await pingBackend(); const ok = Boolean(json?.ok); setBackendStatus(ok); if(ok) addLog("success", "Backend online."); }
  catch (e) { setBackendStatus(false); console.warn("Ping fallido:", e); }
}

function wireEvents() {
  const btnStart = $("btnStart"); const btnStartB = $("btnStartBottom");
  if(btnStart) btnStart.addEventListener("click", doStart);
  if(btnStartB) btnStartB.addEventListener("click", doStart);

  const btnClear = $("btnClear"); const btnClearB = $("btnClearBottom");
  if(btnClear) btnClear.addEventListener("click", clearUi);
  if(btnClearB) btnClearB.addEventListener("click", clearUi);

  const btnExport = $("btnExport"); // <-- Evento Exportar
  if(btnExport) btnExport.addEventListener("click", exportResults);

  const btnRefresh = $("btnRefreshModels");
  if(btnRefresh) btnRefresh.addEventListener("click", async () => {
    const provider = $("llmProvider").value;
    if (provider === "local") await refreshLocalModels(); else await refreshCloudModels();
  });

  const btnUpload = $("btnUploadCv"); if(btnUpload) btnUpload.addEventListener("click", extractCv);
  const btnPing = $("btnPing"); if(btnPing) btnPing.addEventListener("click", doPing);
  const btnReconnect = $("btnReconnectWs"); if(btnReconnect) btnReconnect.addEventListener("click", () => { addLog("info", "Reconectando WS..."); connectWs(); });

  const providerSel = $("llmProvider");
  if(providerSel) providerSel.addEventListener("change", () => {
    const provider = providerSel.value; setProviderUi(provider);
    if (provider === "local") refreshLocalModels(); else renderCloudModelSelect(state.cloudModels);
  });

  const useCaseSel = $("llmUseCase"); if(useCaseSel) useCaseSel.addEventListener("change", () => { renderCloudModelSelect(state.cloudModels); });
  const endpointInp = $("localEndpoint"); if(endpointInp) endpointInp.addEventListener("change", refreshLocalModels);
}

function bootstrap() {
  console.log("Iniciando Job Hunter UI...");
  setupTabs(); renderSources();
  const providerSel = $("llmProvider"); if(providerSel) setProviderUi(providerSel.value);
  connectWs(); refreshLocalModels(); doPing(); wireEvents();
}

if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", bootstrap); else bootstrap();
