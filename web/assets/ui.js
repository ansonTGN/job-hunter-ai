import { state } from "./state.js";
import { $, escapeHtml, escapeAttr, clamp01, providerDefaults, modelTags } from "./utils.js";

// --- CONSTANTES ---
const SOURCES = [
  { id: "remoteok", name: "RemoteOK" },
  { id: "wwr", name: "We Work Remotely" },
  { id: "arbeitnow", name: "Arbeitnow" },
  { id: "himalayas", name: "Himalayas" },
  { id: "jobspresso", name: "Jobspresso" },
  { id: "remotive", name: "Remotive" },
  { id: "jobicy", name: "Jobicy" },
  { id: "findwork", name: "FindWork" },
  { id: "workingnomads", name: "Working Nomads" },
  { id: "vuejobs", name: "VueJobs" },
  { id: "cryptojobs", name: "CryptoJobs" },
  { id: "devitjobs", name: "DevITJobs" },
  { id: "golangprojects", name: "Golang Projects" },
  { id: "pythonorg", name: "Python.org" },
  { id: "remoteco", name: "Remote.co" },
];

// --- RENDERIZADO DE FUENTES (Grid Layout) ---
export function renderSources() {
  const host = $("sources");
  if (!host) return;
  host.innerHTML = "";
  // Aplicamos grid para que se vea ordenado
  host.style.display = "grid";
  host.style.gridTemplateColumns = "repeat(auto-fill, minmax(160px, 1fr))";
  host.style.gap = "10px";

  for (const s of SOURCES) {
    const label = document.createElement("label");
    label.className = "check";
    label.innerHTML = `
      <input type="checkbox" id="src_${s.id}" checked />
      <div><div class="source-title">${escapeHtml(s.name)}</div></div>
    `;
    host.appendChild(label);
  }
}

export function buildSourceConfigs() {
  const sourceConfigs = {};
  const globalDelay = Number($("globalDelay")?.value || 1200);
  for (const s of SOURCES) {
    const el = $(`src_${s.id}`);
    sourceConfigs[s.id] = {
      enabled: el ? el.checked : true,
      delay_ms: globalDelay,
      user_agent: "Mozilla/5.0",
      use_proxy: false
    };
  }
  return sourceConfigs;
}

// --- RENDERIZADO DE KEYWORDS (CV) ---
export function renderCvKeywords(keywords) {
  const area = $("cvKeywordsArea");
  const container = $("cvKeywordsChips");
  
  if (!area || !container) return; 
  if (!keywords || !keywords.length) { area.style.display = "none"; return; }

  area.style.display = "block";
  container.innerHTML = "";

  keywords.forEach(kw => {
    const chip = document.createElement("button");
    chip.className = "chip";
    chip.textContent = kw;
    chip.type = "button";
    chip.onclick = () => {
      const input = $("keywords");
      let current = input.value.split(",").map(s => s.trim()).filter(Boolean);
      if (!current.some(ex => ex.toLowerCase() === kw.toLowerCase())) {
        current.push(kw);
        input.value = current.join(", ");
        chip.classList.add("chip--active");
        chip.disabled = true;
      }
    };
    container.appendChild(chip);
  });
}

// --- LOGS & ESTADO ---
export function addLog(level, msg) {
  const logs = $("logs");
  if (!logs) return;
  
  const timestamp = new Date().toLocaleTimeString([], { hour12: false });
  const line = `[${timestamp}] [${String(level || "info").toUpperCase()}] ${msg}`;
  
  logs.textContent += (logs.textContent ? "\n" : "") + line;
  logs.scrollTop = logs.scrollHeight;

  state.logsCount++;
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
  const host = $("toasts");
  if (!host) return;
  const t = document.createElement("div");
  t.className = "toast";
  t.innerHTML = `<div class="toast__title">${escapeHtml(title)}</div><div class="toast__msg">${escapeHtml(msg)}</div>`;
  host.appendChild(t);
  setTimeout(() => t.remove(), 4000);
}

