// Cloud provider constants
export const CLOUDS = {
  AWS: "aws",
  AZURE: "azure",
  GCP: "gcp",
} as const;

// Polling intervals (in milliseconds)
export const POLLING = {
  STATUS_INTERVAL: 1000,      // 1 second for deployment status
  ROLLBACK_INTERVAL: 500,     // 500ms for rollback status
  SSO_CHECK_INTERVAL: 1000,   // 1 second for SSO polling
  SSO_MAX_ATTEMPTS: 60,       // 60 seconds max wait for SSO
  MIN_LOADING_TIME: 1000,     // Minimum loading time for UX
} as const;

// Display names for clouds
export const CLOUD_DISPLAY_NAMES: Record<string, string> = {
  [CLOUDS.AWS]: "AWS",
  [CLOUDS.AZURE]: "Azure",
  [CLOUDS.GCP]: "GCP",
};

// Variable display name mappings
export const VARIABLE_DISPLAY_NAMES: Record<string, string> = {
  // Workspace
  prefix: "Workspace Name",
  workspace_name: "Workspace Name",
  admin_user: "Admin Email",
  root_storage_name: "Storage Account Name",
  workspace_sku: "Pricing Tier",
  // Cloud-specific
  region: "Region",
  location: "Region",
  resource_group_name: "Resource Group",
  // Network
  cidr: "VNet CIDR",
  cidr_block: "VPC CIDR",
  subnet_public_cidr: "Public Subnet CIDR",
  subnet_private_cidr: "Private Subnet CIDR",
  create_new_vnet: "Create New VNet",
  vnet_name: "Existing VNet Name",
  vnet_resource_group_name: "VNet Resource Group",
  // Other
  tags: "Resource Tags",
  // Advanced
  existing_vpc_id: "Existing VPC ID",
  existing_subnet_ids: "Existing Subnet IDs",
  existing_security_group_id: "Existing Security Group ID",
  metastore_id: "Existing Metastore ID",
  existing_metastore_id: "Existing Metastore ID",
};

// Variable description overrides (takes precedence over Terraform description)
export const VARIABLE_DESCRIPTION_OVERRIDES: Record<string, string> = {
  // Workspace
  prefix: "Name for your Databricks workspace. Also used as prefix for storage, credentials, and network resources.",
  workspace_name: "Name for your Databricks workspace.",
  admin_user: "Email address of the workspace admin. Must already exist in your Databricks account.",
  root_storage_name: "Name for the storage account (Azure) or S3 bucket (AWS). Lowercase letters and numbers only, 3-24 characters.",
  workspace_sku: "Pricing tier for the workspace. Premium is required for Unity Catalog.",
  // Cloud-specific
  region: "AWS region where your Databricks workspace will be deployed.",
  location: "Azure region where your Databricks workspace will be deployed.",
  resource_group_name: "Azure resource group to deploy the workspace into. Select an existing one or enter a new name.",
  tags: "Optional key-value pairs to tag all created resources for cost tracking and organization.",
  // Network - Azure
  create_new_vnet: "Enable to create a new VNet, or disable to use an existing VNet. New subnets will be created in either case.",
  vnet_name: "Name of your existing VNet where Databricks subnets will be created.",
  vnet_resource_group_name: "Resource group containing the VNet. Usually the same as the main resource group.",
  cidr: "Address space for the new VNet (e.g., 10.0.0.0/20). Must be large enough for the subnets.",
  subnet_public_cidr: "CIDR range for the public (host) subnet within the VNet address space.",
  subnet_private_cidr: "CIDR range for the private (container) subnet within the VNet address space.",
  // Network - AWS
  cidr_block: "Address space for the new VPC (e.g., 10.4.0.0/16). Subnets will be automatically allocated within this range.",
  // Advanced
  existing_vpc_id: "Use an existing VPC instead of creating a new one. Leave empty for auto-creation.",
  existing_subnet_ids: "Use existing subnets. Required if using an existing VPC.",
  existing_security_group_id: "Use an existing security group. Required if using an existing VPC.",
  metastore_id: "Use an existing Unity Catalog metastore. Leave empty to auto-detect or create a new one.",
  existing_metastore_id: "Use an existing Unity Catalog metastore. Leave empty to auto-detect or create a new one.",
};

