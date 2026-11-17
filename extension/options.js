const DEFAULT_EXTS = [
  "3gp","7z","aac","ace","aif","apk","arj","asf","avi","bin","bz2","exe","gz","gzip",
  "img","iso","lzh","m4a","m4v","mkv","mov","mp3","mp4","mpa","mpe","mpeg","mpg",
  "msi","msu","ogg","ogv","pdf","plj","pps","ppt","qt","ra","rar","rm","rmvb","sea",
  "sit","sitx","tar","tif","tiff","wav","wma","wmv","z","zip","flv","webm","ts","doc",
  "docx","xls","xlsx","epub"
];

chrome.storage.local.get({ enabled: true, intercept: true, panel: true, exts: DEFAULT_EXTS }, (s) => {
  document.getElementById("enabled").checked = !!s.enabled;
  document.getElementById("intercept").checked = !!s.intercept;
  document.getElementById("panel").checked = !!s.panel;
  document.getElementById("exts").value = (s.exts || []).join(" ");
});

document.getElementById("save").addEventListener("click", () => {
  const exts = document.getElementById("exts").value.split(/\s+/).filter(Boolean);
  chrome.storage.local.set({
    enabled: document.getElementById("enabled").checked,
    intercept: document.getElementById("intercept").checked,
    panel: document.getElementById("panel").checked,
    exts
  }, () => {
    document.getElementById("status").textContent = "Saved.";
    setTimeout(() => (document.getElementById("status").textContent = ""), 2000);
  });
});

document.getElementById("ping").addEventListener("click", () => {
  chrome.runtime.sendMessage({ type: "ping" }, (r) => {
    document.getElementById("status").textContent =
      r && r.ok ? "Connected to Open Download Manager " + (r.version || "") : "Not connected.";
  });
});
