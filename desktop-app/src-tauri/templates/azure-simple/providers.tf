# Azure credentials via environment variables (ARM_TENANT_ID, ARM_SUBSCRIPTION_ID, etc.)
# or Azure CLI authentication
provider "azurerm" {
  subscription_id = var.azure_subscription_id
  tenant_id = var.tenant_id
  features {}
}

# Databricks workspace provider - uses Azure CLI authentication
provider "databricks" {
  host = azurerm_databricks_workspace.this.workspace_url
}

# Databricks account provider - uses Databricks OAuth credentials (service principal)
# Explicitly use OAuth M2M to avoid conflict with Azure ARM_TENANT_ID env var
provider "databricks" {
  alias      = "accounts"
  host       = "https://accounts.azuredatabricks.net"
  account_id = var.databricks_account_id
  auth_type  = "oauth-m2m"
}