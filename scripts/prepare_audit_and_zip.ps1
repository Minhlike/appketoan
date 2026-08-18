$ErrorActionPreference = "Continue"

$repoRoot = "D:\appketoan"
$auditDir = Join-Path $repoRoot "audit"
$zipPathV4 = "D:\appketoan-audit-v4.zip"
$zipPath = "D:\appketoan-audit.zip"
$stageDir = "D:\temp_audit_stage"

Write-Host "=== 1. Preparing audit directory: $auditDir ==="
if (-not (Test-Path $auditDir)) {
    New-Item -ItemType Directory -Path $auditDir -Force | Out-Null
}

$env:PATH = "C:\Program Files\Git\cmd;D:\DevTools\w64devkit\bin;C:\Users\Acer\.cargo\bin;D:\DevTools\npm-global;" + $env:PATH

# 1. audit/commit.txt
Write-Host "-> Generating audit/commit.txt"
Set-Location $repoRoot
$commitHash = & git rev-parse HEAD
$commitInfo = & git log -1 --pretty=fuller
Set-Content -Path (Join-Path $auditDir "commit.txt") -Value "COMMIT_HASH: $commitHash`n`n$commitInfo" -Encoding UTF8

# 2. audit/git-status.txt
Write-Host "-> Generating audit/git-status.txt"
$gitStatus = & git status
Set-Content -Path (Join-Path $auditDir "git-status.txt") -Value $gitStatus -Encoding UTF8

# 3. audit/git-diff-stat.txt
Write-Host "-> Generating audit/git-diff-stat.txt"
$diffDetail = & git diff --stat HEAD
Set-Content -Path (Join-Path $auditDir "git-diff-stat.txt") -Value "=== GIT DIFF STAT ===`n$diffDetail" -Encoding UTF8

# 4. audit/test-results.txt
Write-Host "-> Running tests and generating audit/test-results.txt"
Set-Location $repoRoot
$cargoTestOutput = & cargo test --workspace -- --nocapture 2>&1 | Out-String

Set-Location $repoRoot
$npmTestOutput = & npm test 2>&1 | Out-String

$testResults = @"
================================================================================
AUDIT TEST SUITE EXECUTION RESULTS - FIX V4 MULTI-SOURCE CORRECTNESS
Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
================================================================================

--- 1. RUST CORE RECONCILIATION TEST SUITE (cargo test --workspace) ---
$cargoTestOutput

--- 2. FRONTEND TEST SUITE (npm test / vitest) ---
$npmTestOutput
"@
Set-Content -Path (Join-Path $auditDir "test-results.txt") -Value $testResults -Encoding UTF8

# 5. audit/build-results.txt
Write-Host "-> Running build and generating audit/build-results.txt"
Set-Location $repoRoot
$npmBuildOutput = & npm run build 2>&1 | Out-String
$tauriCheckOutput = & cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | Out-String

$buildResults = @"
================================================================================
AUDIT BUILD VERIFICATION RESULTS - FIX V4
Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
================================================================================

--- 1. FRONTEND PRODUCTION BUNDLE (npm run build) ---
$npmBuildOutput

--- 2. TAURI RUST PRODUCTION CRATE CHECK ---
$tauriCheckOutput
"@
Set-Content -Path (Join-Path $auditDir "build-results.txt") -Value $buildResults -Encoding UTF8

Write-Host "=== 2. Staging files into $stageDir ==="
if (Test-Path $stageDir) {
    Remove-Item -Path $stageDir -Recurse -Force
}
New-Item -ItemType Directory -Path $stageDir -Force | Out-Null

$excludeFolders = @(
    "node_modules",
    "target",
    "src-tauri\target",
    "dist",
    "releases",
    ".git"
)

$excludeExtensions = @(
    ".exe",
    ".msi",
    ".pdb",
    ".zip",
    ".lock",
    ".xlsx",
    ".xls",
    ".xlsb"
)

$files = Get-ChildItem -Path $repoRoot -Recurse -File

