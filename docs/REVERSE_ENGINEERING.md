# Reverse-engineering notes — IDM 6.43 build 18

Everything below was derived from the copy installed on this machine and is
reproducible from the commands in the last section.

## 1. Target identity

| Fact | Value |
|---|---|
| Path | `C:\Program Files (x86)\Internet Download Manager\IDMan.exe` |
| Version | 6.43.18.2 (registry `DisplayVersion 6.43.18`, `idmvers v6.43b18 Trial`) |
| Size | 6,517,616 bytes |
| SHA-256 | `a705c0154fafc7cd509decf165cb6f6c050b6521b40adb6f730dd827c86051c7` |
| Format | PE32, x86, GUI subsystem, MSVC, not packed (file entropy 6.42) |
| Functions analysed | 20,680 (Ghidra 12.1.4 auto-analysis) |
| Imported DLLs / functions | 14 / 520 (+111 delay imports) |
| Resources | 436 (16 dialog templates, 174 bitmaps, icon groups, 505 PNG) |

REA evidence: `open_binary`, `inspect_pe_imports`, `inspect_pe_resources`,
`inspect_byte_entropy` (exported to `rea-evidence.json`).

## 2. Toolchain

* **REA 6.8.0** MCP server — PE parsing, resource inventory, entropy, evidence
  bundle. `rea doctor` initially reported the bundled Ghidra as unavailable
  because no 64-bit JDK was on the service PATH; the MCP server process could
  not pick up a newly installed JDK, and Ghidra's own profile writes under
  `%APPDATA%` were denied in this session. PE/resource/entropy analysis through
  REA worked throughout.
