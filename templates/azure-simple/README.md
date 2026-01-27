# Azure Databricks Workspace - Standard VNet Injected

This Terraform template deploys a production-grade Databricks workspace on Azure with secure networking.

## 🚀 Quick Start (2 minutes)

### Step 1: Create Virtual Environment (First Time Only)
```bash
# macOS/Linux
python3 -m venv venv
source venv/bin/activate

# Windows (Command Prompt)
python -m venv venv
venv\Scripts\activate

# Windows (PowerShell)
python -m venv venv
venv\Scripts\Activate.ps1
```

### Step 2: Install Python Dependencies
```bash
# After activating virtual environment above
pip install -r requirements.txt
```

### Step 3: Run the Deployment UI
```bash
# Make sure virtual environment is activated (you should see (venv) in your prompt)
python deploy.py
```

The web interface will automatically open at **http://localhost:8081**

### Step 3: Configure and Deploy
1. Fill in your configuration values in the web UI
2. Click "Save Configuration"
3. Click 4Run Terraform Plan" to preview changes
4. Click "Apply Configuration" to deploy

That's it! ✨

---

## 📋 Prerequisites

Before running the deployment, ensure you have:

### Required Software
- **Python 3.7+** - [Download here](https://www.python.org/downloads/)
- **Terraform CLI** - [Download here](https://www.terraform.io/downloads)
- **Azure CLI** - [Download here](https://docs.microsoft.com/en-us/cli/azure/install-azure-cli)

### Azure Setup
1. Log in to Azure CLI:
   ```bash
   az login
   ```

2. Verify you have the correct subscription selected:
   ```bash
   az account show
   ```

### Databricks Account
You'll need the following from your Databricks account:
- **Account ID** (found in account console)
- **Client ID** (from service principal)
- **Client Secret** (from service principal)

---

## 🏗️ What Will Be Deployed

This template creates:
- **Databricks Workspace** with VNet injection for network isolation
- **Virtual Network (VNet)** with private and public subnets
- **Network Security Groups (NSGs)** controlling traffic
- **NAT Gateway** for secure outbound internet access
- **Azure Resource Group** for organized resource management
- **Unity Catalog Metastore** (optional, if configured)

---

## 🎯 Deployment Options

### Option 1: Web UI (Recommended)
Use the local deployment UI as described in Quick Start above.

### Option 2: Manual Terraform Commands
If you prefer command-line:

```bash
# Initialize Terraform
terraform init

# Review the deployment plan
terraform plan

# Apply the configuration
terraform apply
```

### Option 3: Edit Configuration File Manually
Edit `terraform.tfvars` directly, then run terraform commands.

---

## ⚙️ Configuration Variables

Key variables to configure in the UI or `terraform.tfvars`:

| Variable | Description | Example |
|----------|-------------|---------|
| `databricks_account_id` | Your Databricks account ID | `00000000-0000-0000-0000-000000000000` |
| `databricks_client_id` | Service principal client ID | `00000000-0000-0000-0000-000000000000` |
| `databricks_client_secret` | Service principal secret | `your-secret-here` |
| `workspace_name` | Name for your workspace | `my-databricks-workspace` |
| `location` | Azure region | `East US` |
| `vnet_cidr` | Virtual network CIDR | `10.0.0.0/16` |
| `resource_group_name` | Resource group name | `my-databricks-rg` |

---

## 🔒 Security Best Practices

- **Never commit** `terraform.tfstate` or `terraform.tfvars` with secrets to version control
- Use **remote state storage** (Azure Storage) for team collaboration
- Review all configuration before running `terraform apply`
- Ensure you have appropriate **Azure permissions** (Contributor or Owner role)

---

## 🆘 Troubleshooting

### Port 8081 already in use
Kill the process using the port:
```bash
# macOS/Linux
lsof -ti:8081 | xargs kill -9

# Windows
netstat -ano | findstr :8081
taskkill /PID <PID> /F
```

### Flask not found
Install dependencies:
```bash
pip3 install -r requirements.txt
```

### Azure CLI not authenticated
Run `az login` and follow the prompts.

### Terraform errors
- Ensure Terraform is installed: `terraform version`
- Run `terraform init` in the template directory
- Check Azure permissions

---

## 📚 Additional Resources

- [Databricks on Azure Documentation](https://docs.databricks.com/administration-guide/cloud-configurations/azure/index.html)
- [Terraform Azure Provider Docs](https://registry.terraform.io/providers/hashicorp/azurerm/latest/docs)
- [Azure CLI Reference](https://docs.microsoft.com/en-us/cli/azure/)

---

**Questions or issues?** Contact your Databricks representative or check the Databricks documentation.
