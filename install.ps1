$ErrorActionPreference = "Stop"

$RepoOwner = "nghi-nv"
$RepoName = "nl-tester"
$InstallDir = "$env:LOCALAPPDATA\lumi-tester\bin"
$FileName = "lumi-tester.exe"

# Create install directory
if (!(Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

# Determine Architecture
$Arch = "amd64" # Assume x64 for now as we only build that
$AssetName = "lumi-tester-windows-$Arch.exe"

# Get Latest Release Download URL
$ReleaseUrl = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
try {
    $ReleaseInfo = Invoke-RestMethod -Uri $ReleaseUrl
    $Asset = $ReleaseInfo.assets | Where-Object { $_.name -eq $AssetName }
    
    if (-not $Asset) {
        Write-Error "Could not find asset $AssetName in the latest release."
    }
    
    $DownloadUrl = $Asset.browser_download_url
}
catch {
    Write-Error "Failed to fetch release info: $_"
}

Write-Host "Downloading $AssetName from $DownloadUrl..."
$OutputPath = Join-Path -Path $InstallDir -ChildPath $FileName
Invoke-WebRequest -Uri $DownloadUrl -OutFile $OutputPath

# Add to PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Adding $InstallDir to User PATH..."
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", [EnvironmentVariableTarget]::User)
    $env:Path += ";$InstallDir"
    Write-Host "Added to PATH. restart your shell to take effect globally."
}

Write-Host "Successfully installed lumi-tester to $OutputPath"
Write-Host "Run 'lumi-tester --version' to verify."
