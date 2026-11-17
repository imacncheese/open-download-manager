const keys = ["enabled", "intercept", "panel"];
chrome.storage.local.get({ enabled: true, intercept: true, panel: true }, (s) => {
  for (const k of keys) {
    const el = document.getElementById(k);
    el.checked = !!s[k];
    el.addEventListener("change", () => chrome.storage.local.set({ [k]: el.checked }));
  }
});

document.getElementById("ping").addEventListener("click", () => {
  const el = document.getElementById("status");
  el.textContent = "Checking...";
  chrome.runtime.sendMessage({ type: "ping" }, (r) => {
    if (r && r.ok) el.textContent = "Connected to Open Download Manager " + (r.version || "");
    else el.textContent = "Open Download Manager is not running or the native host is not installed.";
  });
});
