import { state } from "./state.js";
import { $, escapeHtml, escapeAttr, clamp01, providerDefaults, modelTags } from "./utils.js";

const SOURCES = [
  { id: "remoteok", name: "RemoteOK" },
  { id: "wwr", name: "We Work Remotely" },
  { id: "arbeitnow", name: "Arbeitnow" },
  { id: "himalayas", name: "Himalayas" },
  { id: "jobspresso", name: "Jobspresso" },
];

export function renderSources() {
  const host = $("sources");
  if (!host) {
    console.warn("UI: Contenedor #sources no encontrado.");
    return;
  }
  host.innerHTML = "";

  for (const s of SOURCES) {
    const label = document.createElement("label");
    label.className = "check";
    label.innerHTML = `
      <input type="checkbox" id="src_${s.id}" checked />
      <div>
        <div style="font-weight:800">${escapeHtml(s.name)}</div>
        <div class="hint">id: ${escapeHtml(s.id)}</div>
      </div>
    `;
    host.appendChild(label);
  }
}

export function buildSourceConfigs() {
  const sourceConfigs = {};
  for (const s of SOURCES) {
    const el = $(`src_${s.id}`);
    const enabled = el ? el.checked : true;
    sourceConfigs[s.id] = {
      enabled,
      delay_ms: 1200,
      user_agent: "Mozilla/5.0",
      use_proxy: false
    };
  }
  return sourceConfigs;
}

// --- KEYWORDS DEL CV ---
export function renderCvKeywords(keywords) {
  const area = $("cvKeywordsArea");
  const container = $("cvKeywordsChips");
  
  if (!area || !container) return; 

  if (!keywords || !Array.isArray(keywords) || keywords.length === 0) {
    area.style.display = "none";
    return;
  }

  area.style.display = "block";
  container.innerHTML = "";

  keywords.forEach(kw => {
    const chip = document.createElement("button");
    chip.className = "chip";
    chip.textContent = kw;
    chip.type = "button";
    chip.onclick = () => addKeywordToInput(kw, chip);
    container.appendChild(chip);
  });
}

function addKeywordToInput(kw, chipEl) {
  const input = $("keywords");
  if (!input) return;

  // Evitar duplicados
  let current = input.value.split(",").map(s => s.trim()).filter(Boolean);
  if (!current.some(existing => existing.toLowerCase() === kw.toLowerCase())) {
    current.push(kw);
    input.value = current.join(", ");
    
    // Feedback visual
    chipEl.classList.add("chip--active");
    chipEl.disabled = true;
  }
}

// --- LOGS & ESTADO ---

export function addLog(level, msg) {
  const logs = $("logs");
  if (!logs) return;
  
  const line = `[${String(level || "info").toUpperCase()}] ${msg}`;
  logs.textContent += (logs.textContent ? "\n" : "") + line;
  logs.scrollTop = logs.scrollHeight;

  state.logsCount += 1;
  const summary = $("summaryLogs");
  if (summary) summary.textContent = String(state.logsCount);
}

export function setLastEvent(text) {
  const el = $("summaryEvent");
  if (el) el.textContent = String(text || "—");
}

export function setWsStatus(connected) {
  state.wsConnected = connected;
  const dot = $("wsDot");
  const label = $("wsLabel");
  if (dot) dot.className = "dot " + (connected ? "dot--ok" : "dot--bad");
  if (label) label.textContent = connected ? "WS: conectado" : "WS: desconectado";
}

export function setBackendStatus(ok) {
  const dot = $("backendDot");
  const label = $("backendLabel");
  if (dot) dot.className = "dot " + (ok ? "dot--ok" : "dot--bad");
  if (label) label.textContent = ok ? "Backend: OK" : "Backend: KO";
}

export function setRunStatus(kind) {
  state.runState = kind;
}

export function showToast(title, msg) {
  const toasts = $("toasts");
  if (!toasts) return;
  const t = document.createElement("div");
  t.className = "toast";
  t.innerHTML = `<div class="toast__title">${escapeHtml(title)}</div><div class="toast__msg">${escapeHtml(msg)}</div>`;
  toasts.appendChild(t);
  setTimeout(() => t.remove(), 3500);
}

export function clearUi() {
  const logs = $("logs");
  if (logs) logs.textContent = "";
  
  const jobs = $("jobs");
  if (jobs) jobs.innerHTML = `<div class="emptyState" id="emptyState">Aún no hay resultados. Inicia una búsqueda para ver ofertas aquí.</div>`;
  
  state.jobEls.clear();
  state.jobsCount = 0;
  state.logsCount = 0;
  
  const sumJobs = $("summaryJobs");
  const sumLogs = $("summaryLogs");
  if (sumJobs) sumJobs.textContent = "0";
  if (sumLogs) sumLogs.textContent = "0";
  
  setLastEvent("—");
}

// --- RENDERIZADO DE JOBS ---

