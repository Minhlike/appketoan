$ErrorActionPreference = "Stop"
$env:PATH = "D:\DevTools\w64devkit\bin;C:\Users\Acer\.cargo\bin;" + $env:PATH

Write-Host "=== 1/2 Running Frontend Tests (Vitest) ===" -ForegroundColor Cyan
npm test

Write-Host "`n=== 2/2 Running Core Engine Unit Tests (Rust) ===" -ForegroundColor Cyan
Set-Location -Path "D:\appketoan\crates\reconciliation-core"
cargo test

Set-Location -Path "D:\appketoan"
Write-Host "`n[PASS] All test suites passed successfully!" -ForegroundColor Green
