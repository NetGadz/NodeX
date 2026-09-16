# NodeX Windows Clean Uninstallation Script
Write-Host "==> Cleaning NodeX Application & Local Data..." -ForegroundColor Yellow

$AppDataNodeX = "$env:APPDATA\NodeX"
if (Test-Path $AppDataNodeX) {
    Remove-Item -Recurse -Force $AppDataNodeX
    Write-Host "[OK] Cleaned $AppDataNodeX" -ForegroundColor Green
}

$DesktopLnk = "$env:USERPROFILE\Desktop\NodeX.lnk"
if (Test-Path $DesktopLnk) {
    Remove-Item -Force $DesktopLnk
    Write-Host "[OK] Cleaned Desktop Shortcut" -ForegroundColor Green
}

Write-Host "[SUCCESS] NodeX uninstalled successfully." -ForegroundColor Green
