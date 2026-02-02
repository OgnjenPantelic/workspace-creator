# Databricks Deployer

A desktop application for deploying production-grade Databricks workspaces on AWS and Azure using battle-tested Terraform templates.

## Features

- **Guided Deployment** - Step-by-step wizard, no Terraform knowledge required
- **Enterprise Security** - VNet injection, private networking, NSGs included
- **Unity Catalog** - Automatic metastore detection and assignment
- **Multi-Cloud** - AWS (BYOVPC) and Azure (VNet injection) support
- **Auto-Install** - Downloads Terraform if not present

## Prerequisites

| Requirement | Notes |
|-------------|-------|
| **Terraform** | Auto-installed if missing |
| **AWS CLI** | For AWS deployments (SSO supported) |
| **Azure CLI** | For Azure deployments |
| **Databricks Service Principal** | Account admin privileges required |

## Quick Start

1. **Download** from [Releases](releases) or build from source
2. **Select** your cloud provider (AWS or Azure)
3. **Enter** Databricks and cloud credentials
4. **Configure** workspace settings (names, networking CIDRs)
5. **Deploy** - review plan and confirm

Deployments are saved to `~/Library/Application Support/com.databricks.deployer/deployments/`

## Build from Source

```bash
# Install dependencies
npm install

# Development
npm run tauri dev

# Production build
npm run tauri build
# Output: src-tauri/target/release/bundle/
```

## Credentials

### Databricks (Required)
- **Account ID** - Found in account console URL
- **Client ID** - Service principal application ID
- **Client Secret** - Service principal secret

The service principal needs **Account Admin** role in Databricks.

### AWS
Use AWS CLI profiles (including SSO) or provide:
- Access Key ID, Secret Access Key, Session Token (optional)

### Azure
Use Azure CLI (`az login`) or provide:
- Tenant ID, Subscription ID, Client ID, Client Secret

## Adding Templates

1. Create folder: `src-tauri/templates/{cloud}-{name}/`

2. Add Terraform files (`variables.tf` required):
```hcl
variable "workspace_name" {
  description = "Name of the Databricks workspace"
  type        = string
}
```

3. Register in `src-tauri/src/commands.rs` → `get_templates()`

4. Increment `TEMPLATES_VERSION` in `commands.rs`

5. Rebuild: `npm run tauri build`

## Architecture

```
src/                    # React frontend
  App.tsx               # Main application
  components/           # Reusable UI components
  hooks/                # Custom React hooks
  
src-tauri/              # Rust backend
  src/
    commands.rs         # Tauri commands
    terraform.rs        # Terraform execution
    dependencies.rs     # CLI detection
  templates/            # Terraform templates
    aws-simple/
    azure-simple/
```

## License

MIT
