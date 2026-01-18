import { addLog, setWsStatus, upsertJob, setRunStatus, setLastEvent } from "./ui.js";

function wsUrl() {
  const proto = (location.protocol === "https:") ? "wss" : "ws";
  return `${proto}://${location.host}/ws`;
}

export function connectWs() {
  const ws = new WebSocket(wsUrl());

  ws.onopen = () => {
    setWsStatus(true);
    setLastEvent("WS conectado");
    addLog("info", "WebSocket conectado.");
  };

  ws.onclose = () => {
    setWsStatus(false);
    setLastEvent("WS desconectado");
    addLog("warn", "WebSocket desconectado. Reintentando...");
    setTimeout(connectWs, 1200);
  };

  ws.onmessage = (ev) => {
    try {
      const msg = JSON.parse(ev.data);

      if (msg.type === "log") {
        addLog(msg.payload?.level || "info", msg.payload?.msg || "");
        setLastEvent(msg.payload?.msg || "log");

      } else if (msg.type === "job_analyzed") {
        upsertJob(msg.payload, "analyzed");
        setLastEvent("job_analyzed");

      } else if (msg.type === "job_found") {
        upsertJob(msg.payload, "found");
        setLastEvent("job_found");

      } else if (msg.type === "status") {
        const p = String(msg.payload || "");
        addLog("info", `Estado: ${p}`);
        setLastEvent(`status:${p}`);
        if (p.toLowerCase().includes("started")) setRunStatus("running");
        if (p.toLowerCase().includes("finished") || p.toLowerCase().includes("completed")) setRunStatus("idle");
      } else {
        setLastEvent("mensaje");
      }
    } catch {
      addLog("warn", String(ev.data));
      setLastEvent("raw");
    }
  };

  return ws;
}

