param(
    [string]$OutputRoot = "$env:TEMP\lumi-windows-desktop-smoke"
)

$ErrorActionPreference = "Stop"

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw "This smoke script must run on Windows."
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class LumiWin32 {
  [DllImport("user32.dll")]
  public static extern IntPtr GetForegroundWindow();
}
"@

$handle = [LumiWin32]::GetForegroundWindow()
if ($handle -eq [IntPtr]::Zero) {
    throw "No interactive foreground desktop window is available. Run this from an interactive Windows desktop session."
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

cargo run --locked -- doctor --platform windows
cargo run --locked -- validate e2e\desktop\windows-native-smoke.yaml
cargo run --locked -- validate e2e\desktop\windows-uia-selector-smoke.yaml

cargo run --locked -- run e2e\desktop\windows-native-smoke.yaml `
    --platform windows `
    --output (Join-Path $OutputRoot "native") `
    --events-jsonl

cargo run --locked -- run e2e\desktop\windows-uia-selector-smoke.yaml `
    --platform windows `
    --output (Join-Path $OutputRoot "uia-selector") `
    --events-jsonl

Write-Host "Windows desktop smoke completed: $OutputRoot"
