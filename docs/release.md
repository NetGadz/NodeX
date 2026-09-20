# NodeX Release & Distribution Guide

## 1. Building Release Binaries
```powershell
# Windows
.\scripts\build-release.ps1

# Package Installer / Zip
.\scripts\package-windows.ps1
```

## 2. Release Checklist
- Run all automated unit and integration tests: `cargo test --workspace`.
- Check all security and crypto tests pass (`cargo test -p nodex-messenger`).
- Verify binary size and absence of debug symbols: `cargo build --release`.
- Test clean installation and uninstallation on clean Windows VM.
