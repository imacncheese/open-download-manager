// In-page IDM download panel (video/audio sniffed by the service worker).
(() => {
  if (window.__idmRsPanel) return;
  window.__idmRsPanel = true;

  let panel = null;
  let current = null;
  let hideTimer = null;

  function ensurePanel() {
    if (panel) return panel;
    panel = document.createElement("div");
    panel.id = "odm-panel";
    panel.innerHTML = `
      <div class="odm-head">
        <span class="odm-logo">ODM</span>
        <span class="odm-title">Download panel</span>
        <button class="odm-close" title="Close">×</button>
      </div>
      <div class="odm-body">
        <div class="odm-name"></div>
        <button class="odm-dl">Download this video with ODM</button>
      </div>`;
    document.documentElement.appendChild(panel);
    panel.querySelector(".odm-close").addEventListener("click", () => hide());
    panel.querySelector(".odm-dl").addEventListener("click", () => {
      if (!current) return;
      chrome.runtime.sendMessage({ type: "add", url: current, referer: location.href });
      hide();
    });
    return panel;
  }

  function fileName(url) {
    try {
      const u = new URL(url);
      const n = decodeURIComponent(u.pathname.split("/").pop() || "media");
      return n.length > 60 ? n.slice(0, 57) + "..." : n;
    } catch (e) {
      return "media";
    }
  }

  function show(url) {
    if (!url || url === current) return;
    current = url;
    const p = ensurePanel();
    p.querySelector(".odm-name").textContent = fileName(url);
    p.classList.add("odm-show");
    clearTimeout(hideTimer);
    hideTimer = setTimeout(hide, 20000);
  }

  function hide() {
    if (panel) panel.classList.remove("odm-show");
  }

  chrome.runtime.onMessage.addListener((msg) => {
    if (msg && msg.type === "idm-media" && msg.url) show(msg.url);
  });

  // Also expose current <video>/<audio> sources.
  document.addEventListener("mouseover", (e) => {
    const el = e.target;
    if (el && (el.tagName === "VIDEO" || el.tagName === "AUDIO")) {
      const src = el.currentSrc || el.src;
      if (src) show(src);
    }
  }, true);
})();
