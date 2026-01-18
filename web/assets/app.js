import { state } from "./state.js";
import { $, providerDefaults } from "./utils.js";
import { connectWs } from "./ws.js";
import { uploadCv, startSearchV1, startSearchLegacy, listOllamaModels, listCloudModels, pingBackend } from "./api.js";
import {
  renderSources, buildSourceConfigs, addLog, clearUi,
  setProviderUi, renderCloudModelSelect, setCloudError,
  showToast, setupTabs, setBackendStatus, setLastEvent,
  renderCvKeywords 
} from "./ui.js";

function setCvStatus(text) {
  const el = $("cvStatus");
  if (el) el.textContent = text;
}

async function refreshLocalModels() {
  const endpoint = $("localEndpoint").value;
  try {
    const json = await listOllamaModels(endpoint);
    const models = (json.models || []).map(m => m.name).filter(Boolean);
    const select = $("localModel");
    select.innerHTML = "";

    if (models.length === 0) {
        const opt = document.createElement("option");
        opt.value = "llama3";
        opt.textContent = "llama3 (fallback)";
        select.appendChild(opt);
        showToast("Ollama", "No se detectaron modelos. Asegúrate de que Ollama corre.");
        return;
    }

    for (const m of models.sort((a,b)=>a.localeCompare(b))) {
        const opt = document.createElement("option");
        opt.value = m;
        opt.textContent = m;
        select.appendChild(opt);
    }
  } catch (e) {
      console.warn("Error listando modelos Ollama:", e);
  }
}

async function refreshCloudModels() {
  const provider = $("llmProvider").value;
  const apiKey = $("apiKey").value;
  const baseUrl = $("cloudBaseUrl").value || providerDefaults(provider).baseUrl;

  setCloudError("");
  addLog("info", `Listando modelos (${provider})...`);

  try {
    const json = await listCloudModels(provider, apiKey, baseUrl);

    if (json.error) {
        state.cloudModels = [];
        renderCloudModelSelect([]);
        setCloudError(`Error: ${json.error}`);
        addLog("warn", `Error listando modelos: ${json.error}`);
        return;
    }

    state.cloudModels = json.models || [];
    renderCloudModelSelect(state.cloudModels);
    addLog("success", `Modelos cargados: ${state.cloudModels.length}`);
  } catch (e) {
      addLog("error", "Excepción de red al listar modelos.");
  }
}

async function extractCv() {
  const fileInput = $("cvFile");
  if (!fileInput || !fileInput.files[0]) {
    showToast("CV", "Selecciona un archivo PDF/TXT.");
    return;
  }
  const file = fileInput.files[0];
  const btn = $("btnUploadCv");

  // UI Feedback
  if(btn) {
      btn.disabled = true;
      btn.textContent = "Procesando...";
  }
  setCvStatus("Subiendo y analizando CV con IA...");
  
  try {
    const provider = $("llmProvider").value;
    const llmConfig = {
        provider: provider,
        model: provider === "local" ? $("localModel").value : $("cloudModel").value,
        endpoint: $("localEndpoint").value,
        apiKey: $("apiKey").value,
        baseUrl: $("cloudBaseUrl").value
    };

    addLog("info", `Analizando CV: ${file.name} (${llmConfig.provider})...`);
    
    const json = await uploadCv(file, llmConfig);

    if (!json || (!json.text && !json.error)) {
        throw new Error("Respuesta vacía del servidor.");
    }

    if (json.error) {
        throw new Error(json.error);
    }

    $("cvText").value = json.text || "";
    
    if (json.keywords && json.keywords.length > 0) {
        renderCvKeywords(json.keywords);
        addLog("success", `CV analizado. ${json.keywords.length} skills detectadas.`);
        setCvStatus(`Análisis completo: ${json.keywords.length} skills encontradas.`);
    } else {
        renderCvKeywords([]);
        addLog("warn", "CV extraído, pero no se detectaron skills.");
        setCvStatus("Texto extraído (sin skills detectadas).");
    }
    
    setLastEvent("cv_loaded");

  } catch (e) {
      console.error(e);
      addLog("error", `Fallo al procesar CV: ${e.message}`);
      setCvStatus("Error al procesar el archivo.");
      showToast("Error", e.message);
  } finally {
      if(btn) {
          btn.disabled = false;
          btn.textContent = "Cargar y Analizar CV";
      }
  }
}

function splitKeywords(s) {
  return String(s || "")
    .split(",")
    .map(x => x.trim())
    .filter(Boolean);
}

