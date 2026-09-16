# NodeX Windows Packaging Script
param(
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"
Write-Host "==> Building NodeX Release Binary..." -ForegroundColor Cyan
cargo build --release -p nodex-gui

$ReleaseExe = "target\release\nodex.exe"
if (-not (Test-Path $ReleaseExe)) {
    # Check if target is redirected
    $ReleaseExe = "$env:USERPROFILE\.cargo_target\kademlia_dht\release\nodex.exe"
}

$DistDir = "dist\nodex-v$Version-windows-x64"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

Copy-Item $ReleaseExe -Destination "$DistDir\nodex.exe"
Copy-Item "nodex-gui\assets\logo.png" -Destination "$DistDir\logo.png"
Copy-Item "config\default.json" -Destination "$DistDir\config.json"
Copy-Item "readme.md" -Destination "$DistDir\README.txt"

Write-Host "==> Creating ZIP distribution package..." -ForegroundColor Cyan
Compress-Archive -Path "$DistDir\*" -DestinationPath "dist\NodeX-v$Version-windows-x64.zip" -Force

Write-Host "[SUCCESS] NodeX Windows package built at dist/NodeX-v$Version-windows-x64.zip" -ForegroundColor Green
