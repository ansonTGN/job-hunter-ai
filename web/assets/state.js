export const state = {
  ws: null,
  wsConnected: false,

  runState: "idle", // idle | running
  logsCount: 0,

  jobsCount: 0,
  jobEls: new Map(), // url -> element

  cloudModels: [],
  provider: "local",
};
