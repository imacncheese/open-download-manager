# Build Open Download Manager (release) and stage the extension next to the binaries.
param(
    [string]$Profile = "release"
)
$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    if ($Profile -eq "release") {
        cargo build --release -p odm-app -p odm-host
    } else {
        cargo build -p odm-app -p odm-host
    }
    $out = Join-Path $PSScriptRoot "target\$Profile"
    Copy-Item -Recurse -Force (Join-Path $PSScriptRoot "extension") (Join-Path $out "extension")
    if (Test-Path (Join-Path $PSScriptRoot "assets\odm.ico")) {
        Copy-Item -Force (Join-Path $PSScriptRoot "assets\odm.ico") (Join-Path $out "odm.ico")
    }
    Write-Host "Built: $out\OpenDownloadManager.exe"
} finally {
    Pop-Location
}
