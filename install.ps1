# Install Open Download Manager for the current user and install browser integration.
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$src = Join-Path $root "target\release"
if (-not (Test-Path (Join-Path $src "OpenDownloadManager.exe"))) {
    throw "Build first: .\build.ps1"
}
$dest = Join-Path $env:LOCALAPPDATA "Programs\Open Download Manager"
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item -Force (Join-Path $src "OpenDownloadManager.exe") $dest
Copy-Item -Force (Join-Path $src "odm-native-host.exe") $dest
Copy-Item -Recurse -Force (Join-Path $src "extension") (Join-Path $dest "extension")
if (Test-Path (Join-Path $src "odm.ico")) {
    Copy-Item -Force (Join-Path $src "odm.ico") $dest
}

# Install the native messaging host with Chrome, Edge, CocCoc and Firefox (HKCU).
& (Join-Path $dest "odm-native-host.exe") --install

# Start menu shortcut.
$shell = New-Object -ComObject WScript.Shell
$lnk = Join-Path ([Environment]::GetFolderPath("Programs")) "Open Download Manager.lnk"
$s = $shell.CreateShortcut($lnk)
$s.TargetPath = Join-Path $dest "OpenDownloadManager.exe"
$s.WorkingDirectory = $dest
$icon = Join-Path $dest "odm.ico"
if (Test-Path $icon) { $s.IconLocation = $icon }
$s.Save()

Write-Host "Installed to $dest"
Write-Host "Start menu shortcut: $lnk"
Write-Host ""
Write-Host "Load the browser extension:"
Write-Host "  chrome://extensions -> Developer mode -> Load unpacked -> $dest\extension"
