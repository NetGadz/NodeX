# Automated Kademlia DHT Cluster Demonstration Script (PowerShell)

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "    Kademlia DHT Automated Demo Cluster    " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Build binary first
Write-Host "[DEMO] Building project in release mode..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Cargo build failed!" -ForegroundColor Red
    exit 1
}

$exe = ".\target\release\kademlia-dht.exe"

# Start Node 1 (Bootstrap)
Write-Host "[DEMO] Starting Node 1 (Bootstrap) on port 9001..." -ForegroundColor Green
$node1 = Start-Process -FilePath $exe -ArgumentList "--port 9001 --state-file demo_state1.json" -PassThru -NoNewWindow

Start-Sleep -Seconds 2

# Start Node 2
Write-Host "[DEMO] Starting Node 2 on port 9002 (bootstrapping to 9001)..." -ForegroundColor Green
$node2 = Start-Process -FilePath $exe -ArgumentList "--port 9002 --bootstrap 127.0.0.1:9001 --state-file demo_state2.json" -PassThru -NoNewWindow

Start-Sleep -Seconds 2

# Start Node 3
Write-Host "[DEMO] Starting Node 3 on port 9003 (bootstrapping to 9002)..." -ForegroundColor Green
$node3 = Start-Process -FilePath $exe -ArgumentList "--port 9003 --bootstrap 127.0.0.1:9002 --state-file demo_state3.json" -PassThru -NoNewWindow

Start-Sleep -Seconds 2

Write-Host "[DEMO] 3-node P2P cluster is running!" -ForegroundColor Cyan
Write-Host "[DEMO] Running test suite to verify cluster operations..." -ForegroundColor Yellow

cargo test --test integration_test

Write-Host "[DEMO] Stopping cluster processes..." -ForegroundColor Yellow
Stop-Process -Id $node1.Id -ErrorAction SilentlyContinue
Stop-Process -Id $node2.Id -ErrorAction SilentlyContinue
Stop-Process -Id $node3.Id -ErrorAction SilentlyContinue

Write-Host "[DEMO] Demo completed successfully!" -ForegroundColor Green
