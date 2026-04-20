# Security Overview — Databricks Deployer

This document addresses common security questions from Solutions Architects and customers about how the Databricks Deployer desktop application handles authentication, credentials, data privacy, and the embedded AI assistant.

---

## Architecture Summary

Databricks Deployer is a **locally-installed desktop application** built with Tauri (Rust backend + React frontend). It runs entirely on the user's machine. There is no hosted backend, no SaaS component, and no telemetry or data collection by the app vendor.

The app orchestrates Terraform to deploy Databricks workspaces on AWS, Azure, or GCP. It collects cloud and Databricks credentials through a guided wizard, validates them via direct API calls, then generates `terraform.tfvars` and runs `terraform apply` as a local child process.

```
┌──────────────────────────────────────────────┐
│              User's Machine                  │
│                                              │
│  ┌──────────────┐    ┌───────────────────┐   │
│  │  React UI    │◄──►│  Rust Backend     │   │
│  │  (WebView)   │IPC │  (Tauri)          │   │
│  └──────────────┘    └─────┬─────────────┘   │
│                            │                 │
│                    ┌───────┴────────┐        │
│                    │  Terraform     │        │
│                    │  (child proc)  │        │
│                    └───────┬────────┘        │
│                            │                 │
└────────────────────────────┼─────────────────┘
                             │ HTTPS
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
         Cloud APIs    Databricks APIs   LLM APIs
        (AWS/Azure/GCP) (Account/WS)   (optional)
```

---

## 1. Authentication — Cloud Providers

The app supports three cloud providers with standard, customer-approved authentication methods. **No proprietary authentication mechanism is used.** The app delegates to the same CLI tools and SDKs that customers already trust.

### AWS

| Method | How it works |
|--------|-------------|
| **CLI Profile** (recommended) | Sets `AWS_PROFILE` env var. Supports SSO-enabled profiles. No static keys leave the user's machine. |
| **Access Keys** | Sets `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_SESSION_TOKEN` as environment variables for the Terraform process. |

Conflict prevention: when one method is active, the env vars for the other are explicitly cleared to prevent stale shell values from leaking in.

### Azure

| Method | How it works |
|--------|-------------|
| **Azure CLI** (recommended) | Relies on the active `az login` session. Sets `ARM_TENANT_ID` and `ARM_SUBSCRIPTION_ID`. No client secrets are involved. |
| **Service Principal** | Additionally sets `ARM_CLIENT_ID` and `ARM_CLIENT_SECRET` as env vars for the `azurerm` Terraform provider. |

### GCP

| Method | How it works |
|--------|-------------|
| **Application Default Credentials** (recommended) | Uses the user's `gcloud auth` session with service account impersonation. Sets `GOOGLE_APPLICATION_CREDENTIALS` and `GOOGLE_OAUTH_ACCESS_TOKEN`. No private keys leave the user's machine. |
| **Service Account JSON Key** | The JSON key content is passed as `GOOGLE_CREDENTIALS` env var and as a Terraform variable. |

---

## 2. Authentication — Databricks

Databricks authentication is **independent** from cloud authentication (except on GCP where the service account is shared). Three methods are supported:

| Method | Mechanism | Credentials delivery |
|--------|-----------|---------------------|
| **Azure Identity** (`azure-cli`) | Reuses the user's Azure AD session to obtain a Databricks token. No separate Databricks credentials needed. | `auth_type` written to `terraform.tfvars` |
| **CLI Profile** (`databricks-cli`) | Uses an existing profile from `~/.databrickscfg` (OAuth or PAT-based). Supports SSO login. | Profile name in tfvars + `DATABRICKS_CONFIG_PROFILE` env var |
| **Service Principal** (`oauth-m2m`) | Client ID + client secret exchanged for an OAuth token via the Databricks OIDC endpoint. | On AWS: env vars (`DATABRICKS_CLIENT_ID/SECRET`). On Azure: written to `terraform.tfvars` |

### Pre-deploy validation

Before any Terraform operation, the app validates credentials by making direct API calls to the Databricks account endpoint:
- SP: OAuth2 client_credentials grant → SCIM API call
- CLI profile: `databricks account users list --profile {name}`
- Azure Identity: `az account get-access-token` → SCIM API call
- GCP: ID token exchange → accounts API call

