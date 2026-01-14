# Nexus API

A powerful API testing tool built with Tauri, React, and TypeScript. Create, manage, and execute API test flows with an intuitive YAML-based configuration.

## Features

- 📝 **YAML-based Test Flows**: Define your API tests using simple YAML syntax
- 🔄 **Flow Composition**: Organize tests into reusable flows and nested structures
- 🌍 **Environment Variables**: Manage multiple environments with variable interpolation
- 📊 **Test Analytics**: Comprehensive reporting and analytics dashboard
- 🎯 **Request/Response Inspection**: Detailed view of requests and responses with JSON viewer
- ⚡ **Fast Execution**: Built with Tauri for native performance
- 🎨 **Modern UI**: Beautiful, responsive interface built with React and Tailwind CSS

## Tech Stack

- **Frontend**: React 19, TypeScript, Vite, Tailwind CSS
- **Backend**: Tauri 2, Rust
- **Editor**: Monaco Editor
- **Charts**: Recharts
- **State Management**: Zustand

## Getting Started

### Prerequisites

- Node.js 20+
- Rust (latest stable)
- Yarn

### Installation

```bash
# Install dependencies
yarn install

# Run in development mode
yarn tauri dev

# Build for production
yarn tauri build
```

## Project Structure

```
nexus-api/
├── src/                    # Frontend React application
│   ├── components/        # React components
│   ├── services/          # Business logic services
│   ├── stores/            # Zustand state management
│   └── utils/             # Utility functions
├── src-tauri/             # Tauri backend (Rust)
│   ├── src/               # Rust source code
│   └── Cargo.toml         # Rust dependencies
└── .github/               # GitHub Actions workflows
```

## Development

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Building

The project uses GitHub Actions for automated builds. To create a release:

1. Create a tag: `git tag v1.0.0`
2. Push the tag: `git push origin v1.0.0`
3. GitHub Actions will automatically build and create a release

## License

MIT License

## Author

Nghin Nguyen
