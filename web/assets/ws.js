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
        
      } else if (msg.type === "job_analyzed") {
        // Aquí es donde estaba fallando antes
        upsertJob(msg.payload, "analyzed");
        setLastEvent("job_analyzed");

      } else if (msg.type === "job_found") {
        upsertJob(msg.payload, "found");
        setLastEvent("job_found");

      } else if (msg.type === "status") {
        const p = String(msg.payload || "");
        addLog("info", `Estado: ${p}`);
        if (p.includes("started")) setRunStatus("running");
        if (p.includes("done") || p.includes("error")) setRunStatus("idle");
        
      } else {
        // Mensajes desconocidos
        console.log("Mensaje desconocido:", msg);
      }
    } catch (e) {
      console.error("Error procesando WS msg:", e);
      // Mostramos el error real en el log para depurar
      addLog("error", `UI Error: ${e.message}`);
    }
  };

  return ws;
}

