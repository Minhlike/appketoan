$ErrorActionPreference = "Stop"
$env:PATH = "D:\DevTools\w64devkit\bin;C:\Users\Acer\.cargo\bin;" + $env:PATH

Write-Host "=== 1/2 Building Frontend Bundle (Vite) ===" -ForegroundColor Cyan
npm run build

Write-Host "`n=== 2/2 Building Backend (Rust) ===" -ForegroundColor Cyan
Set-Location -Path "D:\appketoan\src-tauri"
cargo build

Set-Location -Path "D:\appketoan"
Write-Host "`n[PASS] Build completed successfully!" -ForegroundColor Green
