# Workspace Creator

Tools for deploying Databricks workspaces across AWS, Azure, and GCP.

## Components

| Directory | Description |
|-----------|-------------|
| `desktop-app/` | Tauri desktop app for guided workspace deployment (React + Rust) |
| `download-site/` | Flask-based download page for distributing the app |

## Quick Start

### Desktop App
```bash
cd desktop-app
npm install
npm run tauri dev
```

See [desktop-app/README.md](desktop-app/README.md) for full documentation.

### Download Site
```bash
cd download-site
pip install -r requirements.txt
python app.py
```

Runs on port 8080 by default. Deployed to Google App Engine via `app.yaml`.

## CI/CD

**PR Validation** (`.github/workflows/ci.yml`)
- Runs TypeScript compilation and Vitest tests on pull requests
- Fast feedback (~3-5 minutes)

**Release Builds** (`.github/workflows/build-desktop.yml`)
- Triggers only on tag pushes (`v*`) or manual dispatch
- Builds for macOS ARM64, macOS x64, and Windows x64
- Includes Rust cargo caching for faster builds
- Automatically creates GitHub Release with all artifacts

**Releases:** [github.com/OgnjenPantelic/workspace-creator/releases](https://github.com/OgnjenPantelic/workspace-creator/releases)
