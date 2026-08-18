$ErrorActionPreference = "Stop"

$sourceRoot = "D:\appketoan"
$destinationZip = "D:\appketoan-source.zip"

Write-Host "Creating clean source archive at: $destinationZip" -ForegroundColor Cyan

# Remove old zip if exists
if (Test-Path $destinationZip) {
    Remove-Item $destinationZip -Force
}

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$zipFile = [System.IO.Compression.ZipFile]::Open($destinationZip, [System.IO.Compression.ZipArchiveMode]::Create)

$allFiles = Get-ChildItem -Path $sourceRoot -Recurse -File -Force

$includedCount = 0
$excludedCount = 0

foreach ($file in $allFiles) {
    $relative = $file.FullName.Substring($sourceRoot.Length).TrimStart('\', '/')
    $normalized = $relative.Replace('\', '/')

    # Exclusion rules
    $exclude = $false

    # 1. Target & build directories
    if ($normalized -like "node_modules/*" -or $normalized -eq "node_modules") { $exclude = $true }
    if ($normalized -like "target/*" -or $normalized -eq "target") { $exclude = $true }
    if ($normalized -like "*/target/*" -or $normalized -like "*\target\*") { $exclude = $true }
    if ($normalized -like "dist/*" -or $normalized -eq "dist") { $exclude = $true }
    if ($normalized -like "dist-ssr/*" -or $normalized -eq "dist-ssr") { $exclude = $true }
    if ($normalized -like ".git/*" -or $normalized -eq ".git") { $exclude = $true }
    if ($normalized -like "releases/*.exe" -or $normalized -like "releases/*.msi" -or $normalized -like "releases/*.zip") { $exclude = $true }
    
    # 2. Temp and log files
    if ($file.Extension -in @(".log", ".tmp", ".pdb", ".rlib", ".rmeta", ".d", ".o")) { $exclude = $true }
    
    # 3. Real business Excel workbooks (None in this project, but strict rule enforced)
    if ($file.Extension -in @(".xlsx", ".xls", ".xlsm", ".xlsb")) {
        # Only exclude if not in fixtures/synthetic
        if (-not ($normalized -like "fixtures/synthetic/*")) {
            $exclude = $true
        }
    }

    if ($exclude) {
        $excludedCount++
        continue
    }

    # Add to zip archive with relative path
    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zipFile, $file.FullName, $relative, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    $includedCount++
}

$zipFile.Dispose()

Write-Host "Archive created successfully!" -ForegroundColor Green
Write-Host "Included files: $includedCount"
Write-Host "Excluded files: $excludedCount"
$item = Get-Item $destinationZip
$zipBytes = [int64]$item.Length
Write-Host ("Zip file size: {0:N2} KB ({1} bytes)" -f ($zipBytes / 1KB, $zipBytes))
