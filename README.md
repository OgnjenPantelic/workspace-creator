# Workspace Creator

Tools for deploying Databricks workspaces.

## Components

| Directory | Description |
|-----------|-------------|
| `desktop-app/` | Tauri desktop app for guided workspace deployment |
| `download-site/` | Download page for distributing the app |

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
