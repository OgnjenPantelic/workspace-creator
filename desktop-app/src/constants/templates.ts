export const VARIABLE_DISPLAY_NAMES: Record<string, string> = {
  // Workspace
  prefix: "Workspace Name",
  workspace_name: "Workspace Name",
  databricks_workspace_name: "Workspace Name",
  admin_user: "Admin Email",
  root_storage_name: "Storage Account Name",
  workspace_sku: "Pricing Tier",
  // Cloud-specific - AWS
  region: "Region",
  // Cloud-specific - Azure
  location: "Region",
  resource_group_name: "Resource Group",
  // Cloud-specific - GCP
  google_region: "Region",
  google_project_name: "Project ID",
  google_service_account_email: "Service Account Email",
  // Network - Azure
  cidr: "VNet CIDR",
  subnet_public_cidr: "Public Subnet CIDR",
  subnet_private_cidr: "Private Subnet CIDR",
  create_new_vnet: "Create New VNet",
  vnet_name: "Existing VNet Name",
  vnet_resource_group_name: "VNet Resource Group",
  // Network - AWS
  cidr_block: "VPC CIDR",
  // Network - GCP
  subnet_cidr: "Subnet CIDR",
  // Other
  tags: "Resource Tags",
  // Advanced
  existing_vpc_id: "Existing VPC ID",
  existing_subnet_ids: "Existing Subnet IDs",
  existing_security_group_id: "Existing Security Group ID",
  metastore_id: "Existing Metastore ID",
  existing_metastore_id: "Existing Metastore ID",
};

export const VARIABLE_DESCRIPTION_OVERRIDES: Record<string, string> = {
  // Workspace
  prefix: "Name for your Databricks workspace. Also used as prefix for storage, credentials, and network resources.",
  workspace_name: "Name for your Databricks workspace.",
  databricks_workspace_name: "Name for your Databricks workspace.",
  admin_user: "Email address of the workspace admin. Must already exist in your Databricks account.",
  root_storage_name: "Storage account (Azure: 3-24 chars) or S3 bucket (AWS: 3-63 chars). Lowercase letters and numbers only.",
  workspace_sku: "Pricing tier for the workspace. Premium is required for Unity Catalog.",
  // Cloud-specific - AWS
  region: "AWS region where your Databricks workspace will be deployed.",
  // Cloud-specific - Azure
  location: "Azure region where your Databricks workspace will be deployed.",
  resource_group_name: "Azure resource group to deploy the workspace into. Select an existing one or enter a new name.",
  // Cloud-specific - GCP
  google_region: "GCP region where your Databricks workspace will be deployed.",
  google_project_name: "GCP project ID for workspace resources.",
  google_service_account_email: "Service account email used for authentication. Must have Owner role and be added to Databricks Account Console with admin role.",
  tags: "Optional key-value pairs to tag/label all created resources for cost tracking and organization.",
  // Network - Azure
  create_new_vnet: "Enable to create a new VNet, or disable to use an existing VNet. New subnets will be created in either case.",
  vnet_name: "Name of your existing VNet where Databricks subnets will be created.",
  vnet_resource_group_name: "Resource group containing the VNet. Usually the same as the main resource group.",
  cidr: "Address space for the new VNet (e.g., 10.0.0.0/20). Must be large enough for the subnets.",
  subnet_public_cidr: "CIDR range for the public (host) subnet within the VNet address space.",
  subnet_private_cidr: "CIDR range for the private (container) subnet within the VNet address space.",
  // Network - AWS
  cidr_block: "Address space for the new VPC (e.g., 10.4.0.0/16). Subnets will be automatically allocated within this range.",
  // Network - GCP
  subnet_cidr: "CIDR range for the Databricks subnet (e.g., 10.0.0.0/16).",
  // Advanced
  existing_vpc_id: "Use an existing VPC instead of creating a new one. Leave empty for auto-creation.",
  existing_subnet_ids: "Use existing subnets. Required if using an existing VPC.",
  existing_security_group_id: "Use an existing security group. Required if using an existing VPC.",
  metastore_id: "Use an existing Unity Catalog metastore. Leave empty to auto-detect or create a new one.",
  existing_metastore_id: "Use an existing Unity Catalog metastore. Leave empty to auto-detect or create a new one.",
};

export const EXCLUDE_VARIABLES = [
  "databricks_account_id",
  "databricks_client_id",
  "databricks_client_secret",
  "databricks_profile",
  "databricks_auth_type",
  "aws_access_key_id",
  "aws_secret_access_key",
  "aws_session_token",
  "tenant_id",
  "azure_tenant_id",
  "azure_subscription_id",
  "azure_client_id",
  "azure_client_secret",
  "create_new_resource_group",
  // GCP variables - collected in credentials screen
  "gcp_project_id",
  "google_project",
  "google_project_name",
  "gcp_credentials_json",
  "google_service_account_email",
  // Unity Catalog variables - configured in dedicated UC setup screen
  "existing_metastore_id",
  "create_unity_catalog",
  "uc_catalog_name",
  "uc_storage_name",
  "uc_force_destroy",
] as const;
