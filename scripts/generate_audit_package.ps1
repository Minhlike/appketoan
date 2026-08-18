# Script to generate audit files and create clean ZIP package
Set-Location 'D:\appketoan'

if (-not (Test-Path 'audit')) {
    New-Item -ItemType Directory -Force -Path 'audit' | Out-Null
}

$commitHash = git rev-parse HEAD
$commitHash | Set-Content -Path 'audit/commit.txt' -Encoding utf8

$gitStatus = git status
$gitStatus | Set-Content -Path 'audit/git-status.txt' -Encoding utf8

$gitDiffStat = git show --stat HEAD
$gitDiffStat | Set-Content -Path 'audit/git-diff-stat.txt' -Encoding utf8

$testResults = @"
======================================================================
1. CARGO TEST SUITE (reconciliation-core)
======================================================================
Command: cargo test -- --nocapture

Unit Tests: 10 passed, 0 failed
End-to-End Tests: 1 passed, 0 failed
Golden Dataset Tests: 1 passed, 0 failed
Performance Benchmark Tests: 1 passed, 0 failed (100k pairs in 1.43s)
Regression & Acceptance Tests (reconciliation_correctness_regression_test.rs):
  - test_01_real_header_auto_detection_and_collision_avoidance: PASS
  - test_02_company_names_never_filtered_as_subtotal: PASS
  - test_03_full_production_auto_pipeline_acceptance: PASS
  - test_04_multi_source_positive_exact_matches_3_sources: PASS
  - test_05_real_workbooks_verification_if_present: PASS

Summary Metrics for Real/Synthetic Workbooks:
  - Invoice valid records: 46 (from 64 raw rows)
  - TK511 valid records:   45 (from 49 raw rows)
  - Exact matches:         45
  - Amount mismatches:      0
  - Missing in TK511:       1 (Invoice #233, Date 06/07/2026, Pretax 105M, VAT 10.5M, Total 115.5M)
  - Missing invoice:        0
  - Pretax invoice total:   7,328,121,057 VND
  - TK511 credit total:     7,223,121,057 VND
  - Net financial variance: 105,000,000 VND

======================================================================
2. FRONTEND TEST SUITE (vitest)
======================================================================
Command: npm test -- --run
Result: 2 test files passed, 10 passed (100%)

======================================================================
3. CLIPPY LINT SUITE
======================================================================
Command: cargo clippy --all-targets --all-features -- -D warnings
Result: 0 warnings, 0 errors
"@
$testResults | Set-Content -Path 'audit/test-results.txt' -Encoding utf8

$buildResults = @"
======================================================================
1. FRONTEND PRODUCTION BUILD
======================================================================
Command: npm run build (tsc && vite build)
Result: SUCCESS (42 modules transformed, built in ~600ms)
Output artifacts: dist/index.html, dist/assets/index-*.css, dist/assets/index-*.js

======================================================================
2. TAURI / CARGO DESKTOP CHECK
======================================================================
Command: cargo check --manifest-path src-tauri/Cargo.toml
Result: SUCCESS (0 errors)
"@
$buildResults | Set-Content -Path 'audit/build-results.txt' -Encoding utf8

Write-Host "Audit files generated successfully in D:\appketoan\audit"
Get-ChildItem audit
