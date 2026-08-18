$ErrorActionPreference = "Stop"

$destinationZip = "D:\appketoan-source.zip"

Write-Host "=== 1. Checking ZIP Existence & Size ===" -ForegroundColor Cyan
if (-not (Test-Path $destinationZip)) {
    Write-Error "ZIP file not found at $destinationZip"
    exit 1
}

$zipFileInfo = Get-Item -LiteralPath $destinationZip
$sizeBytes = [int64]$zipFileInfo.Length
$sizeKB = [math]::Round($sizeBytes / 1024, 2)
$sizeMB = [math]::Round($sizeBytes / 1048576, 2)
Write-Host "ZIP Path: $($zipFileInfo.FullName)" -ForegroundColor Green
Write-Host "ZIP Size: $sizeKB KB ($sizeMB MB) - $sizeBytes bytes" -ForegroundColor Green

Write-Host "`n=== 2. Inspecting ZIP Contents ===" -ForegroundColor Cyan
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($destinationZip)

$totalEntries = $zip.Entries.Count
Write-Host "Total Files in ZIP: $totalEntries"

$topLevelDirs = @{}
$violations = @()
$keySourceFiles = @(
    "AGENTS.md",
    "README.md",
    "package.json",
    "package-lock.json",
    "tsconfig.json",
    "vite.config.ts",
    "Cargo.toml",
    "src/App.tsx",
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "src-tauri/src/main.rs",
    "crates/reconciliation-core/Cargo.toml",
    "crates/reconciliation-core/src/lib.rs"
)

$foundKeyFiles = @{}
foreach ($k in $keySourceFiles) {
    $foundKeyFiles[$k] = $false
}

foreach ($entry in $zip.Entries) {
    $path = $entry.FullName.Replace('\', '/')
    
    # Check top level
    $parts = $path.Split('/')
    if ($parts.Length -gt 1) {
        $topLevelDirs[$parts[0]] = $true
    } else {
        $topLevelDirs["(root files)"] = $true
    }

    # Check key files
    if ($foundKeyFiles.ContainsKey($path)) {
        $foundKeyFiles[$path] = $true
    }

    # Verify exclusions
    if ($path -like "*target/*" -or $path -like "target/*") {
        $violations += "Target build artifact found: $path"
    }
    if ($path -like "*node_modules/*" -or $path -like "node_modules/*") {
        $violations += "Node modules found: $path"
    }
    if ($path -like "*.git/*" -or $path -like ".git/*") {
        $violations += "Git internal found: $path"
    }
    if ($path -like "*dist/*" -or $path -like "dist/*") {
        $violations += "Dist build artifact found: $path"
    }
    if ($path -like "*releases/*.exe" -or $path -like "*releases/*.msi") {
        $violations += "Release binary found: $path"
    }
    if ($path -like "*.xlsx" -or $path -like "*.xls" -or $path -like "*.xlsm" -or $path -like "*.xlsb") {
        if (-not ($path -like "fixtures/synthetic/*")) {
            $violations += "Potential real Excel workbook found: $path"
        }
    }
}

$zip.Dispose()

Write-Host "`nTop-Level Items in ZIP:" -ForegroundColor Cyan
foreach ($d in $topLevelDirs.Keys | Sort-Object) {
    Write-Host "  - $d"
}

Write-Host "`nKey Essential Source Files Verification:" -ForegroundColor Cyan
$allKeyFilesFound = $true
foreach ($k in $foundKeyFiles.Keys) {
    $found = $foundKeyFiles[$k]
    if ($found) {
        Write-Host "  [PASS] $k" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Missing: $k" -ForegroundColor Red
        $allKeyFilesFound = $false
    }
}

Write-Host "`nExclusion & Hygiene Violations:" -ForegroundColor Cyan
if ($violations.Count -eq 0) {
    Write-Host "  [PASS] Zero violations! All build artifacts, node_modules, git, and workbooks properly excluded." -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Found $($violations.Count) violations:" -ForegroundColor Red
    foreach ($v in $violations) {
        Write-Host "    - $v" -ForegroundColor Red
    }
}

if ($allKeyFilesFound -and $violations.Count -eq 0) {
    Write-Host "`n>>> VERIFICATION SUCCESS: SAFE_TO_UPLOAD: YES <<<" -ForegroundColor Green
} else {
    Write-Host "`n>>> VERIFICATION FAILED: SAFE_TO_UPLOAD: NO <<<" -ForegroundColor Red
}