This ensures credentials work **before** infrastructure changes are attempted.

---

## 3. Credential Storage & Handling

### In-memory (during wizard flow)
Credentials entered in the UI live in React component state. They are never written to `localStorage` or `sessionStorage`. They are passed to the Rust backend via Tauri IPC only when the user triggers a save or deploy action.

### On disk (after save)

| What | Where | Encrypted? |
|------|-------|-----------|
| `terraform.tfvars` (may contain DB client secret, GCP SA JSON) | `{app_data}/deployments/{name}/terraform.tfvars` | No — plaintext. Protected by OS file permissions. This is standard Terraform behavior. |
| Databricks CLI profile (if app creates one) | `~/.databrickscfg` | No — plaintext. Standard Databricks CLI behavior. |
| AI assistant API keys | `{app_data}/assistant-settings.json` | **Yes — AES-256-GCM** with a locally-generated 32-byte key stored in `{app_data}/assistant-keyfile` |
| GitHub OAuth token (if used for repo push) | `{app_data}/github-settings.json` | **Yes — AES-256-GCM** with a separate keyfile |

### What is NOT stored
- Cloud credentials (AWS keys, Azure SP secrets, GCP tokens) are **not** persisted by the app outside of `terraform.tfvars`.
- No credentials are transmitted to any server controlled by the app vendor.
- No telemetry or analytics collect credential data.

### Git integration safety
When the app initializes a git repository for a deployment, it automatically adds `.gitignore` rules to exclude sensitive files:
```
*.tfvars
*.tfstate
*.tfstate.backup
.terraform/
```

---

## 4. Network Communication

The app makes **no outbound calls** to any vendor-operated infrastructure. All network traffic goes directly to the services the customer already trusts:

| Destination | Purpose | When |
|-------------|---------|------|
| Cloud provider APIs (AWS STS, Azure RM, Google IAM) | Credential validation, resource listing | Pre-deploy checks |
| Databricks account/workspace APIs | Credential validation, metastore detection, permission assignment | Pre-deploy and during Terraform |
| `registry.terraform.io` / `releases.hashicorp.com` | Terraform provider download, connectivity check | First deploy or provider update |
| `github.com` / `api.github.com` | Optional: repo creation, code push, app update check | Only if user opts into GitHub integration |
| LLM provider APIs (see Section 5) | Optional: AI assistant chat | Only if user configures the assistant |

### Proxy & TLS
- The app detects OS-level proxy settings (environment variables and, on Windows, registry) and applies them to all HTTP clients and Terraform subprocesses.
- HTTPS connections use `native-tls`, which trusts the **OS certificate store** — important for corporate environments with TLS inspection proxies.

---

## 5. AI Assistant — Security & Privacy

### Overview

The app bundles an **optional** AI assistant that provides contextual help (explaining screens, troubleshooting errors, answering Databricks questions). It is **not** required for deploying workspaces and can be completely ignored.

### Key security properties

| Property | Detail |
|----------|--------|
| **No vendor backend** | Chat traffic goes directly from the user's machine to the chosen LLM provider. There is no intermediary server. |
| **User-supplied API keys** | The user provides their own API key. The app does not ship with any baked-in LLM credentials. |
| **API keys encrypted at rest** | AES-256-GCM encryption with a locally-generated key. Keys are decrypted only in memory for the duration of a chat request. |
| **No customer credentials sent** | The app programmatically builds the chat context. It sends only: cloud provider name (e.g., "azure"), template name (e.g., "azure-simple"), current screen name, and a static knowledge base. **No AWS keys, Databricks tokens, subscription IDs, or tfvars values are included.** |
| **User-typed content** | Anything the user **manually types** into the chat is sent to the LLM provider. Users should avoid pasting secrets into the chat. |
| **Conversation history** | The last 6 messages are saved locally in `assistant-settings.json` (plaintext). Not transmitted anywhere except to the LLM on subsequent requests as context. |
| **Low temperature** | The assistant uses `temperature = 0.05` to minimize hallucination. |

### Supported providers

