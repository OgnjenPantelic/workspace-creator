# =============================================================================
# Azure Configuration
# =============================================================================

tenant_id = ""
azure_subscription_id = ""
tags = {
}
resource_group_name = ""

# =============================================================================
# Databricks Configuration
# =============================================================================

workspace_name = ""
admin_user = ""
root_storage_name = ""
location = "northeurope"
databricks_account_id = ""

# =============================================================================
# Unity Catalog Metastore Configuration
# =============================================================================

existing_metastore_id = ""
new_metastore_name = ""

# =============================================================================
# Network Configuration
# =============================================================================

create_new_vnet = true
vnet_name = ""
vnet_resource_group_name = ""
cidr = "10.0.0.0/20"
subnet_public_cidr = "10.0.1.0/24"
subnet_private_cidr = "10.0.2.0/24"