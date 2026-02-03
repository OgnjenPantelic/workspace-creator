# Azure Databricks Workspace - VNet Injected

Terraform template for deploying a Databricks workspace on Azure with secure networking.

## What Gets Deployed

- Databricks Workspace with VNet injection
- Virtual Network with private/public subnets
- Network Security Groups
- NAT Gateway
- Resource Group
- Unity Catalog resources (optional)

## Prerequisites

- Terraform CLI
- Azure CLI (`az login`)
- Databricks service principal with account admin privileges

## Usage

This template is designed to be used via the Databricks Deployer desktop app.

For manual deployment:

```bash
terraform init
terraform plan -var-file="terraform.tfvars"
terraform apply -var-file="terraform.tfvars"
```

## Variables

| Variable | Description |
|----------|-------------|
| `databricks_account_id` | Databricks account ID |
| `databricks_client_id` | Service principal client ID |
| `databricks_client_secret` | Service principal secret |
| `workspace_name` | Workspace name |
| `location` | Azure region |
| `resource_group_name` | Resource group name |
| `vnet_cidr` | VNet CIDR block |

## Security Notes

- Don't commit `terraform.tfstate` or `terraform.tfvars` with secrets
- Use remote state for team collaboration
- Ensure Contributor or Owner role on subscription
