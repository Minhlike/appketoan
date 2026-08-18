Add-Type -AssemblyName System.IO.Compression.FileSystem

$zipPath = "D:\appketoan-audit.zip"
$zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
$entries = $zip.Entries | ForEach-Object { $_.FullName }
$zip.Dispose()

$zipFile = Get-Item $zipPath

Write-Host "ZIP Path: $zipPath"
Write-Host "ZIP Size: $([math]::Round($zipFile.Length / 1KB, 2)) KB"
Write-Host "Total Files: $($entries.Count)"

$forbidden = $entries | Where-Object {
    $_ -like "*node_modules*" -or
    $_ -like "*target*" -or
    $_ -like ".git/*" -or
    $_ -like ".git\\*" -or
    $_ -like "*.exe" -or
    $_ -like "*.msi" -or
    $_ -like "*.pdb"
}

Write-Host "Forbidden items count: $($forbidden.Count)"
if ($forbidden.Count -eq 0) {
    Write-Host "CLEAN_AUDIT_ZIP_VERIFIED: YES"
} else {
    Write-Host "CLEAN_AUDIT_ZIP_VERIFIED: NO"
    $forbidden | ForEach-Object { Write-Host " - $_" }
}