$includedCount = 0
foreach ($file in $files) {
    $relPath = $file.FullName.Substring($repoRoot.Length + 1)
    
    # Check folder exclusion
    $skip = $false
    foreach ($exFolder in $excludeFolders) {
        if ($relPath.StartsWith("$exFolder\") -or $relPath.Contains("\$exFolder\")) {
            $skip = $true
            break
        }
    }
    
    # Check extension exclusion
    if (-not $skip) {
        $ext = $file.Extension.ToLower()
        if ($excludeExtensions -contains $ext) {
            $skip = $true
        }
    }

    if (-not $skip) {
        $targetFilePath = Join-Path $stageDir $relPath
        $targetFileDir = Split-Path -Path $targetFilePath -Parent
        if (-not (Test-Path $targetFileDir)) {
            New-Item -ItemType Directory -Path $targetFileDir -Force | Out-Null
        }
        Copy-Item -Path $file.FullName -Destination $targetFilePath -Force
        $includedCount++
    }
}

Write-Host "Total staged files: $includedCount"

Write-Host "=== 3. Compressing staged files into $zipPathV4 and $zipPath ==="
if (Test-Path $zipPathV4) { Remove-Item -Path $zipPathV4 -Force }
if (Test-Path $zipPath) { Remove-Item -Path $zipPath -Force }

Compress-Archive -Path "$stageDir\*" -DestinationPath $zipPathV4 -CompressionLevel Optimal
Copy-Item -Path $zipPathV4 -Destination $zipPath -Force

# Cleanup stage directory
Remove-Item -Path $stageDir -Recurse -Force

Write-Host "=== 4. Validating Created ZIP Archive ==="
$zipInfo = Get-Item $zipPathV4
$zipSizeKB = [math]::Round($zipInfo.Length / 1KB, 2)
$zipSizeMB = [math]::Round($zipInfo.Length / 1MB, 2)

Write-Host "ZIP Path: $zipPathV4"
Write-Host "ZIP Size: $zipSizeKB KB ($zipSizeMB MB)"

# Validate archive contents
Add-Type -AssemblyName System.IO.Compression.FileSystem
$verifyZip = [System.IO.Compression.ZipFile]::OpenRead($zipPathV4)
$entryNames = $verifyZip.Entries | ForEach-Object { $_.FullName }
$verifyZip.Dispose()

$hasAuditCommit = ($entryNames | Where-Object { $_ -like "*commit.txt" })
$hasAuditStatus = ($entryNames | Where-Object { $_ -like "*git-status.txt" })
$hasAuditDiff = ($entryNames | Where-Object { $_ -like "*git-diff-stat.txt" })
$hasAuditTest = ($entryNames | Where-Object { $_ -like "*test-results.txt" })
$hasAuditBuild = ($entryNames | Where-Object { $_ -like "*build-results.txt" })
$hasCoreLib = ($entryNames | Where-Object { $_ -like "*crates/reconciliation-core/src/lib.rs" -or $_ -like "*reconciliation-core\src\lib.rs" })
$hasMatcherEngine = ($entryNames | Where-Object { $_ -like "*crates/reconciliation-core/src/matcher/engine.rs" -or $_ -like "*reconciliation-core\src\matcher\engine.rs" })
$hasFrontendApp = ($entryNames | Where-Object { $_ -like "*src/App.tsx" -or $_ -like "*src\App.tsx" })
$hasRealExcel = ($entryNames | Where-Object { $_ -like "*.xlsx" -or $_ -like "*.xls" })

Write-Host "Verification Checks:"
Write-Host " - audit/commit.txt: $($hasAuditCommit -ne $null)"
Write-Host " - audit/git-status.txt: $($hasAuditStatus -ne $null)"
Write-Host " - audit/git-diff-stat.txt: $($hasAuditDiff -ne $null)"
Write-Host " - audit/test-results.txt: $($hasAuditTest -ne $null)"
Write-Host " - audit/build-results.txt: $($hasAuditBuild -ne $null)"
Write-Host " - crates/reconciliation-core/src/lib.rs: $($hasCoreLib -ne $null)"
Write-Host " - crates/reconciliation-core/src/matcher/engine.rs: $($hasMatcherEngine -ne $null)"
Write-Host " - src/App.tsx: $($hasFrontendApp -ne $null)"
Write-Host " - No confidential Excel in zip: $($hasRealExcel -eq $null)"

if ($hasAuditCommit -and $hasAuditStatus -and $hasAuditDiff -and $hasAuditTest -and $hasAuditBuild -and $hasCoreLib -and $hasMatcherEngine -and $hasFrontendApp -and ($hasRealExcel -eq $null)) {
    Write-Host "SAFE_FOR_INDEPENDENT_AUDIT: YES"
} else {
    Write-Host "SAFE_FOR_INDEPENDENT_AUDIT: NO"
}
