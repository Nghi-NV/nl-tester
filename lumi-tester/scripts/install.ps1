$ErrorActionPreference = "Stop"

$Repo = "Nghi-NV/nl-tester"
$AssetName = "lumi-tester-x86_64-pc-windows-msvc.exe"
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/$AssetName"
$InstallDir = "$env:USERPROFILE\.lumi-tester\bin"

Write-Host "Installing lumi-tester to $InstallDir..."
if (!(Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$OutputFile = "$InstallDir\lumi-tester.exe"

# Try downloading with gh if available (best for private repos)
$ghInstalled = Get-Command gh -ErrorAction SilentlyContinue
$ghAuthenticated = $false
if ($ghInstalled) {
    try {
        gh auth status | Out-Null
        $ghAuthenticated = $true
    } catch { }
}

if ($ghAuthenticated) {
    Write-Host "Detected GitHub CLI. Using 'gh release download' for secure access..."
    try {
        gh release download -R $Repo --pattern $AssetName --dir $env:TEMP --clobber
        Move-Item -Path "$env:TEMP\$AssetName" -Destination $OutputFile -Force
    } catch {
        Write-Error "Error: 'gh release download' failed. Ensure you have access to the repository."
        exit 1
    }
} else {
    Write-Host "Downloading from $DownloadUrl..."
    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $OutputFile
    } catch {
        Write-Error "Error: Download failed. Check your internet connection or repository access."
        if ($Repo -eq "Nghi-NV/nl-tester") {
            Write-Host "If this is a private repo, please ensure you have repository access."
        }
        exit 1
    }
}

# Verify the file size (binary should be large, error messages are small)
$fileSize = (Get-Item $OutputFile).Length
if ($fileSize -lt 10000) {
    $content = Get-Content $OutputFile -Raw -TotalCount 500
    if ($content -like "*<!DOCTYPE html>*" -or $content -like "*Not Found*") {
        Write-Error "Error: Downloaded file appears to be an error page or 'Not Found' message."
        Write-Host "This usually happens with private repositories when using standard download methods."
        Write-Host "Recommendation: Install GitHub CLI ('gh'), run 'gh auth login', and try again."
        Remove-Item $OutputFile
        exit 1
    }
}

# Add to PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Adding $InstallDir to PATH..."
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", [EnvironmentVariableTarget]::User)
    $env:Path += ";$InstallDir"
    Write-Host "PATH updated. You may need to restart your terminal."
}

Write-Host "lumi-tester installed successfully!"

Write-Host "Initializing system components (ADB, Playwright)..."
& "$OutputFile" system install --all

Write-Host "Done! You can now use 'lumi-tester' command."