function scoreBadge(score01) {
  const s = clamp01(score01);
  if (s >= 0.70) return { cls: "badge badge--ok", text: `Match ${(s*100).toFixed(0)}%` };
  if (s >= 0.40) return { cls: "badge badge--warn", text: `Match ${(s*100).toFixed(0)}%` };
  return { cls: "badge badge--bad", text: `Match ${(s*100).toFixed(0)}%` };
}

export function upsertJob(job, mode = "analyzed") {
  const jobsHost = $("jobs");
  if (!jobsHost) return;
  
  const empty = $("emptyState");
  if (empty) empty.remove();

  const url = String(job.url || "");
  const key = url || String(job.id || Math.random());

  let card = state.jobEls.get(key);
  const isNew = !card;

  if (!card) {
    card = document.createElement("div");
    card.className = "job";
    state.jobEls.set(key, card);
  }

  const title = escapeHtml(job.title || "(sin título)");
  const company = escapeHtml((job.company && job.company.name) ? job.company.name : (job.company_name || "Unknown"));
  const loc = escapeHtml(job.location || "Remote");
  const score = clamp01(job.match_score ?? 0);
  const badge = scoreBadge(score);

  const redFlags = (job.red_flags || []).slice(0, 3).map(escapeHtml).join(" · ");
  const reasons = (job.match_reasons || []).slice(0, 3).map(escapeHtml).join(" · ");

  const statusBadge = mode === "found"
    ? `<span class="badge">Enriquecido</span>`
    : `<span class="badge">Analizado</span>`;

  card.innerHTML = `
    <div class="job__top">
      <div>
        <h3 class="job__title">${title}</h3>
        <div class="job__meta">
          <span class="${badge.cls}">${badge.text}</span>
          ${statusBadge}
          <span class="badge">${company}</span>
          <span class="badge">${loc}</span>
        </div>
      </div>
      <div class="job__links">
        ${url ? `<a class="link" href="${escapeAttr(url)}" target="_blank" rel="noopener">Abrir</a>` : ""}
      </div>
    </div>

    <div class="job__body">
      ${redFlags ? `<div><span class="muted">Red flags:</span> ${redFlags}</div>` : ""}
      ${reasons ? `<div style="margin-top:6px;"><span class="muted">Razones:</span> ${reasons}</div>` : ""}
    </div>
  `;

  if (isNew) {
    jobsHost.prepend(card);
    state.jobsCount += 1;
    const sum = $("summaryJobs");
    if (sum) sum.textContent = String(state.jobsCount);
  }
}

// --- GESTIÓN DE UI DEL PROVEEDOR LLM ---

export function setProviderUi(provider) {
  state.provider = provider;
  const isLocal = provider === "local";
  const isCloud = !isLocal;

  // Helper para ocultar/mostrar seguro
  const toggle = (id, show) => {
      const el = $(id);
      if (el) el.style.display = show ? "block" : "none";
  };

  toggle("fieldLocalEndpoint", isLocal);
  toggle("fieldLocalModel", isLocal);
  toggle("fieldApiKey", isCloud);
  toggle("fieldBaseUrl", isCloud);
  toggle("fieldCloudModel", isCloud);
  toggle("fieldUseCase", isCloud);

  if (isCloud) {
    const d = providerDefaults(provider);
    const inpBase = $("cloudBaseUrl");
    if (inpBase && !inpBase.value) inpBase.value = d.baseUrl;
  }
}

export function renderCloudModelSelect(models) {
  const provider = $("llmProvider").value;
  const useCase = $("llmUseCase").value;

  const select = $("cloudModel");
  if (!select) return;
  const prev = select.value;

  select.innerHTML = "";
  const optAuto = document.createElement("option");
  optAuto.value = "auto";
  optAuto.textContent = "auto";
  select.appendChild(optAuto);

  const filtered = (models || []).filter(m => {
    const tags = modelTags(provider, m);
    return tags.includes(useCase) || (useCase === "balanced" && tags.includes("balanced"));
  }).sort((a,b)=>a.localeCompare(b));

  for (const m of filtered) {
    const opt = document.createElement("option");
    opt.value = m;
    opt.textContent = m;
    select.appendChild(opt);
  }

  const hint = $("cloudModelHint");
  if (hint) hint.textContent = `Modelos: ${models.length} · Filtrados(${useCase}): ${filtered.length}`;
  
  if (prev && Array.from(select.options).some(o => o.value === prev)) select.value = prev;
}

export function setCloudError(text) {
  const e = $("cloudError");
  if (!e) return;
  if (!text) {
    e.style.display = "none";
    e.textContent = "";
  } else {
    e.style.display = "block";
    e.textContent = text;
  }
}

export function setupTabs() {
  const tabs = Array.from(document.querySelectorAll(".tab"));
  const views = {
    search: $("view-search"),
    logs: $("view-logs"),
    results: $("view-results"),
  };

  function activate(name) {
    for (const t of tabs) t.classList.toggle("tab--active", t.dataset.tab === name);
    for (const [k, v] of Object.entries(views)) {
        if (v) v.classList.toggle("view--active", k === name);
    }
  }

  for (const t of tabs) {
    t.addEventListener("click", () => activate(t.dataset.tab));
  }
}
