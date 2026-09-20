# Run All NodeX Unit and Integration Tests
Write-Host "==> Running Full Test Suite Across All Workspace Crates..." -ForegroundColor Cyan
cargo test --workspace -- --nocapture

if ($LASTEXITCODE -eq 0) {
    Write-Host "[SUCCESS] All tests passed cleanly!" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Some tests failed." -ForegroundColor Red
}
