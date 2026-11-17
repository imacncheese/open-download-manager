# Build the Open Download Manager MSI with the bundled WiX 3.14 toolset.
param(
    [string]$Version = "1.0.0"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$wix = Join-Path $root "tools\wix"
$src = Join-Path $root "dist"
$obj = Join-Path $PSScriptRoot "obj"
$out = Join-Path $src "OpenDownloadManager-$Version.msi"

if (-not (Test-Path (Join-Path $wix "candle.exe"))) {
    throw "WiX not found at $wix"
}
if (-not (Test-Path (Join-Path $src "OpenDownloadManager.exe"))) {
    throw "Build the app first: .\build.ps1"
}
New-Item -ItemType Directory -Force $obj | Out-Null
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot "tmp") | Out-Null
$env:TEMP = Join-Path $PSScriptRoot "tmp"
$env:TMP = $env:TEMP
Get-ChildItem $obj -File | Remove-Item -Force

# Stable GUIDs.
$guids = @{}
Get-Content (Join-Path $PSScriptRoot "guids.txt") | ForEach-Object {
    if ($_ -match '^\s*([^=]+)=(.*)$') { $guids[$matches[1].Trim()] = $matches[2].Trim() }
}

# Harvest the extension folder into components.
& (Join-Path $wix "heat.exe") dir (Join-Path $src "extension") `
    -cg ExtensionComponents -dr ExtensionFolder -gg -g1 -sfrag -srd `
    -var var.ExtensionDir -out (Join-Path $obj "extension.wxs")
if ($LASTEXITCODE -ne 0) { throw "heat failed" }

# Compile.
& (Join-Path $wix "candle.exe") -nologo -arch x64 `
    -dSourceDir="$src" -dExtensionDir="$src\extension" -dProjectDir="$PSScriptRoot" -dVersion="$Version" `
    -dUpgradeGuid="$($guids['UpgradeGuid'])" -dMainGuid="$($guids['MainGuid'])" `
    -dHostGuid="$($guids['HostGuid'])" -dIconGuid="$($guids['IconGuid'])" `
    -dShortcutGuid="$($guids['ShortcutGuid'])" -dArpGuid="$($guids['ArpGuid'])" `
    -out "$obj\" (Join-Path $PSScriptRoot "Product.wxs") (Join-Path $obj "extension.wxs")
if ($LASTEXITCODE -ne 0) { throw "candle failed" }

# Link.
& (Join-Path $wix "light.exe") -nologo -sval -ext WixUIExtension -cultures:en-us `
    -out "$out" (Join-Path $obj "Product.wixobj") (Join-Path $obj "extension.wixobj")
if ($LASTEXITCODE -ne 0) { throw "light failed" }

Write-Host "Built: $out"
Get-Item $out | Select-Object FullName, Length
