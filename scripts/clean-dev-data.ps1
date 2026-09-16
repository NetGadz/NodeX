# Clean all local database and temporary test files
Write-Host "==> Cleaning local development state and database files..." -ForegroundColor Yellow
Remove-Item -Force -ErrorAction SilentlyContinue messenger_db_*.json
Remove-Item -Force -ErrorAction SilentlyContinue nodex_*state*.json
Remove-Item -Force -ErrorAction SilentlyContinue *.log
Write-Host "[OK] Local development files cleaned." -ForegroundColor Green
