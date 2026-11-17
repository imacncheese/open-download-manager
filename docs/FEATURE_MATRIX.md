# Feature parity matrix — Open Download Manager vs. IDM 6.43

Status of IDM 6.43 features in Open Download Manager. “Parity” means visible behaviour and
defaults match; implementation details differ (egui vs. Win32, reqwest vs.
WinINet/OpenSSL).

## Download engine

| IDM feature | Status | Notes |
|---|---|---|
| HTTP / HTTPS downloads | ✅ parity | reqwest + rustls/native TLS |
| Segmented download, up to 32 connections | ✅ parity | default 8, matches `MaxConnectionsNumber` |
| Dynamic/optional connections | ✅ equivalent | parts sized from file length (≥512 KiB/part) |
| Resume after stop or crash | ✅ parity | part positions persisted every 3 s |
| Resume when server ignores Range | ✅ | restart from zero with clear error |
| Unknown-length / non-resumable streams | ✅ | single connection, EOF-terminated |
| Redirects, cookies, referer, auth | ✅ | per-download cookie jar, Basic auth, captured cookies |
| Content-Disposition filename handling | ✅ | RFC 5987 + percent decoding |
| Last-Modified timestamp preservation | ✅ | `SetFileTime` |
| Duplicate file naming | ✅ | ask / numbered / overwrite / resume modes |
| Speed Limiter | ✅ parity | KB/s token bucket, global + per-download field |
| Proxy: system / manual HTTP / SOCKS | ✅ | PAC is not executed |
| Per-server connection caps (MCN) | ✅ | wildcard exceptions |
| FTP / FTPS | ❌ | UI/config only |
| Live stream (RTSP/HLS) saving | ❌ | media URLs can still be fetched if plain HTTP |
| Video+audio muxing | ❌ | downloads tracks separately |
| Virus scan hook | ⚠️ | configuration stored; scanner not launched |
| Dial-up/VPN dialing | ⚠️ | configuration stored |
| CRC/disk error recovery | ⚠️ | errors surfaced, no block-level retry |

## UI / UX

| IDM feature | Status |
|---|---|
| Main window layout (menu, toolbar, category tree, list, status bar) | ✅ |
| Menu structure: Tasks / File / Downloads / View / Help | ✅ |
| Toolbar: Add URL, Resume, Stop, Stop All, Delete, Options, Scheduler, Start/Stop Queue, Grabber, Speed Limiter, Find | ✅ |
| Large / small / classic toolbar | ✅ |
| Columns: Q, File Name, Status, Size, Time left, Transfer rate, Description, Date Added, Last Try, Save To, Referer | ✅ |
| Sort by any column, arrange menu, column chooser | ✅ |
| Category tree with counts, add/edit/delete categories | ✅ |
| Add URL dialog (auth, save-to, description, category, queue, download later) | ✅ |
| Download progress dialog (per-connection bars, rate, time left) | ✅ |
| Options tabs: General, File Types, Connection, Proxy/Socks, Sites Logins, Dial Up/VPN, Sounds, Downloads, Browser | ✅ |
| Download File Info / start dialog | ⚠️ simplified into Add URL + progress |
| Scheduler & queues dialog, days, limits, completion actions | ✅ |
| Batch download dialog with `*` preview | ✅ |
| Site Grabber window (BFS, filters, checked download) | ✅ basic |
| Find dialog / F3 | ✅ |
| File properties dialog | ✅ read-only view |
| Completion dialog | ✅ |
| Tip of the Day | ✅ (reads installed `tips.txt`) |
| About dialog | ✅ |
| Dark mode | ✅ |
| Tray icon, minimize/close to tray | ✅ |
| Sounds for events | ⚠️ settings only, not played |
| Drag & drop links onto the window | ❌ (file/list drop not wired) |
| Column tooltips, per-IDM bitmaps | ⚠️ icons are vector equivalents |

## Integration

| IDM feature | Status |
|---|---|
| Chrome/Edge/Firefox extension takeover | ✅ native messaging (`com.opendm.host`) |
| Context menus: download link/media/image/all links/selection | ✅ |
| Cookie + referer + UA forwarding | ✅ |
| In-page download panel for media | ✅ (basic) |
| Clipboard URL monitoring with prompt | ✅ |
| “Download all links” with filters | ✅ (extension collects anchors) |
| Advanced network driver / WFP interception | ❌ not installed; extension-based |
| IE BHO / COM integration | ❌ |
| Shell “Download with IDM” context menu | ❌ |
| Import/export link lists | ✅ IDM-style text export |
| Single instance + URL forwarding | ✅ loopback IPC |
| Launch on startup | ✅ setting (Run key wiring via install script not enforced) |
| Update check | ⚠️ message only |

## Deliberate differences

* No registration, serial check, trial counter or activation — by design.
* No telemetry/update downloads; the app never contacts Tonec servers.
* Engine and UI are cross-platform Rust; only Windows-specific features
  (tray, registry, timestamps) are gated.
