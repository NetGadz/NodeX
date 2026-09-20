# NodeX Multi-Node Local Cluster Spawner
param(
    [int]$Count = 4
)

Write-Host "==> Spawning $Count local Kademlia DHT nodes..." -ForegroundColor Cyan

# Root seed node
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cargo run -p nodex-cli -- --port 8000"
Start-Sleep -Seconds 1

for ($i = 1; $i -lt $Count; $i++) {
    $port = 8000 + $i
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cargo run -p nodex-cli -- --port $port --bootstrap 127.0.0.1:8000"
    Start-Sleep -Milliseconds 300
}

Write-Host "[SUCCESS] $Count nodes spawned on ports 8000..$(8000 + $Count - 1)" -ForegroundColor Green
