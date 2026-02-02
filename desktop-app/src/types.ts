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
  // Databricks
  databricks_account_id?: string;
  databricks_client_id?: string;
  databricks_client_secret?: string;
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

export type AppScreen = 
  | 'welcome'
  | 'dependencies'
  | 'cloud-selection'
  | 'databricks-credentials'
  | 'aws-credentials'
  | 'azure-credentials'
  | 'template-selection'
  | 'configuration'
  | 'deployment';
