# Azure credentials via environment variables (ARM_TENANT_ID, ARM_SUBSCRIPTION_ID, etc.)
# or Azure CLI authentication
provider "azurerm" {
  subscription_id = var.azure_subscription_id
  tenant_id = var.tenant_id
  features {}
}

# Databricks workspace provider
# - For databricks-cli (SSO): Uses Azure-native auth via azure_workspace_resource_id
# - For oauth-m2m (SP): Uses Databricks SP credentials directly
#   (SP must be added to workspace first via account provider - see databricks.tf)
provider "databricks" {
  host                        = azurerm_databricks_workspace.this.workspace_url
  # Azure auth for SSO profiles
  azure_workspace_resource_id = var.databricks_auth_type == "databricks-cli" ? azurerm_databricks_workspace.this.id : null
  # SP auth for oauth-m2m
  auth_type                   = var.databricks_auth_type == "oauth-m2m" ? "oauth-m2m" : null
  client_id                   = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_id : null
  client_secret               = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_secret : null
}

# Databricks account provider
# auth_type is set dynamically:
# - "oauth-m2m" for service principal credentials (uses client_id/client_secret vars)
# - "databricks-cli" for CLI profile authentication (uses profile var)
provider "databricks" {
  alias         = "accounts"
  host          = "https://accounts.azuredatabricks.net"
  account_id    = var.databricks_account_id
  auth_type     = var.databricks_auth_type
  # For oauth-m2m: use client credentials
  client_id     = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_id : null
  client_secret = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_secret : null
  # For databricks-cli: use profile
  profile       = var.databricks_auth_type == "databricks-cli" ? var.databricks_profile : null
}