// Open Download Manager browser integration service worker.
// Mirrors the classic download-manager integration module: context menus, download takeover,
// media sniffing and the in-page download panel.

const HOST = "com.opendm.host";
const DEFAULT_EXTS = [
  "3gp","7z","aac","ace","aif","apk","arj","asf","avi","bin","bz2","exe","gz","gzip",
  "img","iso","lzh","m4a","m4v","mkv","mov","mp3","mp4","mpa","mpe","mpeg","mpg",
  "msi","msu","ogg","ogv","pdf","plj","pps","ppt","qt","ra","rar","rm","rmvb","sea",
  "sit","sitx","tar","tif","tiff","wav","wma","wmv","z","zip","flv","webm","ts","doc",
  "docx","xls","xlsx","epub"
];

let port = null;
let seq = 0;
const pending = new Map();
const intercepted = new Set();
const mediaByTab = new Map();

function connect() {
  if (port) return port;
  try {
    port = chrome.runtime.connectNative(HOST);
    port.onMessage.addListener((msg) => {
      const id = msg && msg.__id;
      if (id && pending.has(id)) {
        pending.get(id)(msg);
        pending.delete(id);
      }
    });
    port.onDisconnect.addListener(() => {
      port = null;
      for (const [, resolve] of pending) resolve({ ok: false, error: "native host disconnected" });
      pending.clear();
    });
  } catch (e) {
    port = null;
  }
  return port;
}

function native(msg, timeoutMs = 8000) {
  return new Promise((resolve) => {
    const p = connect();
    if (!p) {
      resolve({ ok: false, error: "native host unavailable" });
      return;
    }
    const id = ++seq;
    pending.set(id, resolve);
    try {
      p.postMessage({ ...msg, __id: id });
    } catch (e) {
      pending.delete(id);
      resolve({ ok: false, error: String(e) });
    }
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        resolve({ ok: false, error: "timeout" });
      }
    }, timeoutMs);
  });
}

async function settings() {
  const s = await chrome.storage.local.get({ enabled: true, intercept: true, exts: DEFAULT_EXTS, panel: true });
  return s;
}

async function cookieHeader(url) {
  try {
    const cs = await chrome.cookies.getAll({ url });
    return cs.map((c) => `${c.name}=${c.value}`).join("; ");
  } catch (e) {
    return "";
  }
}

async function addDownload(info) {
  const cookies = await cookieHeader(info.url);
  return native({
    cmd: "add",
    url: info.url,
    referer: info.referer || "",
    cookies,
    userAgent: navigator.userAgent,
    filename: info.filename || "",
    description: info.description || "",
    start: info.start !== false
  });
}

chrome.runtime.onInstalled.addListener(async () => {
  const menus = [
    { id: "idm-link", title: "Download with ODM", contexts: ["link"] },
    { id: "idm-media", title: "Download this video/audio with ODM", contexts: ["video", "audio"] },
    { id: "idm-image", title: "Download this image with ODM", contexts: ["image"] },
    { id: "idm-page", title: "Download all links with ODM", contexts: ["page"] },
    { id: "idm-selection", title: "Download selected links with ODM", contexts: ["selection"] }
  ];
  for (const m of menus) {
    try { await chrome.contextMenus.create(m); } catch (e) {}
  }
  ensureHost();
});

