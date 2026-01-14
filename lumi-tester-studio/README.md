# 🎨 Lumi Tester Studio
A professional Desktop IDE for mobile and web automation testing, built with Tauri, React, and TypeScript.

Lumi Tester Studio provides a seamless experience for authoring, managing, and executing test cases using the `lumi-tester` engine.

## ✨ Key Features

- 📑 **Project Management**: VSCode-like file explorer for organizing test suites and assets.
- ✍️ **Intelligent Editor**: Monaco-based YAML editor with autocomplete for `lumi-tester` commands.
- 🎯 **Device Hub**: Real-time device discovery and selection for Android, iOS, and Web.
- ⚡ **Live Execution**: Run tests directly from the IDE and monitor progress in real-time.
- 📊 **Results Viewer**: Built-in dashboard to analyze test reports, screenshots, and videos.

## 🛠️ Development

### Prerequisites

- **Node.js**: 20 or higher
- **Rust**: Latest stable toolchain
- **Yarn**: Recommended package manager

### Getting Started

```bash
# Install dependencies
yarn install

# Start in development mode
yarn tauri dev

# Build for production
yarn tauri build
```

## 🏗️ Architecture

- **Frontend**: React 19, Vite, Tailwind CSS, Zustand
- **Backend (Tauri)**: Rust, handling file system and CLI bridging
- **Editor**: Monaco Editor with custom YAML language support

## 📝 License
See the root [LICENSE](../LICENSE) file.

