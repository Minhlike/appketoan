param(
    [string]$ExePath = (Join-Path $PSScriptRoot "..\target\release\appketoan.exe"),
    [int]$TimeoutSeconds = 20
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path
if ([IO.Path]::GetExtension($resolvedExe) -ne ".exe") {
    throw "Release smoke target must be an .exe file: $resolvedExe"
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class AppKetoanSmokeNative
{
    public delegate bool EnumWindowProc(IntPtr window, IntPtr state);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowProc callback, IntPtr state);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr window, StringBuilder className, int capacity);

    [DllImport("user32.dll")]
    public static extern bool IsIconic(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr window, int command);
}
"@

function Find-TauriWindow([int]$ProcessId) {
    $script:tauriWindow = [IntPtr]::Zero
    $callback = [AppKetoanSmokeNative+EnumWindowProc] {
        param([IntPtr]$window, [IntPtr]$state)
        [uint32]$ownerProcessId = 0
        [AppKetoanSmokeNative]::GetWindowThreadProcessId($window, [ref]$ownerProcessId) | Out-Null
        if ($ownerProcessId -eq $ProcessId) {
            $className = New-Object Text.StringBuilder 128
            [AppKetoanSmokeNative]::GetClassName($window, $className, $className.Capacity) | Out-Null
            if ($className.ToString() -eq "Tauri Window") {
                $script:tauriWindow = $window
                return $false
            }
        }
        return $true
    }
    [AppKetoanSmokeNative]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
    return $script:tauriWindow
}

$process = $null
try {
    $process = Start-Process -FilePath $resolvedExe -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $window = [IntPtr]::Zero
    while ((Get-Date) -lt $deadline -and $window -eq [IntPtr]::Zero -and -not $process.HasExited) {
        Start-Sleep -Milliseconds 200
        $window = Find-TauriWindow $process.Id
        $process.Refresh()
    }

    if ($process.HasExited) {
        throw "Release executable exited during startup with code $($process.ExitCode)."
    }
    if ($window -eq [IntPtr]::Zero) {
        throw "Tauri main window was not created within $TimeoutSeconds seconds."
    }
    if ([AppKetoanSmokeNative]::IsIconic($window)) {
        [AppKetoanSmokeNative]::ShowWindow($window, 9) | Out-Null
    }

    $headingFound = $false
    $failureFound = $false
    $elementCount = 0
    $observedNames = [Collections.Generic.HashSet[string]]::new()
    while ((Get-Date) -lt $deadline -and -not $headingFound -and -not $failureFound) {
        $root = [Windows.Automation.AutomationElement]::FromHandle($window)
        $elements = $root.FindAll(
            [Windows.Automation.TreeScope]::Descendants,
            [Windows.Automation.Condition]::TrueCondition
        )
        $elementCount = $elements.Count
        for ($index = 0; $index -lt $elements.Count; $index++) {
            $name = [string]$elements.Item($index).Current.Name
            if (-not [string]::IsNullOrWhiteSpace($name)) {
                $observedNames.Add($name) | Out-Null
            }
            $normalizedName = ($name -replace "\s+", " ").Trim()
            if ($normalizedName -eq "APPKETOAN_UI_READY") {
                $headingFound = $true
            }
            if ($normalizedName -eq "AppKetoan không thể khởi động giao diện") {
                $failureFound = $true
            }
        }
        if (-not $headingFound -and -not $failureFound) {
            Start-Sleep -Milliseconds 250
        }
    }

    if ($failureFound) {
        throw "The executable rendered its bootstrap failure screen."
    }
    if (-not $headingFound) {
        $sample = ($observedNames | Select-Object -First 12) -join " | "
        throw "The executable stayed alive but did not render the AppKetoan UI. UI elements found: $elementCount. Observed names: $sample"
    }

    Write-Output "UI_SMOKE=PASS"
    Write-Output "EXE=$resolvedExe"
    Write-Output "UI_ELEMENTS=$elementCount"
    Write-Output "EXPECTED_MARKER=APPKETOAN_UI_READY"
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id
        Wait-Process -Id $process.Id -ErrorAction SilentlyContinue
    }
}