| Provider | API Endpoint | Default Model | Auth Header |
|----------|-------------|---------------|-------------|
| GitHub Models (free) | `models.github.ai` | `openai/gpt-4o-mini` (configurable) | `Authorization: Bearer {PAT}` |
| OpenAI | `api.openai.com` | `gpt-4o-mini` | `Authorization: Bearer {key}` |
| Anthropic Claude | `api.anthropic.com` | `claude-3-5-haiku-latest` | `x-api-key: {key}` |

### What the LLM receives

Each chat request includes:

1. **System prompt**: A static knowledge base (`assistant-knowledge.md`) with product documentation, screen descriptions, and troubleshooting guides. No customer-specific data.
2. **Screen context**: A static description of the current wizard step (e.g., "The user is configuring cloud credentials").
3. **State metadata**: Only `Cloud provider: {name}` and `Template: {name}`. Explicitly sanitized — the code comment reads `"no secrets"`.
4. **Chat history**: Up to 6 prior messages (user questions + assistant replies).
5. **User message**: The current question typed by the user.

### Opting out

If the AI assistant is never configured (no API key provided), it makes zero network calls. The chat icon appears in the UI but is non-functional until setup is completed.

---

## 6. Terraform Execution Security

- Terraform runs as a **local child process** with the user's OS permissions. No elevated privileges are required.
- The app bundles no Terraform binary — it downloads the official HashiCorp release or uses an existing installation found in `PATH`.
- Terraform state (`terraform.tfstate`) stays on the user's local disk unless the user configures a remote backend.
- The app does not modify Terraform provider code at runtime. Templates are static `.tf` files copied to the deployment directory.

---

## 7. Application Update Security

- The app checks for updates via the **GitHub Releases API** (`api.github.com/repos/.../releases/latest`).
- No auto-update mechanism is present. Users download and install new versions manually.
- The app binary is code-signed (macOS) for integrity verification.

---

## 8. Summary of Security Controls

| Area | Control |
|------|---------|
| Credential transport | HTTPS only, native TLS, proxy-aware |
| Credential storage | Encrypted (assistant/GitHub tokens), OS-protected files (tfvars) |
| Credential scope | Minimal — only what Terraform needs. Cloud and Databricks auth are independent. |
| Network destinations | Customer-controlled cloud/Databricks APIs only. No vendor infrastructure. |
| AI assistant privacy | User-supplied keys, no customer data in prompts, optional and off by default |
| Terraform isolation | Local child process, no remote state by default, gitignore for sensitive files |
| Pre-deploy validation | Direct API calls to verify credentials before any infrastructure changes |
| Data residency | Everything stays on the user's machine. No data leaves except to customer-chosen APIs. |

---

## FAQ for Solutions Architects

**Q: Does the app phone home or send telemetry?**
A: No. There is no analytics, telemetry, or usage tracking. The only outbound calls are to cloud/Databricks APIs that the customer explicitly configures, and optionally to LLM providers if the user sets up the assistant.

**Q: Can the app access customer data in Databricks?**
A: The app creates workspaces and configures Unity Catalog. It does not read, query, or process any data stored in Databricks tables, volumes, or notebooks.

**Q: Where are credentials stored after deployment?**
A: In `terraform.tfvars` on the user's local filesystem, protected by OS file permissions. The app's `.gitignore` prevents accidental commits. For long-term credential security, customers should use CLI-based authentication (AWS profiles, Azure CLI, GCP ADC) which avoid storing static secrets entirely.

**Q: Is the AI assistant sending my credentials to OpenAI/GitHub/Anthropic?**
A: No. The app explicitly sanitizes the context sent to the LLM. Only the cloud name and template name are included — no keys, tokens, account IDs, or configuration values. However, users should avoid pasting credentials into the chat input.

**Q: Does the app require admin/root privileges?**
A: No. It runs with standard user permissions. Terraform operations require only the cloud/Databricks permissions associated with the provided credentials.

**Q: Can I use this behind a corporate proxy with TLS inspection?**
A: Yes. The app detects OS proxy settings and uses the OS certificate store (`native-tls`), so corporate CA certificates installed on the machine are automatically trusted.
