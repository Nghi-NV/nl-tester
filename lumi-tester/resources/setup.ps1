# Setup resources for lumi-tester development environment (Windows)
# Downloads: ADBKeyboard, ADB (platform-tools), FFmpeg, Playwright

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ResourcesDir = $ScriptDir
$ApkDir = Join-Path $ResourcesDir "apk"
$InstallDir = Join-Path $env:USERPROFILE ".lumi-tester"

Write-Host "🔧 Setting up lumi-tester resources for Windows..." -ForegroundColor Cyan
Write-Host ""

# Create directories
$dirs = @(
    $ApkDir,
    "$InstallDir\apk",
    "$InstallDir\platform-tools",
    "$InstallDir\playwright"
)
foreach ($dir in $dirs) {
    if (!(Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
}

#---------------------------------------
# 1. ADBKeyboard APK
#---------------------------------------
Write-Host "📦 [1/4] ADBKeyboard APK..." -ForegroundColor White
$AdbKeyboardUrl = "https://github.com/senzhk/ADBKeyBoard/raw/master/ADBKeyboard.apk"
$AdbKeyboardApk = Join-Path $ApkDir "ADBKeyboard.apk"

if (!(Test-Path $AdbKeyboardApk)) {
    Invoke-WebRequest -Uri $AdbKeyboardUrl -OutFile $AdbKeyboardApk
    Write-Host "   ✓ Downloaded ADBKeyboard.apk" -ForegroundColor Green
} else {
    Write-Host "   ✓ ADBKeyboard.apk already exists" -ForegroundColor Green
}
Copy-Item $AdbKeyboardApk -Destination "$InstallDir\apk\" -Force

#---------------------------------------
# 2. Android Platform Tools (ADB)
#---------------------------------------
Write-Host "📦 [2/4] Android Platform Tools (ADB)..." -ForegroundColor White
$PlatformToolsUrl = "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
$AdbPath = "$InstallDir\platform-tools\adb.exe"

if (!(Test-Path $AdbPath)) {
    $TempZip = "$env:TEMP\platform-tools.zip"
    Write-Host "   Downloading platform-tools..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $PlatformToolsUrl -OutFile $TempZip
    Expand-Archive -Path $TempZip -DestinationPath $InstallDir -Force
    Remove-Item $TempZip
    Write-Host "   ✓ Downloaded and extracted platform-tools" -ForegroundColor Green
} else {
    Write-Host "   ✓ ADB already exists at $AdbPath" -ForegroundColor Green
}

#---------------------------------------
# 3. FFmpeg (via Playwright)
#---------------------------------------
Write-Host "📦 [3/4] FFmpeg..." -ForegroundColor White
$FfmpegPath = "$InstallDir\playwright\ffmpeg.exe"

if (!(Test-Path $FfmpegPath)) {
    # Try to find ffmpeg in Playwright cache
    $PlaywrightCache = "$env:LOCALAPPDATA\ms-playwright"
    $FoundFfmpeg = Get-ChildItem -Path $PlaywrightCache -Filter "ffmpeg.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    
    if ($FoundFfmpeg) {
        Copy-Item $FoundFfmpeg.FullName -Destination $FfmpegPath -Force
        Write-Host "   ✓ Copied ffmpeg from Playwright cache" -ForegroundColor Green
    } else {
        Write-Host "   ⚠ FFmpeg not found. Run 'npx playwright install' first, then re-run this script" -ForegroundColor Yellow
    }
} else {
    Write-Host "   ✓ FFmpeg already exists" -ForegroundColor Green
}

#---------------------------------------
# 4. Playwright (Node.js dependency)
#---------------------------------------
Write-Host "📦 [4/4] Playwright browsers..." -ForegroundColor White
$NpxPath = Get-Command npx -ErrorAction SilentlyContinue
if ($NpxPath) {
    Write-Host "   Installing Playwright browsers..." -ForegroundColor Yellow
    & npx playwright install chromium 2>$null
    Write-Host "   ✓ Playwright browsers installed" -ForegroundColor Green
} else {
    Write-Host "   ⚠ npx not found. Install Node.js to use Playwright" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "✅ Resources setup complete!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "Installed locations:"
Write-Host "  📱 ADBKeyboard: $InstallDir\apk\"
Write-Host "  🤖 ADB:         $InstallDir\platform-tools\adb.exe"
Write-Host "  🎬 FFmpeg:      $InstallDir\playwright\ffmpeg.exe"
Write-Host "  🌐 Playwright:  $env:LOCALAPPDATA\ms-playwright (browsers)"
Write-Host ""
Write-Host "Note: iOS testing is not supported on Windows" -ForegroundColor Gray
