# Databricks Deployer

**Version:** 1.0.8

Desktop app for deploying Databricks workspaces on AWS, Azure, and GCP using Terraform.

**Download:** See [GitHub Releases](https://github.com/OgnjenPantelic/workspace-creator/releases) for the latest builds.

## Features

- Guided deployment wizard for AWS, Azure, and GCP
- Template-based deployments with per-cloud template selection
- VNet injection (Azure) / BYOVPC (AWS) / Customer-managed VPC (GCP)
- Unity Catalog metastore auto-detection and assignment
- Catalog creation with isolated storage (S3 / Azure Storage / GCS)
- Auto-installs Terraform if missing (v1.9.8)
- Supports CLI profiles, SSO, and service principal authentication
- Azure identity for Databricks (no service principal needed with Azure CLI + Account Admin)
- GCP service account creation with custom IAM role and impersonation setup
- Cloud-specific permission validation before deployment
- Rollback support (terraform destroy with resource cleanup)
- Single-instance enforcement (prevents running multiple copies)

## Prerequisites

| Requirement | Status | Notes |
|-------------|--------|-------|
| Git | Required | For Terraform module downloads |
| Terraform | Required | Auto-installed if missing (v1.9.8) |
| Databricks CLI | Optional | Enables profile-based auth |
| AWS CLI | Optional | For AWS deployments, SSO supported |
| Azure CLI | Optional | For Azure deployments |
| Google Cloud CLI | Optional | For GCP deployments (ADC and impersonation) |

## Quick Start

1. Download from Releases or build from source
2. Select cloud provider (AWS, Azure, or GCP)
3. Verify dependencies (Terraform, Git, cloud CLIs)
4. Enter cloud credentials (CLI profile, service principal, or ADC)
5. Enter Databricks credentials (profile, service principal, or Azure identity)
6. Select deployment template
7. Configure workspace (name, region, networking)
8. Configure Unity Catalog (optional -- auto-detects existing metastore)
9. Review and deploy

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
npm run dev            # Frontend only (Vite dev server)
npm run tauri dev      # Full app with Rust backend
npm run build          # Build frontend
npm run tauri build    # Production build
npm run test           # Run tests (watch mode)
npm run test:run       # Run tests once
```

Output: `src-tauri/target/release/bundle/` (macOS: `.dmg`, Windows: `.msi`/`.exe`)

## Version Management & Releases

The project uses automated version syncing across all configuration files (`package.json`, `Cargo.toml`, `tauri.conf.json`).

### Creating a New Release

From the `desktop-app/` directory:

```bash
# Bump version (pick one)
npm version patch --no-git-tag-version   # 1.0.8 → 1.0.9
npm version minor --no-git-tag-version   # 1.0.8 → 1.1.0
npm version major --no-git-tag-version   # 1.0.8 → 2.0.0

# Commit and tag from the repo root
cd ..
git add .
git commit -m "v1.0.9"
git tag v1.0.9
git push --follow-tags
```

The `npm version` command automatically syncs the version to `Cargo.toml` and `tauri.conf.json` via the `version` lifecycle hook.

GitHub Actions will then build macOS (arm64 + x64) and Windows installers and create a GitHub release.

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

**Azure users:** Can use Azure identity if authenticated via Azure CLI and have Account Admin role (see Azure authentication).

### AWS
- CLI profiles (`~/.aws/credentials`) including SSO
- Or manual: Access Key ID, Secret Access Key
- Required IAM permissions: EC2, VPC, S3, IAM, STS (for cross-account roles)

### Azure
- CLI auth (`az login`)
- Or service principal: Tenant ID, Subscription ID, Client ID, Client Secret

**Option 3: Azure Identity (Azure CLI only)**
- Requires Azure CLI (`az login`) and Databricks Account Admin privileges
- Uses Azure AD token directly (no Databricks service principal needed)
- Terraform auth type: `azure-cli`

### GCP
**Option 1: ADC with Service Account Impersonation** (recommended)
```bash
gcloud auth application-default login
```
The app can create a service account with a custom IAM role and configure impersonation automatically.

**Option 2: Service Account Key**
- Paste a JSON key file for a service account with the required permissions
- Required GCP permissions: Compute, Service Networking, IAM, Service Account Admin, Storage

## Development

### Running Tests
```bash
npm run test           # Watch mode
npm run test:run       # Single run
```

Tests use Vitest with React Testing Library. Tauri commands are automatically mocked in `src/test/setup.ts`.

### Code Quality
```bash
npm run build          # TypeScript compilation check
```

### Adding Features
- Template changes require incrementing `TEMPLATES_VERSION` in `src-tauri/src/commands/mod.rs`
- Variable display names go in `src/constants.ts`
- See `.cursor/rules/` for project conventions

## Templates

### Available Templates

| ID | Name | Cloud | Description |
|----|------|-------|-------------|
| `aws-simple` | AWS Standard BYOVPC | AWS | Standard workspace with customer-managed VPC ([README](src-tauri/templates/aws-simple/README.md)) |
| `azure-simple` | Azure Standard VNet | Azure | Standard workspace with VNet injection ([README](src-tauri/templates/azure-simple/README.md)) |
| `gcp-simple` | GCP Standard BYOVPC | GCP | Standard workspace with customer-managed VPC ([README](src-tauri/templates/gcp-simple/README.md)) |

Each template creates Unity Catalog metastore/catalog, workspace, networking, and required cloud resources.

### Adding Templates

1. Create `src-tauri/templates/{cloud}-{name}/` with Terraform files (`variables.tf` required)
2. Register in `src-tauri/src/commands/templates.rs` -> `get_templates()`
3. Add variable display names to `VARIABLE_DISPLAY_NAMES` in `src/constants.ts`
4. Add variable descriptions to `VARIABLE_DESCRIPTION_OVERRIDES` in `src/constants.ts`
5. Add credential/internal variables to `EXCLUDE_VARIABLES` or `INTERNAL_VARIABLES`
6. Add section mappings in `groupVariablesBySection()` in `src/utils/variables.ts`
7. Increment `TEMPLATES_VERSION` in `src-tauri/src/commands/mod.rs`
8. Rebuild

## Project Structure

```
src/
  App.tsx                # Main application entry
  main.tsx               # Vite entry point
  constants.ts           # Configuration constants, regions, variable overrides
  types.ts               # TypeScript types and interfaces
  styles.css             # Global styles
  context/
    WizardContext.tsx    # Wizard state management and shared context
  hooks/
    useAwsAuth.ts        # AWS authentication (profiles, SSO, access keys)
    useAzureAuth.ts      # Azure authentication (CLI, service principals)
    useGcpAuth.ts        # GCP authentication (ADC, impersonation, SA keys)
    useDatabricksAuth.ts # Databricks account authentication (profiles, SP)
    useDeployment.ts     # Deployment orchestration (init, plan, apply, rollback)
    useUnityCatalog.ts   # Unity Catalog metastore detection and configuration
    useWizard.ts         # Wizard navigation and step management
    useSsoPolling.ts     # SSO login polling
  utils/
    variables.ts         # Variable grouping, formatting, suffix generation
    cloudValidation.ts   # Cloud permission validation
    databricksValidation.ts # Databricks credential validation
  components/
    WizardRouter.tsx     # Main wizard routing logic
    common/
      ErrorBoundary.tsx  # React error boundary
    ui/
      Alert.tsx          # Alert and StatusMessage components
      AuthModeSelector.tsx    # Radio-button auth mode selector
      AzureAdminDialog.tsx    # Azure admin consent flow dialog
      FormGroup.tsx           # FormGroup and collapsible FormSection
      LoadingSpinner.tsx      # Spinner and LoadingOverlay
      PermissionWarningDialog.tsx  # Cloud permission warnings
    screens/
      WelcomeScreen.tsx
      CloudSelectionScreen.tsx
      DependenciesScreen.tsx
      TemplateSelectionScreen.tsx
      ConfigurationScreen.tsx
      UnityCatalogConfigScreen.tsx
      DeploymentScreen.tsx
      credentials/
        AwsCredentialsScreen.tsx
        AzureCredentialsScreen.tsx
        GcpCredentialsScreen.tsx
        DatabricksCredentialsScreen.tsx
  test/
    setup.ts             # Vitest global setup (mocks @tauri-apps/api)
    hooks/               # Hook unit tests
    utils/               # Utility function tests

src-tauri/
  src/
    lib.rs               # Tauri app setup, plugin registration, template extraction
    main.rs              # Application entry point
    commands/
      mod.rs             # Command exports and template version
      aws.rs             # AWS-specific commands
      azure.rs           # Azure-specific commands
      gcp.rs             # GCP-specific commands and SA creation
      databricks.rs      # Databricks authentication
      deployment.rs      # Deployment configuration and tfvars generation
      templates.rs       # Template listing and variable parsing
    terraform.rs         # Terraform execution (init, plan, apply, destroy)
    dependencies.rs      # CLI detection, version checks, Terraform auto-install
    errors.rs            # Error helpers
  templates/
    aws-simple/          # AWS BYOVPC template (see template README)
    azure-simple/        # Azure VNet injection template (see template README)
    gcp-simple/          # GCP customer-managed VPC template (see template README)
```

## Troubleshooting

### Terraform not found after install
Restart the app. Terraform is installed to `~/.databricks-deployer/bin/`.

### Azure CLI not detected
Run `az login` in terminal first, then restart the app.

### AWS SSO session expired
Run `aws sso login --profile <profile>` to refresh.

### GCP ADC not working
Run `gcloud auth application-default login` in terminal, then restart the app. If using impersonation, ensure the service account exists and your user has the `Service Account Token Creator` role on it.

### GCP service account missing permissions
The app creates a custom role with the required permissions. If validation reports missing permissions, some (like `storage.buckets.setIamPolicy`) are bucket-level and cannot be validated at the project level -- they are still present in the role.

### Deployment stuck
Check the logs in the deployment folder. You can run `terraform apply` manually from there.

### Unity Catalog permissions error
Ensure your identity has metastore admin or the required grants (`CREATE CATALOG`, `CREATE EXTERNAL LOCATION`) on the existing metastore.

### GCP destroy fails on VPC deletion
Databricks creates firewall rules in the VPC that are not managed by Terraform. Delete them manually with `gcloud compute firewall-rules list --filter="network:<vpc-name>" --format="value(name)" | xargs -I {} gcloud compute firewall-rules delete {} --quiet`, then re-run destroy.

### Azure identity validation fails
Run `az login` and verify you have Databricks Account Admin. Test with:
```bash
az account get-access-token --resource 2ff814a6-3304-4ab8-85cb-cd0e6f879c1d
```

### Databricks token cache issues
Delete `~/.databricks/token-cache.json` and re-run `databricks auth login`.
