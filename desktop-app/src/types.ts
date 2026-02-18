export interface DependencyStatus {
  name: string;
  installed: boolean;
  version: string | null;
  required: boolean;
  install_url: string;
}

export interface Template {
  id: string;
  name: string;
  cloud: string;
  description: string;
  features: string[];
}

export interface TerraformVariable {
  name: string;
  description: string;
  var_type: string;
  default: string | null;
  required: boolean;
  sensitive: boolean;
  validation: string | null;
}

export interface DeploymentStatus {
  running: boolean;
  command: string | null;
  output: string;
  success: boolean | null;
  can_rollback: boolean;
}

export interface CloudCredentials {
  // AWS
  aws_profile?: string;
  aws_access_key_id?: string;
  aws_secret_access_key?: string;
  aws_session_token?: string;
  aws_region?: string;
  // Azure
  azure_tenant_id?: string;
  azure_subscription_id?: string;
  azure_client_id?: string;
  azure_client_secret?: string;
  azure_account_email?: string;           // Email/user from Azure account
  azure_databricks_use_identity?: boolean; // Use Azure identity for Databricks (account admin flow)
  // GCP
  gcp_project_id?: string;
  gcp_credentials_json?: string;    // Service account JSON content
  gcp_use_adc?: boolean;            // Use Application Default Credentials
  gcp_oauth_token?: string;         // OAuth access token from gcloud auth print-access-token
  gcp_service_account_email?: string; // Service account email for Databricks provider
  // Databricks
  databricks_account_id?: string;
  databricks_client_id?: string;
  databricks_client_secret?: string;
  databricks_profile?: string;      // Profile name from ~/.databrickscfg
  databricks_auth_type?: string;    // "profile" or "credentials"
  // Cloud identifier
  cloud?: string;                   // "aws", "azure", or "gcp"
}

export interface AwsProfile {
  name: string;
  is_sso: boolean;
}

export interface AwsIdentity {
  account: string;
  arn: string;
  user_id: string;
}

export interface AzureSubscription {
  id: string;
  name: string;
  is_default: boolean;
  tenant_id: string;
}

export interface AzureAccount {
  user: string;
  tenant_id: string;
  subscription_id: string;
  subscription_name: string;
}

export interface DatabricksProfile {
  name: string;
  host: string;
  account_id: string | null;
  has_client_credentials: boolean;
  has_token: boolean;
  cloud: string;
}

export interface UnityCatalogConfig {
  enabled: boolean;
  catalog_name: string;
  storage_name: string;  // S3 bucket name or Azure storage account name
  metastore_id: string;  // Detected or user-provided existing metastore ID
}

export interface MetastoreInfo {
  exists: boolean;
  metastore_id: string | null;
  metastore_name: string | null;
  region: string | null;
}

export interface UCPermissionCheck {
  metastore: MetastoreInfo;
  has_create_catalog: boolean;
  has_create_external_location: boolean;
  has_create_storage_credential: boolean;
  can_create_catalog: boolean;  // true if all permissions present OR no metastore (will be created)
  message: string;
}

export interface CloudPermissionCheck {
  has_all_permissions: boolean;
  checked_permissions: string[];
  missing_permissions: string[];
  message: string;
  is_warning: boolean;  // true = soft warning (can continue), false = hard block
}

export interface GcpValidation {
  valid: boolean;
  project_id: string | null;
  account: string | null;
  message: string;
  oauth_token: string | null;
  impersonated_account: string | null;
}

export type AppScreen = 
  | 'welcome'
  | 'dependencies'
  | 'cloud-selection'
  | 'databricks-credentials'
  | 'aws-credentials'
  | 'azure-credentials'
  | 'gcp-credentials'
  | 'template-selection'
  | 'configuration'
  | 'unity-catalog-config'
  | 'deployment';

// AI Assistant types
export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface AssistantSettings {
  active_provider: string;
  configured: boolean;
  github_model?: string;
  cached_models?: [string, string][];
  models_cache_timestamp?: number;
  chat_history?: ChatMessage[];
  has_github_key: boolean;
  has_openai_key: boolean;
  has_claude_key: boolean;
}

export interface ModelOption {
  id: string;
  name: string;
}