* **Ghidra 12.1.4** (bundled with REA) — headless auto-analysis and decompile of
  all functions, run with `JAVA_HOME` set to Temurin 21 and `APPDATA` redirected
  to a writable folder. Output: `D:\dsh\re\pseudo\` (20,680 `.c` files,
  `index.tsv`, `strings.tsv`).
* Installed artefacts: `Languages\template.lng` (2,147 lines of UI text),
  `tips.txt`, registry `HKCU\Software\DownloadManager`, `%APPDATA%\IDM`
  (scheduler `s_1.dt`, categories, list settings).

## 3. What the binary shows

### 3.1 HTTP engine is range-based and multi-connection

Request builder at `0x0058b2b0` emits literal range headers:

```
0058b2b0.c:286  "Range: bytes=%I64d-%I64d\r\n"
0058b2b0.c:299  "Range: bytes=%I64d-\r\n"
0058b2b0.c:324  "Range: bytes=%I64d-%I64d\r\n"   (optional connection range)
0058b2b0.c:338  "Range: bytes=0-%I64d\r\n"
0058b2b0.c:357  "Range: bytes=0-\r\n"
```

A second builder at `0x005adcd0` contains the same set (HTTPS/proxy path), and
`0x00581cf0`, `0x005827f0`, `0x0058c770` parse `Content-Range:` replies. The
request template itself is visible at `0x007127d0`:

```
%s %s HTTP/1.1\r\nHost: %s%s\r\nUser-Agent: %s\r\nAccept: %s\r\n...
```

with a CONNECT variant at `0x00712db4`, `Proxy-Authorization: Basic/Digest`
templates at `0x00712df4`/`0x00712e18`, and `Cookie:`/`Referer:` headers in the
same block. This matches the clone's `httpmeta.rs` and per-part `Range` loop.

### 3.2 Connection count: default 8, per-server exceptions

`0x004ff150` (class `CMaxConnNumbers`) reads registry value
`MaxConnectionsNumber`; when absent it stores the immediate `0x08`:

```
004ff150.c:43  RegQueryValueExA(..., "MaxConnectionsNumber", ...)
004ff150.c:45  if (LVar2 != 0) { lpData[0] = '\b'; ... }      // 8
004ff150.c:51  RegCreateKeyExA(..., "Software\\DownloadManager\\MCN", ...)
```

`0x004fe830` writes the same value back. The `MCN` subkey enumerates per-server
overrides. Runtime “optional connections” are confirmed by the error string
`HTTPSession::TryStartNewOptionalConnections()` at `0x007137c0` and
`FTPSession::CreateOptionalConnections()` at `0x007116f8`. The clone uses the
same default of 8, a 1–32 clamp, `connection_exceptions`, and a minimum 512 KiB
per part.

### 3.3 Part files, temp folder, resume

`TempPath` is read/written throughout the configuration code (`0x0044c9e0`,
`0x0046e6f0`, `0x0050e720`, …). Parts are written under the temporary folder,
then assembled and moved to “Save To”, which is why IDM survives a restart with
partial data. The clone implements this with `temp\<id>\partN` plus a JSON part
map (position saved every 3 s) and joins on completion.

### 3.4 Browser integration

The binary registers native-messaging hosts named `com.tonec.idm`:

```
006dd41c  SOFTWARE\Google\Chrome\NativeMessagingHosts
006dd47c  SOFTWARE\Google\Chrome\NativeMessagingHosts\com.tonec.idm
006dd3f4  SOFTWARE\Mozilla\NativeMessagingHosts
006dd448  SOFTWARE\Mozilla\NativeMessagingHosts\com.tonec.idm
006de21c  SOFTWARE\Microsoft\Edge\NativeMessagingHosts\com.tonec.idm
006de1bc  SOFTWARE\CocCoc\Browser\NativeMessagingHosts\com.tonec.idm
```

The shipped `IDMMsgHost.json`, `idmmzcc*.xpi`, `IDMEdgeExt.crx` and
`idmbrbtn*.dll` are the other halves of that integration. Open Download Manager uses the same
native-messaging model (`com.opendm.host`, HKCU installation, 4-byte length frames)
and ships a Manifest V3 extension with link/media context menus, takeover of
`chrome.downloads`, cookie forwarding and an in-page panel.

### 3.5 UI and localisation

`Languages\template.lng` is the complete English UI catalogue: main-window
columns (strings 20369–20375), toolbar (20376–20383), Add URL (148), Download
File Info (130), Download progress (103), Options (134, tab names 20286–20306),
Scheduler (177/308), Batch (280), Site Grabber (241/262), menus (32771–34046),
error messages (20000–21236) and status strings. The clone uses these exact
labels, the same column set, category names and default folders observed in
`HKCU\Software\DownloadManager\FoldersTree` (Programs/Music/Video/Documents/
Compressed with IDM's masks), the same 57-extension capture list, and the same
`ListSettings` column widths/order.

Registry facts used for defaults:

* `Extensions = 3GP 7Z AAC ... ZIP` (57 entries) → `Config::file_types`
* `FoldersTree\*\mask` and `pathW` → category masks/default folders
* `ListSettings` → column widths/visibility (`FileName 150`, `Description 200`, …)
* `F5`-style refresh of `ConfigTime`, `windowPlacementV6` (1219×679 observed)

## 4. Behaviour → Rust mapping

| IDM behaviour (evidence) | Open Download Manager implementation |
|---|---|
| `Range: bytes=a-b` per connection (`0x58b2b0`) | `engine::fetch_part` |
| `Content-Range` probe/validation (`0x581cf0`) | `httpmeta::probe`, `parse_content_range` |
| Default 8 connections (`0x4ff150`) | `Config::max_connections = 8` |
| Per-server MCN | `Config::connection_exceptions` |
| Optional connections (`0x7137c0`) | connections sized by file size (512 KiB/part) |
| TempPath + assembly | `temp/<id>/partN` + `assemble()` |
| Native messaging | `crates/odm-host`, `extension/` |
| UI catalogue | `template.lng` strings reproduced in `ui/` |
| Scheduler file `s_1.dt` | `Schedule`/`Queue` in `config.json` |

## 5. Reproduction

```powershell
# REA facts (MCP): open_binary + inspect_pe_imports + inspect_pe_resources
#   + inspect_byte_entropy; export_evidence_bundle -> rea-evidence.json

# Ghidra headless (bundled with REA), JDK 21, writable profile:
$env:JAVA_HOME = 'C:\Program Files\Eclipse Adoptium\jdk-21.0.13.9-hotspot'
$env:APPDATA   = 'D:\dsh\ghappdata'
$env:LOCALAPPDATA = 'D:\dsh\ghlocal'
$env:JAVA_TOOL_OPTIONS = '-Duser.home=D:\dsh\ghhome -Djava.io.tmpdir=D:\dsh\ghtmp'
& '<rea>\vendor\ghidra\support\analyzeHeadless.bat' D:\dsh\re\ghproj idmfull `
  -import D:\dsh\re\IDMan.exe -scriptPath D:\dsh\re\scripts -postScript DumpAll.java
```

Evidence locations: `D:\dsh\re\pseudo\` (20,680 files), `strings.tsv`,
`index.tsv`, `rea-evidence.json`, `ghidra_full.log`, `idm-main.png`.

## 6. Limitations / not analysed

* No dynamic instrumentation of IDM (no Frida/GDB runs); the protocol was
  reconstructed statically and validated against the Rust implementation.
* Registration/activation, update signing, VPN driver and media muxing were not
  reverse-engineered; the clone deliberately has no licensing code and does not
  patch or bypass IDM itself.
* FTP, RTSP/stream capture and browser-specific COM/BHO paths are approximated
  or documented as not implemented.