function mapExperience(exp) {
  const e = String(exp || "").toLowerCase();
  if (e === "entry") return "entry";
  if (e === "junior") return "junior";
  if (e === "senior") return "senior";
  if (e === "lead") return "lead";
  if (e === "any") return "any";
  return "mid";
}

function mapSourceKeyToV1(k) {
  if (k === "remoteok") return "remoteok";
  if (k === "wwr") return "wwr";
  if (k === "arbeitnow") return "arbeitnow";
  if (k === "himalayas") return "himalayas";
  if (k === "jobspresso") return "jobspresso";
  return "remoteok";
}

function buildStartPayloadV1() {
  const provider = $("llmProvider").value;
  const legacySources = buildSourceConfigs(); 
  
  const sources_config = Object.entries(legacySources || {}).map(([k,v]) => ({
    source: mapSourceKeyToV1(k),
    enabled: Boolean(v?.enabled),
    delay_ms: Number(v?.delay_ms || 1200),
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
      local: provider === "local" ? {
          endpoint: $("localEndpoint").value,
          model: $("localModel").value
      } : null,
      cloud: provider !== "local" ? {
          api_key: $("apiKey").value,
          base_url: $("cloudBaseUrl").value,
          model: $("cloudModel").value
      } : null
  };

  return { criteria, llm };
}

// Fallback legacy (por si acaso)
function buildStartPayloadLegacy() {
  return buildStartPayloadV1(); // Ya no necesitamos el payload legacy real, V1 es el estándar
}

async function doStart() {
  addLog("info", "Iniciando búsqueda...");
  setLastEvent("start");

  try {
      const payload = buildStartPayloadV1();
      const json = await startSearchV1(payload);

      if (json?.ok) {
        addLog("success", `Búsqueda iniciada (ID: ${json.data.run_id})`);
        showToast("Job Hunter", "Búsqueda iniciada.");
        setLastEvent("started");
      } else {
        throw new Error(json?.error?.message || "Error desconocido");
      }
  } catch (e) {
      addLog("error", e.message);
      showToast("Error", "No se pudo iniciar la búsqueda");
  }
}

async function doPing() {
  try {
    const json = await pingBackend();
    const ok = Boolean(json?.ok);
    setBackendStatus(ok);
    if(ok) addLog("success", "Conexión con Backend establecida.");
  } catch (e) {
    setBackendStatus(false);
    console.warn("Ping fallido:", e);
  }
}

function wireEvents() {
  const btnStart = $("btnStart");
  const btnStartB = $("btnStartBottom");
  if(btnStart) btnStart.addEventListener("click", doStart);
  if(btnStartB) btnStartB.addEventListener("click", doStart);

  const btnClear = $("btnClear");
  const btnClearB = $("btnClearBottom");
  if(btnClear) btnClear.addEventListener("click", clearUi);
  if(btnClearB) btnClearB.addEventListener("click", clearUi);

  const btnRefresh = $("btnRefreshModels");
  if(btnRefresh) btnRefresh.addEventListener("click", async () => {
    const provider = $("llmProvider").value;
    if (provider === "local") await refreshLocalModels();
    else await refreshCloudModels();
  });

  const btnUpload = $("btnUploadCv");
  if(btnUpload) btnUpload.addEventListener("click", extractCv);

  const btnPing = $("btnPing");
  if(btnPing) btnPing.addEventListener("click", doPing);

  const btnReconnect = $("btnReconnectWs");
  if(btnReconnect) btnReconnect.addEventListener("click", () => {
    addLog("info", "Reconectando WS...");
    connectWs();
  });

  const providerSel = $("llmProvider");
  if(providerSel) providerSel.addEventListener("change", () => {
    const provider = providerSel.value;
    setProviderUi(provider);
    if (provider === "local") {
        refreshLocalModels();
    } else {
        renderCloudModelSelect(state.cloudModels);
    }
  });

  const useCaseSel = $("llmUseCase");
  if(useCaseSel) useCaseSel.addEventListener("change", () => {
    renderCloudModelSelect(state.cloudModels);
  });

  const endpointInp = $("localEndpoint");
  if(endpointInp) endpointInp.addEventListener("change", refreshLocalModels);
}

// INICIO DE LA APLICACIÓN
function bootstrap() {
  console.log("Iniciando Job Hunter UI...");
  setupTabs();
  renderSources(); // <-- Esto debería pintar los checkboxes ahora
  
  const providerSel = $("llmProvider");
  if(providerSel) setProviderUi(providerSel.value);
  
  connectWs();
  refreshLocalModels();
  doPing();
  wireEvents();
}

// Arrancar cuando el DOM esté listo
if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", bootstrap);
} else {
    bootstrap();
}
