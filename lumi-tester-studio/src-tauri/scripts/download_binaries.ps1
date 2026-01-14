# PowerShell script để tải và đóng gói các binaries cho Windows

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ResourcesDir = Join-Path $ScriptDir "..\resources\binaries"

Write-Host "📦 Downloading binaries for Windows..." -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $ResourcesDir | Out-Null

# Download ADB (Android Debug Bridge)
Write-Host "⬇️  Downloading ADB..." -ForegroundColor Yellow
$AdbDir = Join-Path $ResourcesDir "platform-tools"
New-Item -ItemType Directory -Force -Path $AdbDir | Out-Null

$AdbUrl = "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
$AdbZip = Join-Path $ResourcesDir "platform-tools.zip"

if (-not (Test-Path (Join-Path $AdbDir "adb.exe"))) {
    Write-Host "Downloading from $AdbUrl..." -ForegroundColor Gray
    Invoke-WebRequest -Uri $AdbUrl -OutFile $AdbZip
    
    Write-Host "Extracting..." -ForegroundColor Gray
    Expand-Archive -Path $AdbZip -DestinationPath $ResourcesDir -Force
    
    Remove-Item $AdbZip -Force
    
    Write-Host "✅ ADB downloaded successfully" -ForegroundColor Green
} else {
    Write-Host "✅ ADB already exists" -ForegroundColor Green
}

# Create copy of adb.exe in binaries directory
if (Test-Path (Join-Path $AdbDir "adb.exe")) {
    Copy-Item (Join-Path $AdbDir "adb.exe") (Join-Path $ResourcesDir "adb.exe") -Force
    # Copy necessary DLLs for ADB on Windows
    Get-ChildItem -Path $AdbDir -Filter "*.dll" | ForEach-Object {
        Copy-Item $_.FullName (Join-Path $ResourcesDir $_.Name) -Force
    }
}

Write-Host ""
Write-Host "✅ Binaries download complete!" -ForegroundColor Green
Write-Host "📁 Binaries location: $ResourcesDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "⚠️  Note: IDB và FFmpeg cần được cài đặt thủ công:" -ForegroundColor Yellow
Write-Host "   - IDB: pip install fb-idb" -ForegroundColor Gray
Write-Host "   - FFmpeg: Download from https://ffmpeg.org/download.html" -ForegroundColor Gray
