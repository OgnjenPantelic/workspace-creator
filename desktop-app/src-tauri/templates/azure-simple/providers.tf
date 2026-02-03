# Azure credentials via environment variables (ARM_TENANT_ID, ARM_SUBSCRIPTION_ID, etc.)
# or Azure CLI authentication
provider "azurerm" {
  subscription_id = var.azure_subscription_id
  tenant_id = var.tenant_id
  features {}
}

# Databricks workspace provider - uses Azure CLI authentication
# This authenticates to the workspace using the same Azure credentials
provider "databricks" {
  host                        = azurerm_databricks_workspace.this.workspace_url
  azure_workspace_resource_id = azurerm_databricks_workspace.this.id
  auth_type                   = "azure-cli"
}

# Databricks account provider
# auth_type is set dynamically:
# - "oauth-m2m" for service principal credentials
# - "databricks-cli" for CLI profile authentication
provider "databricks" {
  alias      = "accounts"
  host       = "https://accounts.azuredatabricks.net"
  account_id = var.databricks_account_id
  auth_type  = var.databricks_auth_type
}