export function clearUi() {
  const logs = $("logs");
  if (logs) logs.textContent = "";
  
  const jobs = $("jobs");
  if (jobs) jobs.innerHTML = `<div class="emptyState" id="emptyState">Aún no hay resultados. Inicia una búsqueda.</div>`;
  
  state.jobEls.clear();
  state.jobsData = [];
  state.jobsCount = 0;
  state.logsCount = 0;
  
  $("summaryJobs").textContent = "0";
  $("summaryLogs").textContent = "0";
  setLastEvent("—");
}

// --- RENDERIZADO DE TARJETAS (ESTRUCTURA PRO) ---

function getScoreClass(score) {
  if (score >= 0.8) return { cls: "score--high", text: `🚀 ${(score*100).toFixed(0)}% Match` };
  if (score >= 0.5) return { cls: "score--mid", text: `⚠️ ${(score*100).toFixed(0)}% Match` };
  return { cls: "score--low", text: `❌ ${(score*100).toFixed(0)}% Match` };
}

export function upsertJob(job, mode = "analyzed") {
  // Aseguramos que existe el array de datos
  if (!state.jobsData) state.jobsData = [];
  const existingIdx = state.jobsData.findIndex(j => j.id === job.id || j.url === job.url);
  if (existingIdx >= 0) { state.jobsData[existingIdx] = { ...state.jobsData[existingIdx], ...job }; }
  else { state.jobsData.push(job); }

  const jobsHost = $("jobs");
  if (!jobsHost) return;
  
  const empty = $("emptyState");
  if (empty) empty.remove();

  // Gestión del elemento DOM
  const url = String(job.url || "");
  const key = url || String(job.id || Math.random());
  let card = state.jobEls.get(key);
  const isNew = !card;

  if (!card) {
    card = document.createElement("div");
    card.className = "job";
    state.jobEls.set(key, card);
  }

  // --- PREPARACIÓN DE DATOS ---
  const title = escapeHtml(job.title || "(Sin título)");
  const company = escapeHtml(job.company?.name || job.company_name || "Confidencial");
  const loc = escapeHtml(job.location || "Remoto");
  const type = escapeHtml(job.job_type || "FullTime");
  const level = escapeHtml(job.experience_level || "Mid");
  
  const scoreVal = clamp01(job.match_score || 0);
  const scoreObj = getScoreClass(scoreVal);

  // Formateo inteligente de salario
  let salary = "No especificado";
  if (job.salary_normalized && job.salary_normalized > 0) {
      salary = `$${(job.salary_normalized/1000).toFixed(0)}k / año`;
  } else if (job.salary_range) {
      salary = `${job.salary_range.currency} ${job.salary_range.min}-${job.salary_range.max}`;
  }

  const reasons = (job.match_reasons || []).slice(0, 4);
  const redFlags = (job.red_flags || []);
  const matching = (job.skills_analysis?.matching || []);
  const missing = (job.skills_analysis?.missing || []);

  // Helpers HTML
  const renderList = (items, isFlag) => 
    items.length 
      ? `<ul class="job__list ${isFlag ? 'flags' : ''}">${items.map(i => `<li>${escapeHtml(i)}</li>`).join('')}</ul>` 
      : `<span class="small muted" style="font-style:italic">Ninguno detectado</span>`;

  const renderTags = (tags, styleClass) => 
    tags.length 
      ? tags.map(t => `<span class="skill-tag ${styleClass}">${escapeHtml(t)}</span>`).join('') 
      : `<span class="small muted" style="padding-left:4px;">—</span>`;

  // --- TEMPLATE HTML MEJORADO ---
  card.innerHTML = `
    <div class="job__header">
      <div class="job__main-info">
        <h3 class="job__title">
          <a href="${escapeAttr(url)}" target="_blank" rel="noopener">${title}</a>
        </h3>
        <div class="job__company">🏢 ${company}</div>
      </div>
      <div class="job__score-badge ${scoreObj.cls}">
        ${scoreObj.text}
      </div>
    </div>

    <!-- Barra de Metadatos -->
    <div class="job__meta-bar">
      <div class="meta-item" title="Ubicación">📍 <span>${loc}</span></div>
      <div class="meta-item" title="Salario">💰 <span>${salary}</span></div>
      <div class="meta-item" title="Tipo">💼 <span>${type}</span></div>
      <div class="meta-item" title="Experiencia">🎓 <span>${level}</span></div>
    </div>

    <div class="job__body">
      <!-- Columna 1: Análisis Cualitativo -->
      <div class="job__col">
        <div class="job__section">
          <span class="section-label">💡 Por qué encaja</span>
          ${renderList(reasons, false)}
        </div>
        
        ${redFlags.length > 0 ? `
        <div class="job__section" style="margin-top:16px;">
          <span class="section-label" style="color:var(--bad)">🚩 Alertas / Red Flags</span>
          ${renderList(redFlags, true)}
        </div>` : ''}
      </div>

      <!-- Columna 2: Skills (Chips Visuales) -->
      <div class="job__col">
        <div class="job__section">
          <span class="section-label">✅ Skills Coincidentes</span>
          <div class="tags-wrapper">
            ${renderTags(matching, 'skill--match')}
          </div>
        </div>

        <div class="job__section" style="margin-top:16px;">
          <span class="section-label">❌ Skills Faltantes</span>
          <div class="tags-wrapper">
            ${renderTags(missing, 'skill--missing')}
          </div>
        </div>
      </div>
    </div>

    <div class="job__footer">
      <a href="${escapeAttr(url)}" target="_blank" rel="noopener" class="btn-link">
        Ver Oferta Original ↗
      </a>
    </div>
  `;

  if (isNew) {
    jobsHost.prepend(card);
    state.jobsCount++;
    $("summaryJobs").textContent = String(state.jobsCount);
  }
}

