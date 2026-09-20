# Build NodeX Release Executable
Write-Host "==> Compiling NodeX in Release Mode..." -ForegroundColor Cyan
cargo build --release -p nodex-gui -p nodex-cli

Write-Host "[SUCCESS] Binaries built successfully in target/release/" -ForegroundColor Green