async function ensureHost() {
  const r = await native({ cmd: "ping" });
  if (!r || !r.ok) {
    chrome.notifications?.create("odm-host", {
      type: "basic",
      iconUrl: "icons/128.png",
      title: "Open Download Manager",
      message: "Start Open Download Manager to enable browser integration."
    });
  }
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const referer = info.pageUrl || (tab && tab.url) || "";
  if (info.menuItemId === "idm-link" && info.linkUrl) {
    await addDownload({ url: info.linkUrl, referer });
  } else if (info.menuItemId === "idm-media" && info.srcUrl) {
    await addDownload({ url: info.srcUrl, referer });
  } else if (info.menuItemId === "idm-image" && info.srcUrl) {
    await addDownload({ url: info.srcUrl, referer });
  } else if (info.menuItemId === "idm-page" && tab) {
    const urls = await collectPageLinks(tab.id);
    if (urls.length) await native({ cmd: "add_many", urls, referer });
  } else if (info.menuItemId === "idm-selection" && tab) {
    const urls = (info.selectionText || "").match(/https?:\/\/[^\s"'<>]+/g) || [];
    if (urls.length) await native({ cmd: "add_many", urls, referer });
  }
});

async function collectPageLinks(tabId) {
  try {
    const res = await chrome.scripting.executeScript({
      target: { tabId },
      func: () => Array.from(document.querySelectorAll("a[href]"))
        .map((a) => a.href)
        .filter((u) => /^https?:/i.test(u))
    });
    return Array.from(new Set((res && res[0] && res[0].result) || []));
  } catch (e) {
    return [];
  }
}

// ------------------------------------------------------------------ takeover

chrome.downloads.onCreated.addListener(async (item) => {
  const cfg = await settings();
  if (!cfg.enabled || !cfg.intercept) return;
  if (item.byExtensionId === chrome.runtime.id) return;
  const url = item.finalUrl || item.url || "";
  if (!/^https?:/i.test(url)) return;
  if (intercepted.has(item.id)) return;

  const name = (item.filename || "").split(/[\\/]/).pop() || "";
  const ext = (name.split(".").pop() || "").toLowerCase();
  const mime = (item.mime || "").toLowerCase();
  const isPage = ["html", "htm", "xhtml", "php", "asp", "aspx", "jsp"].includes(ext) ||
    mime === "text/html" || mime === "application/xhtml+xml";
  if (isPage) return;
  if (cfg.exts.length && ext && !cfg.exts.includes(ext) && !mime.startsWith("video/") && !mime.startsWith("audio/")) {
    return;
  }

  intercepted.add(item.id);
  try {
    await chrome.downloads.cancel(item.id);
    await chrome.downloads.erase({ id: item.id });
  } catch (e) {}
  const resp = await addDownload({
    url,
    referer: item.referrer || "",
    filename: name,
    description: item.filename || ""
  });
  if (!resp || !resp.ok) {
    // If ODM cannot take it, restart the browser download.
    try { chrome.downloads.download({ url, filename: name || undefined }); } catch (e) {}
  }
});

// --------------------------------------------------------------- media sniff

const MEDIA_TYPES = { media: true, xmlhttprequest: true, other: true };
chrome.webRequest.onBeforeRequest.addListener(
  (d) => {
    if (!MEDIA_TYPES[d.type]) return;
    const url = d.url || "";
    if (!/^https?:/i.test(url)) return;
    if (!/\.(mp4|m4v|webm|mkv|flv|mp3|m4a|aac|ogg|ogv|ts|mov|avi|wmv)(\?|$)/i.test(url) &&
        !/\/videoplayback|\/media\?|\/manifest|\/master\.m3u8|\/index\.m3u8/i.test(url)) {
      return;
    }
    const list = mediaByTab.get(d.tabId) || [];
    if (!list.some((m) => m.url === url)) {
      list.push({ url, referer: d.initiator || "", time: Date.now() });
      mediaByTab.set(d.tabId, list.slice(-10));
    }
    if (d.tabId >= 0) {
      chrome.tabs.sendMessage(d.tabId, { type: "idm-media", url, referer: d.initiator || "" }).catch(() => {});
    }
  },
  { urls: ["<all_urls>"] }
);

chrome.tabs.onRemoved.addListener((id) => mediaByTab.delete(id));

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (!msg || !msg.type) return;
  if (msg.type === "add") {
    addDownload({ url: msg.url, referer: msg.referer || (sender.tab && sender.tab.url) || "" })
      .then((r) => sendResponse(r));
    return true;
  }
  if (msg.type === "add_many") {
    native({ cmd: "add_many", urls: msg.urls || [], referer: msg.referer || "" })
      .then((r) => sendResponse(r));
    return true;
  }
  if (msg.type === "last_media") {
    const list = mediaByTab.get(sender.tab && sender.tab.id) || [];
    sendResponse({ ok: true, url: list.length ? list[list.length - 1].url : "" });
    return false;
  }
  if (msg.type === "ping") {
    native({ cmd: "ping" }).then((r) => sendResponse(r));
    return true;
  }
});
