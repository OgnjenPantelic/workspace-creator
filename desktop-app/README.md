# Databricks Deployer

Desktop app for deploying Databricks workspaces on AWS and Azure using Terraform.

**Download:** See [GitHub Releases](https://github.com/OgnjenPantelic/workspace-creator/releases) for the latest builds.

## Features

- Guided deployment wizard
- VNet injection (Azure) / BYOVPC (AWS)
- Unity Catalog metastore detection and assignment
- Auto-installs Terraform if missing
- Supports CLI profiles and SSO authentication

## Prerequisites

| Requirement | Status | Notes |
|-------------|--------|-------|
| Git | Required | For Terraform module downloads |
| Terraform | Required | Auto-installed if missing |
| Databricks CLI | Optional | Enables profile-based auth |
| AWS CLI | Optional | For AWS deployments, SSO supported |
| Azure CLI | Optional | For Azure deployments |

## Quick Start

1. Download from Releases or build from source
2. Select cloud provider (AWS or Azure)
3. Enter Databricks credentials (profile or service principal)
4. Enter cloud credentials (CLI profile or manual)
5. Configure workspace (name, region, networking)
6. Review and deploy

## Deployment Storage

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/com.databricks.deployer/deployments/` |
| Windows | `%APPDATA%\com.databricks.deployer\deployments\` |
| Linux | `~/.local/share/com.databricks.deployer/deployments/` |

## Build from Source

### Requirements
- Node.js 18+
- Rust 1.70+
- Platform build tools (Xcode on macOS, Visual Studio on Windows)

### Commands
```bash
npm install
npm run tauri dev      # Development
npm run tauri build    # Production
```

Output: `src-tauri/target/release/bundle/`

## Authentication

### Databricks
**Option 1: CLI Profile** (recommended)
```bash
databricks auth login --account-id <ACCOUNT_ID>
```

**Option 2: Service Principal**
- Account ID
- Client ID
- Client Secret

### AWS
- CLI profiles (`~/.aws/credentials`) including SSO
- Or manual: Access Key ID, Secret Access Key

### Azure
- CLI auth (`az login`)
- Or service principal: Tenant ID, Subscription ID, Client ID, Client Secret

## Adding Templates

1. Create `src-tauri/templates/{cloud}-{name}/`
2. Add Terraform files (`variables.tf` required)
3. Register in `src-tauri/src/commands.rs` → `get_templates()`
4. Increment `TEMPLATES_VERSION` in `commands.rs`
5. Rebuild

## Project Structure

```
src/
  App.tsx              # Main application
  main.tsx             # Entry point
  constants.ts         # Configuration constants
  types.ts             # TypeScript types
  components/
    common/            # Shared components (ErrorBoundary)
    screens/           # Screen components (Welcome, CloudSelection, Dependencies)

src-tauri/
  src/
    lib.rs             # Tauri app setup
    commands.rs        # Backend commands
    terraform.rs       # Terraform execution
    dependencies.rs    # CLI detection
    errors.rs          # Error helpers
  templates/
    aws-simple/        # AWS BYOVPC template
    azure-simple/      # Azure VNet injection template
```

## Troubleshooting

### Terraform not found after install
Restart the app. Terraform is installed to `~/.databricks-deployer/bin/`.

### Azure CLI not detected
Run `az login` in terminal first, then restart the app.

### AWS SSO session expired
Run `aws sso login --profile <profile>` to refresh.

### Deployment stuck
Check the logs in the deployment folder. You can run `terraform apply` manually from there.

### Unity Catalog permissions error
Ensure your service principal has metastore admin or required grants on the existing metastore.
