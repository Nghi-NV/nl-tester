$ErrorActionPreference = "Stop"

$Repo = if ($env:LUMI_TESTER_REPO) { $env:LUMI_TESTER_REPO } else { "Nghi-NV/nl-tester" }
$Version = if ($env:LUMI_TESTER_VERSION) { $env:LUMI_TESTER_VERSION } else { "latest" }
$InstallDir = if ($env:LUMI_INSTALL_DIR) { $env:LUMI_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".lumi-tester\bin" }
$SkipSystemInstall = $env:LUMI_SKIP_SYSTEM_INSTALL -eq "1"

function Fail($Message) {
    Write-Error $Message
    exit 1
}

function Get-AssetName {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "ARM64" { return "lumi-tester-aarch64-pc-windows-msvc.exe" }
        "AMD64" { return "lumi-tester-x86_64-pc-windows-msvc.exe" }
        default { Fail "Unsupported Windows architecture: $env:PROCESSOR_ARCHITECTURE" }
    }
}

function Get-ReleaseBaseUrl {
    if ($Version -eq "latest") {
        return "https://github.com/$Repo/releases/latest/download"
    }
    return "https://github.com/$Repo/releases/download/$Version"
}

function Download-File($Url, $Output) {
    Invoke-WebRequest -Uri $Url -OutFile $Output -UseBasicParsing
}

function Verify-Checksum($ChecksumsFile, $AssetName, $FilePath) {
    if (!(Test-Path $ChecksumsFile)) {
        return
    }

    $line = Get-Content $ChecksumsFile | Where-Object { $_ -match "\s$([regex]::Escape($AssetName))$" } | Select-Object -First 1
    if (!$line) {
        return
    }

    $expected = ($line -split "\s+")[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 $FilePath).Hash.ToLowerInvariant()
    if ($expected -ne $actual) {
        Fail "Checksum mismatch for $AssetName"
    }
    Write-Host "Checksum verified."
}

$AssetName = Get-AssetName
$BaseUrl = Get-ReleaseBaseUrl
$TempDir = Join-Path $env:TEMP ("lumi-tester-install-" + [Guid]::NewGuid().ToString("N"))
$TempFile = Join-Path $TempDir $AssetName
$ChecksumsFile = Join-Path $TempDir "SHA256SUMS"
$OutputFile = Join-Path $InstallDir "lumi-tester.exe"

try {
    New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    Write-Host "Installing lumi-tester"
    Write-Host "  Repository: $Repo"
    Write-Host "  Version: $Version"
    Write-Host "  Asset: $AssetName"
    Write-Host "  Install dir: $InstallDir"

    Write-Host "Downloading $BaseUrl/$AssetName"
    Download-File "$BaseUrl/$AssetName" $TempFile

    try {
        Download-File "$BaseUrl/SHA256SUMS" $ChecksumsFile
        Verify-Checksum $ChecksumsFile $AssetName $TempFile
    } catch {
        Write-Host "Checksum file not found; skipping checksum verification."
    }

    if ((Get-Item $TempFile).Length -eq 0) {
        Fail "Downloaded file is empty"
    }

    Move-Item -Path $TempFile -Destination $OutputFile -Force

    $UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Host "Adding $InstallDir to PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", [EnvironmentVariableTarget]::User)
        $env:Path += ";$InstallDir"
        Write-Host "PATH updated. Restart your terminal if lumi-tester is not found."
    }

    Write-Host "Installed: $OutputFile"
    & "$OutputFile" --version

    if (!$SkipSystemInstall) {
        Write-Host "Initializing drivers and browser dependencies..."
        & "$OutputFile" system install --all
    } else {
        Write-Host "Skipping system install because LUMI_SKIP_SYSTEM_INSTALL=1"
    }

    Write-Host "Done. Run: lumi-tester --help"
} finally {
    if (Test-Path $TempDir) {
        Remove-Item -Recurse -Force $TempDir
    }
}
