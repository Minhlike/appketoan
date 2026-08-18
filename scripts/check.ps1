$ErrorActionPreference = "Stop"
$env:PATH = "D:\DevTools\w64devkit\bin;C:\Users\Acer\.cargo\bin;" + $env:PATH

Write-Host "=== 1/3 Checking Frontend (TypeScript) ===" -ForegroundColor Cyan
npm run check

Write-Host "`n=== 2/3 Checking Core Engine (Rust) ===" -ForegroundColor Cyan
Set-Location -Path "D:\appketoan\crates\reconciliation-core"
cargo check

Write-Host "`n=== 3/3 Checking Desktop Shell (Tauri) ===" -ForegroundColor Cyan
Set-Location -Path "D:\appketoan\src-tauri"
cargo check

Set-Location -Path "D:\appketoan"
Write-Host "`n[PASS] All checks completed successfully!" -ForegroundColor Green
