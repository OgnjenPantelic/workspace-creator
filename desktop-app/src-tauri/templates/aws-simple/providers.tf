# AWS credentials via environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
provider "aws" {
  region = var.region
  default_tags {
    tags = merge(var.tags, { Project = var.prefix })
  }
}

# Databricks credentials via environment variables (DATABRICKS_CLIENT_ID, DATABRICKS_CLIENT_SECRET)
provider "databricks" {
  alias      = "mws"
  host       = "https://accounts.cloud.databricks.com"
  account_id = var.databricks_account_id
}

provider "databricks" {
  alias = "workspace"
  host  = databricks_mws_workspaces.this.workspace_url
}