// Databricks supported AWS regions
export const AWS_REGIONS: { value: string; label: string }[] = [
  // North America
  { value: "us-east-1", label: "US East (N. Virginia)" },
  { value: "us-east-2", label: "US East (Ohio)" },
  { value: "us-west-1", label: "US West (N. California)" },
  { value: "us-west-2", label: "US West (Oregon)" },
  { value: "ca-central-1", label: "Canada (Central)" },
  // South America
  { value: "sa-east-1", label: "South America (São Paulo)" },
  // Europe
  { value: "eu-central-1", label: "Europe (Frankfurt)" },
  { value: "eu-west-1", label: "Europe (Ireland)" },
  { value: "eu-west-2", label: "Europe (London)" },
  { value: "eu-west-3", label: "Europe (Paris)" },
  // Asia Pacific
  { value: "ap-northeast-1", label: "Asia Pacific (Tokyo)" },
  { value: "ap-northeast-2", label: "Asia Pacific (Seoul)" },
  { value: "ap-south-1", label: "Asia Pacific (Mumbai)" },
  { value: "ap-southeast-1", label: "Asia Pacific (Singapore)" },
  { value: "ap-southeast-2", label: "Asia Pacific (Sydney)" },
  { value: "ap-southeast-3", label: "Asia Pacific (Jakarta)" },
];

// Databricks supported Azure regions
export const AZURE_REGIONS: { value: string; label: string }[] = [
  // North America
  { value: "eastus", label: "East US" },
  { value: "eastus2", label: "East US 2" },
  { value: "westus", label: "West US" },
  { value: "westus2", label: "West US 2" },
  { value: "westus3", label: "West US 3" },
  { value: "centralus", label: "Central US" },
  { value: "northcentralus", label: "North Central US" },
  { value: "southcentralus", label: "South Central US" },
  { value: "canadacentral", label: "Canada Central" },
  { value: "canadaeast", label: "Canada East" },
  // South America
  { value: "brazilsouth", label: "Brazil South" },
  { value: "mexicocentral", label: "Mexico Central" },
  // Europe
  { value: "northeurope", label: "North Europe (Ireland)" },
  { value: "westeurope", label: "West Europe (Netherlands)" },
  { value: "uksouth", label: "UK South" },
  { value: "ukwest", label: "UK West" },
  { value: "francecentral", label: "France Central" },
  { value: "germanywestcentral", label: "Germany West Central" },
  { value: "swedencentral", label: "Sweden Central" },
  { value: "norwayeast", label: "Norway East" },
  { value: "switzerlandnorth", label: "Switzerland North" },
  // Asia Pacific
  { value: "australiaeast", label: "Australia East" },
  { value: "australiasoutheast", label: "Australia Southeast" },
  { value: "australiacentral", label: "Australia Central" },
  { value: "japaneast", label: "Japan East" },
  { value: "japanwest", label: "Japan West" },
  { value: "koreacentral", label: "Korea Central" },
  { value: "eastasia", label: "East Asia (Hong Kong)" },
  { value: "southeastasia", label: "Southeast Asia (Singapore)" },
  { value: "centralindia", label: "Central India" },
  { value: "southindia", label: "South India" },
  // Middle East
  { value: "qatarcentral", label: "Qatar Central" },
  { value: "uaenorth", label: "UAE North" },
];

// Variables to exclude from the configuration form (collected in credentials screens)
export const EXCLUDE_VARIABLES = [
  "databricks_account_id",
  "databricks_client_id",
  "databricks_client_secret",
  "aws_access_key_id",
  "aws_secret_access_key",
  "aws_session_token",
  "tenant_id",
  "azure_tenant_id",
  "azure_subscription_id",
  "azure_client_id",
  "azure_client_secret",
  "create_new_resource_group",
] as const;
