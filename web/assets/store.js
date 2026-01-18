const state = {
  cvText: "",
  jobs: [],
  logs: [],
};

export function setCvText(text) {
  state.cvText = (text || "").toString();
}

export function getCvText() {
  return state.cvText || "";
}

export function addLog(line) {
  state.logs.push(line);
  if (state.logs.length > 1500) state.logs.shift();
}

export function clearLogs() {
  state.logs = [];
}

export function getLogs() {
  return state.logs.slice();
}

export function addJob(job) {
  state.jobs.push(job);
}

export function setJobs(jobs) {
  state.jobs = Array.isArray(jobs) ? jobs.slice() : [];
}

export function clearJobs() {
  state.jobs = [];
}

export function getJobs() {
  return state.jobs.slice();
}

