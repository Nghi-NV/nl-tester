# Bundling Binaries Guide

This guide explains how to bundle necessary dependencies (ADB, IDB, FFmpeg) into the Lumi Tester Studio application for a seamless user experience.

## Overview

The application is configured to search for and use binaries from:
1. **Bundled Resources**: Located within the app bundle (Primary).
2. **System PATH**: Global installation as a fallback.

## Required Binaries

- **ADB** (Android Debug Bridge): Required for Android testing.
- **IDB** (iOS Development Bridge): Required for iOS testing.
- **FFmpeg**: Optional, used for video recording features.

## How to Bundle

### Step 1: Download Binaries

#### macOS / Linux:
```bash
cd lumi-tester-studio/src-tauri
chmod +x scripts/download_binaries.sh
./scripts/download_binaries.sh
```

#### Windows:
```powershell
cd lumi-tester-studio/src-tauri
.\scripts\download_binaries.ps1
```

The script automatically downloads ADB and places it in `resources/binaries/`.

### Step 2: Acquire IDB (iOS)

IDB must be acquired manually as it depends on your specific environment:

#### macOS:
```bash
# Option 1: Install via Homebrew
brew install idb-companion

# Option 2: Install via pip
pip3 install fb-idb
```

After installation, copy the binary to the resources folder:
```bash
# Locate idb
which idb

# Copy to resources (replace path with your actual idb path)
cp /path/to/idb lumi-tester-studio/src-tauri/resources/binaries/
```

### Step 3: Acquire FFmpeg (Optional)

#### macOS:
```bash
brew install ffmpeg
cp $(which ffmpeg) lumi-tester-studio/src-tauri/resources/binaries/
```

#### Windows:
Download from [ffmpeg.org](https://ffmpeg.org/download.html) and copy `ffmpeg.exe` to `resources/binaries/`.

### Step 4: Build the Application

Once all binaries are placed in `resources/binaries/`, build the app:

```bash
cd lumi-tester-studio
npm run tauri:build
```

The binaries will be automatically bundled into the final installer.

## Directory Structure

```
lumi-tester-studio/src-tauri/
├── resources/
│   └── binaries/
│       ├── adb (or adb.exe)
│       ├── idb (or idb.exe)
│       ├── ffmpeg (or ffmpeg.exe)
│       └── platform-tools/  (contains adb and related tools)
└── scripts/
    ├── download_binaries.sh
    └── download_binaries.ps1
```

## Troubleshooting

1. **Permissions**: On macOS/Linux, ensure binaries are executable: `chmod +x resources/binaries/*`.
2. **Codesigning**: On macOS, bundled binaries may need codesigning:
   ```bash
   codesign --force --deep --sign - resources/binaries/adb
   ```
3. **Architecture**: Ensure binaries match the target architecture (ARM64 vs x86_64).
