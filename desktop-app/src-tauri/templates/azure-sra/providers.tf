provider "azurerm" {
  features {}
  subscription_id = var.subscription_id
}

provider "azapi" {
  subscription_id = var.subscription_id
}

provider "databricks" {
  host          = "https://accounts.azuredatabricks.net"
  account_id    = var.databricks_account_id
  auth_type     = var.databricks_auth_type
  client_id     = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_id : null
  client_secret = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_secret : null
  profile       = var.databricks_auth_type == "databricks-cli" ? var.databricks_profile : null
}

provider "databricks" {
  alias         = "hub"
  host          = var.create_hub && length(module.webauth_workspace) > 0 ? module.webauth_workspace[0].workspace_url : "https://placeholder.azuredatabricks.net"
  auth_type     = var.databricks_auth_type
  client_id     = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_id : null
  client_secret = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_secret : null
}

# Spoke provider (required for creating a catalog in the spoke workspace)
provider "databricks" {
  alias         = "spoke"
  host          = module.spoke_workspace.workspace_url
  auth_type     = var.databricks_auth_type
  client_id     = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_id : null
  client_secret = var.databricks_auth_type == "oauth-m2m" ? var.databricks_client_secret : null
}

# These blocks are not required by terraform, but they are here to silence TFLint warnings
provider "null" {}

provider "time" {}
