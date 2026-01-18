export function $(id) { return document.getElementById(id); }

export function escapeHtml(str) {
  return String(str ?? "").replace(/[&<>"']/g, (s) => ({
    "&":"&amp;",
    "<":"&lt;",
    ">":"&gt;",
    "\"":"&quot;",
    "'":"&#39;"
  }[s] || s));
}

export function escapeAttr(str) {
  return String(str ?? "").replace(/"/g, "&quot;");
}

export function clamp01(x) {
  const n = Number(x);
  if (Number.isNaN(n)) return 0;
  return Math.max(0, Math.min(1, n));
}

export function providerDefaults(provider) {
  if (provider === "openai") return { baseUrl: "https://api.openai.com" };
  if (provider === "anthropic") return { baseUrl: "https://api.anthropic.com" };
  return { baseUrl: "" };
}

export function modelTags(provider, id) {
  const s = String(id || "").toLowerCase();
  const tags = new Set();

  if (s.includes("mini") || s.includes("haiku")) tags.add("fast");
  if (s.includes("opus") || s.includes("o3") || s.includes("gpt-5")) tags.add("deep");
  if (s.includes("128k") || s.includes("200k") || s.includes("long")) tags.add("long_context");
  if (tags.size === 0) tags.add("balanced");
  if (tags.has("long_context")) tags.add("balanced");

  return Array.from(tags);
}
