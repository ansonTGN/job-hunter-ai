export const state = {
  ws: null,
  wsConnected: false,

  runState: "idle", // idle | running
  logsCount: 0,

  jobsCount: 0,
  jobEls: new Map(), // Mapa para evitar duplicados en el DOM
  jobsData: [],      // CRÍTICO: Array para guardar los datos de las ofertas

  cloudModels: [],
  provider: "local",
};