// --- EXPORTAR CSV ---
export function exportResults() {
  if (!state.jobsData || state.jobsData.length === 0) { 
    showToast("Exportar", "No hay datos para exportar."); 
    return; 
  }
  
  const headers = ["Title", "Company", "Match Score", "Salary", "Remote", "Matching Skills", "Missing Skills", "Red Flags", "URL"];
  
  const rows = state.jobsData.map(j => {
    const t = (j.title || "").replace(/"/g, '""');
    const c = ((j.company?.name) || j.company_name || "").replace(/"/g, '""');
    const s = (j.match_score || 0).toFixed(2);
    const sal = (j.salary_normalized || "N/A");
    const rem = j.is_remote ? "Yes" : "No";
    const match = (j.skills_analysis?.matching || []).join("; ").replace(/"/g, '""');
    const miss = (j.skills_analysis?.missing || []).join("; ").replace(/"/g, '""');
    const flags = (j.red_flags || []).join("; ").replace(/"/g, '""');
    const u = j.url || "";
    
    return `"${t}","${c}","${s}","${sal}","${rem}","${match}","${miss}","${flags}","${u}"`;
  });

  const csvContent = [headers.join(",")].concat(rows).join("\n");
  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
  const link = document.createElement("a");
  const url = URL.createObjectURL(blob);
  
  link.setAttribute("href", url);
  link.setAttribute("download", `job_hunter_${new Date().toISOString().slice(0,10)}.csv`);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  
  showToast("Exportar", `Descargado: ${state.jobsData.length} ofertas.`);
}

// --- RESTO DE FUNCIONES (Config y Tabs) ---
export function setProviderUi(provider) {
  state.provider = provider;
  const isLocal = provider === "local";
  const isCloud = !isLocal;

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
  e.style.display = text ? "block" : "none";
  e.textContent = text;
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
