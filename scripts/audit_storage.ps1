$ErrorActionPreference = "SilentlyContinue"

function Get-FolderSize($path) {
    if (Test-Path $path) {
        $measure = Get-ChildItem -Path $path -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum
        if ($measure -and $measure.Sum) {
            return [int64]$measure.Sum
        }
    }
    return [int64]0
}

function Format-Size([int64]$bytes) {
    if ($bytes -ge 1GB) { return "{0:N2} GB" -f ($bytes / 1GB) }
    if ($bytes -ge 1MB) { return "{0:N2} MB" -f ($bytes / 1MB) }
    if ($bytes -ge 1KB) { return "{0:N2} KB" -f ($bytes / 1KB) }
    return "$bytes B"
}

Write-Host "=== 1. Top-Level Directories in D:\appketoan ===" -ForegroundColor Cyan
$topDirs = Get-ChildItem -Path "D:\appketoan" -Directory -Force
$results = @()
foreach ($d in $topDirs) {
    $sz = Get-FolderSize $d.FullName
    $results += [PSCustomObject]@{
        Path = $d.FullName
        SizeBytes = $sz
        Size = Format-Size $sz
    }
}
$results | Sort-Object SizeBytes -Descending | Format-Table Path, Size -AutoSize

Write-Host "`n=== 2. Directories in src-tauri ===" -ForegroundColor Cyan
$tauriDirs = Get-ChildItem -Path "D:\appketoan\src-tauri" -Directory -Force
$tauriResults = @()
foreach ($d in $tauriDirs) {
    $sz = Get-FolderSize $d.FullName
    $tauriResults += [PSCustomObject]@{
        Path = $d.FullName
        SizeBytes = $sz
        Size = Format-Size $sz
    }
}
$tauriResults | Sort-Object SizeBytes -Descending | Format-Table Path, Size -AutoSize

Write-Host "`n=== 3. Breakdown of src-tauri\target ===" -ForegroundColor Cyan
if (Test-Path "D:\appketoan\src-tauri\target") {
    $targetDirs = Get-ChildItem -Path "D:\appketoan\src-tauri\target" -Directory -Force
    $targetResults = @()
    foreach ($d in $targetDirs) {
        $sz = Get-FolderSize $d.FullName
        $targetResults += [PSCustomObject]@{
            Path = $d.FullName
            SizeBytes = $sz
            Size = Format-Size $sz
        }
    }
    $targetResults | Sort-Object SizeBytes -Descending | Format-Table Path, Size -AutoSize
}

Write-Host "`n=== 4. Breakdown of crates\reconciliation-core\target ===" -ForegroundColor Cyan
if (Test-Path "D:\appketoan\crates\reconciliation-core\target") {
    $coreTargetDirs = Get-ChildItem -Path "D:\appketoan\crates\reconciliation-core\target" -Directory -Force
    $coreResults = @()
    foreach ($d in $coreTargetDirs) {
        $sz = Get-FolderSize $d.FullName
        $coreResults += [PSCustomObject]@{
            Path = $d.FullName
            SizeBytes = $sz
            Size = Format-Size $sz
        }
    }
    $coreResults | Sort-Object SizeBytes -Descending | Format-Table Path, Size -AutoSize
}

Write-Host "`n=== 5. Specific Folders of Interest ===" -ForegroundColor Cyan
$specialPaths = @(
    "D:\appketoan\node_modules",
    "D:\appketoan\.git",
    "D:\appketoan\dist",
    "D:\appketoan\src-tauri\target\debug",
    "D:\appketoan\src-tauri\target\release",
    "D:\appketoan\src-tauri\target\release\bundle",
    "D:\appketoan\src-tauri\target\release\incremental",
    "D:\appketoan\src-tauri\target\debug\incremental",
    "D:\appketoan\crates\reconciliation-core\target\debug",
    "D:\appketoan\crates\reconciliation-core\target\debug\incremental"
)

$specialResults = @()
foreach ($p in $specialPaths) {
    if (Test-Path $p) {
        $sz = Get-FolderSize $p
        $specialResults += [PSCustomObject]@{
            Path = $p
            SizeBytes = $sz
            Size = Format-Size $sz
        }
    }
}
$specialResults | Sort-Object SizeBytes -Descending | Format-Table Path, Size -AutoSize

Write-Host "`n=== 6. Total Repository Size ===" -ForegroundColor Cyan
$totalBytes = Get-FolderSize "D:\appketoan"
Write-Host ("Total D:\appketoan Size: " + (Format-Size $totalBytes) + " (" + $totalBytes + " bytes)") -ForegroundColor Yellow
