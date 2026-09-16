# NodeX 2-Node Live Demo Launcher
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "        Launching NodeX 2-Node P2P Demo               " -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# Start Node 1 (Alice)
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cargo run -p nodex-gui -- --port 8000 --name 'Alice'"
Start-Sleep -Seconds 2

# Start Node 2 (Bob, connected to Alice)
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cargo run -p nodex-gui -- --port 8001 --bootstrap 127.0.0.1:8000 --name 'Bob'"

Write-Host "[OK] Alice and Bob nodes spawned in separate windows." -ForegroundColor Green
