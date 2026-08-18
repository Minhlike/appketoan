$ErrorActionPreference = "Stop"
$env:PATH = "D:\DevTools\w64devkit\bin;C:\Users\Acer\.cargo\bin;" + $env:PATH

Write-Host "=== 1/2 Checking Frontend (TypeScript) ===" -ForegroundColor Cyan
npm run check

Write-Host "`n=== 2/2 Checking Backend (Rust) ===" -ForegroundColor Cyan
Set-Location -Path "D:\appketoan\src-tauri"
cargo check

Set-Location -Path "D:\appketoan"
Write-Host "`n[PASS] All baseline checks passed successfully!" -ForegroundColor Green
