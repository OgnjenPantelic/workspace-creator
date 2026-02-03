use crate::dependencies::{self, DependencyStatus};
use crate::terraform::{self, DeploymentStatus, TerraformVariable, CURRENT_PROCESS, DEPLOYMENT_STATUS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Stdio;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub cloud: String,
    pub description: String,
    pub features: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CloudCredentials {
    // AWS
    pub aws_profile: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
    pub aws_region: Option<String>,
    // Azure
    pub azure_tenant_id: Option<String>,
    pub azure_subscription_id: Option<String>,
    pub azure_client_id: Option<String>,
    pub azure_client_secret: Option<String>,
    // Databricks
    pub databricks_account_id: Option<String>,
    pub databricks_client_id: Option<String>,
    pub databricks_client_secret: Option<String>,
    pub databricks_profile: Option<String>,        // Profile name from ~/.databrickscfg
    pub databricks_auth_type: Option<String>,      // "profile" or "credentials"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwsProfile {
    pub name: String,
    pub is_sso: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwsIdentity {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

// Version marker to track template updates - increment when templates change
const TEMPLATES_VERSION: &str = "2.9.0";

pub fn setup_templates(app: &AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let templates_dir = app_data_dir.join("templates");
    let version_file = app_data_dir.join(".templates_version");
    
    // Check if we need to update templates
    let needs_update = if templates_dir.exists() {
        // Check version - if missing or different, update
        match fs::read_to_string(&version_file) {
            Ok(version) => version.trim() != TEMPLATES_VERSION,
            Err(_) => true, // No version file means old templates
        }
    } else {
        true // No templates dir means fresh install
    };
    
    if !needs_update {
        return Ok(());
    }
    
    // Remove old templates if they exist
    if templates_dir.exists() {
        // Remove the templates directory to do a fresh copy
        fs::remove_dir_all(&templates_dir).map_err(|e| format!("Failed to remove old templates: {}", e))?;
    }
    
    fs::create_dir_all(&templates_dir).map_err(|e| e.to_string())?;
    
    // Copy embedded templates
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;
    
    let source_templates = resource_dir.join("templates");
    
    // Try resource dir first (production), then fall back to dev location
    let templates_source = if source_templates.exists() {
        source_templates
    } else {
        // In dev mode, templates are in src-tauri/templates relative to workspace
        // Try to find the project root by going up from the executable
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut search_path = exe_path.parent();
        
        // Search upward for src-tauri directory (dev mode)
        let mut dev_templates = None;
        while let Some(path) = search_path {
            let candidate = path.join("src-tauri").join("templates");
            if candidate.exists() {
                dev_templates = Some(candidate);
                break;
            }
            search_path = path.parent();
        }
        
        match dev_templates {
            Some(path) => path,
            None => return Err("Templates not found in resource dir or src-tauri directory".to_string()),
        }
    };
    
    copy_dir_all(&templates_source, &templates_dir)?;
    
    // Write version file
    fs::write(&version_file, TEMPLATES_VERSION).map_err(|e| format!("Failed to write version: {}", e))?;
    
    Ok(())
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    
    Ok(())
}

fn get_templates_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("templates"))
}

fn get_deployments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let deployments_dir = app_data_dir.join("deployments");
    fs::create_dir_all(&deployments_dir).map_err(|e| e.to_string())?;
    Ok(deployments_dir)
}

/// Sanitize deployment name to prevent path traversal attacks
/// Only allows alphanumeric characters, hyphens, and underscores
fn sanitize_deployment_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Deployment name cannot be empty".to_string());
    }
    
    // Only allow alphanumeric, hyphens, and underscores
    let sanitized: String = name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    
    if sanitized.is_empty() {
        return Err("Deployment name contains no valid characters".to_string());
    }
    
    // Ensure it doesn't start with dots or hyphens (security + convention)
    if sanitized.starts_with('-') {
        return Err("Deployment name cannot start with a hyphen".to_string());
    }
    
    // Limit length to prevent filesystem issues
    if sanitized.len() > 200 {
        return Err("Deployment name is too long (max 200 characters)".to_string());
    }
    
    Ok(sanitized)
}

/// Validate AWS profile name to prevent CLI injection
fn validate_aws_profile_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && 
    name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Validate Azure subscription ID format (UUID)
fn validate_azure_subscription_id(id: &str) -> bool {
    // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    if id.len() != 36 {
        return false;
    }
    id.chars().enumerate().all(|(i, c)| {
        match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit()
        }
    })
}

/// Sanitize template ID to prevent path traversal attacks
/// Only allows alphanumeric characters and hyphens
fn sanitize_template_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err("Template ID cannot be empty".to_string());
    }
    
    // Check for path traversal attempts
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err("Invalid template ID".to_string());
    }
    
    // Only allow alphanumeric and hyphens
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Template ID contains invalid characters".to_string());
    }
    
    Ok(id.to_string())
}

/// Clear cached templates and force refresh
#[tauri::command]
pub fn clear_templates_cache(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let templates_dir = app_data_dir.join("templates");
    let version_file = app_data_dir.join(".templates_version");
    
    // Remove templates directory
    if templates_dir.exists() {
        fs::remove_dir_all(&templates_dir)
            .map_err(|e| format!("Failed to remove templates: {}", e))?;
    }
    
    // Remove version file
    if version_file.exists() {
        fs::remove_file(&version_file)
            .map_err(|e| format!("Failed to remove version file: {}", e))?;
    }
    
    // Re-run setup to copy fresh templates
    setup_templates(&app)?;
    
    Ok("Templates cache cleared and refreshed".to_string())
}

#[tauri::command]
pub fn check_dependencies() -> HashMap<String, DependencyStatus> {
    let mut deps = HashMap::new();
    
    deps.insert("terraform".to_string(), dependencies::check_terraform());
    deps.insert("git".to_string(), dependencies::check_git());
    deps.insert("aws".to_string(), dependencies::check_aws_cli());
    deps.insert("azure".to_string(), dependencies::check_azure_cli());
    deps.insert("gcloud".to_string(), dependencies::check_gcloud_cli());
    deps.insert("databricks".to_string(), dependencies::check_databricks_cli());
    
    deps
}

#[tauri::command]
pub fn get_databricks_profiles(cloud: String) -> Vec<dependencies::DatabricksProfile> {
    dependencies::get_databricks_profiles_for_cloud(&cloud)
}

#[tauri::command]
pub async fn databricks_cli_login(cloud: String, account_id: String) -> Result<String, String> {
    let cli_path = dependencies::find_databricks_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("Databricks CLI"))?;
    
    // Determine the account host based on cloud
    let host = if cloud == "azure" {
        "https://accounts.azuredatabricks.net"
    } else {
        "https://accounts.cloud.databricks.com"
    };
    
    // Create a profile name based on the account ID (first 8 chars)
    let profile_name = format!("deployer-{}", &account_id[..8.min(account_id.len())]);
    
    // Clear the token cache to force re-authentication
    // The token cache is at ~/.databricks/token-cache.json
    if let Some(home) = dirs::home_dir() {
        let token_cache_path = home.join(".databricks").join("token-cache.json");
        if token_cache_path.exists() {
            // Read and parse the token cache
            if let Ok(content) = std::fs::read_to_string(&token_cache_path) {
                if let Ok(mut cache) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = cache.as_object_mut() {
                        // Remove entries that match this host/account
                        let keys_to_remove: Vec<String> = obj.keys()
                            .filter(|k| k.contains(&account_id) || k.contains(host))
                            .cloned()
                            .collect();
                        
                        for key in keys_to_remove {
                            obj.remove(&key);
                        }
                        
                        // Write back the modified cache
                        if let Ok(new_content) = serde_json::to_string_pretty(&cache) {
                            let _ = std::fs::write(&token_cache_path, new_content);
                        }
                    }
                }
            }
        }
    }
    
    // Run databricks auth login command with explicit profile name
    // Using spawn() with inherited stdio to allow interactive OAuth flow with browser
    let mut child = std::process::Command::new(&cli_path)
        .args([
            "auth", "login",
            "--host", host,
            "--account-id", &account_id,
            "--profile", &profile_name,
        ])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to run Databricks CLI: {}", e))?;
    
    // Wait for the command to complete
    let status = child.wait()
        .map_err(|e| format!("Failed to wait for Databricks CLI: {}", e))?;
    
    if status.success() {
        Ok(format!("Login successful! Profile '{}' created/updated.", profile_name))
    } else {
        // Check if profile was created anyway (user might have completed OAuth)
        let profiles = dependencies::get_databricks_profiles_for_cloud(&cloud);
        if profiles.iter().any(|p| p.name == profile_name) {
            Ok(format!("Profile '{}' is ready.", profile_name))
        } else {
            Err("Login failed or was cancelled. Please try again.".to_string())
        }
    }
}

#[tauri::command]
pub fn get_databricks_profile_credentials(profile_name: String) -> Result<HashMap<String, String>, String> {
    let config_path = dependencies::get_databricks_config_path()
        .ok_or_else(|| "Databricks config file not found".to_string())?;
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    let mut in_target_profile = false;
    let mut credentials: HashMap<String, String> = HashMap::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        if line.starts_with('[') && line.ends_with(']') {
            let section_name = &line[1..line.len()-1];
            in_target_profile = section_name == profile_name;
            continue;
        }
        
        if in_target_profile {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                credentials.insert(key, value);
            }
        }
    }
    
    if credentials.is_empty() {
        Err(format!("Profile '{}' not found or has no credentials", profile_name))
    } else {
        Ok(credentials)
    }
}

#[tauri::command]
pub async fn install_terraform() -> Result<String, String> {
    let url = dependencies::get_terraform_download_url();
    let install_dir = dependencies::get_terraform_install_path();
    
    // Download terraform
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download Terraform: {}", e))?;
    
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    // Create temp file for zip
    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("terraform.zip");
    
    fs::write(&zip_path, &bytes).map_err(|e| format!("Failed to write zip: {}", e))?;
    
    // Extract zip
    let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = install_dir.join(file.name());
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            
            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = outfile.metadata().map_err(|e| e.to_string())?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&outpath, perms).map_err(|e| e.to_string())?;
            }
        }
    }
    
    Ok(format!("Terraform installed to {}", install_dir.display()))
}

#[tauri::command]
pub async fn validate_databricks_credentials(
    account_id: String,
    client_id: String,
    client_secret: String,
    cloud: String,
) -> Result<String, String> {
    // Use correct host based on cloud provider
    let accounts_host = if cloud == "azure" {
        "accounts.azuredatabricks.net"
    } else {
        "accounts.cloud.databricks.com"
    };
    
    // Get OAuth token from Databricks
    let token_url = format!(
        "https://{}/oidc/accounts/{}/v1/token",
        accounts_host, account_id
    );

    let client = reqwest::Client::new();
    
    // Request OAuth token using client credentials flow
    let token_response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("scope", "all-apis"),
        ])
        .basic_auth(&client_id, Some(&client_secret))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Databricks: {}", e))?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let error_text = token_response.text().await.unwrap_or_default();
        return Err(format!(
            "Authentication failed ({}): Invalid credentials or account ID. {}",
            status, error_text
        ));
    }

    // Parse the token response
    let token_json: serde_json::Value = token_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let access_token = token_json["access_token"]
        .as_str()
        .ok_or("No access token in response")?;

    // Use SCIM API to list users - only account admins can do this
    let users_url = format!(
        "https://{}/api/2.0/accounts/{}/scim/v2/Users?count=1",
        accounts_host, account_id
    );

    let users_response = client
        .get(&users_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to verify account access: {}", e))?;

    if !users_response.status().is_success() {
        let status = users_response.status();
        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(
                "Service principal does not have account admin privileges. \
                Please grant 'Account admin' role in Databricks Account Console → \
                User Management → Service Principals → [Your SP] → Roles."
                .to_string()
            );
        }
        return Err(format!(
            "Cannot verify account access ({}). Check your Account ID and service principal permissions.",
            status
        ));
    }

    // Parse response to confirm we got user data
    let users_json: serde_json::Value = users_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Check if we got any results (totalResults field exists and API worked)
    if users_json.get("totalResults").is_some() || users_json.get("Resources").is_some() {
        return Ok("Credentials validated - Account Admin access confirmed".to_string());
    }

    Ok("Credentials validated successfully".to_string())
}

#[tauri::command]
pub fn get_templates(app: AppHandle) -> Result<Vec<Template>, String> {
    let templates_dir = get_templates_dir(&app)?;
    let mut templates = Vec::new();
    
    // AWS Simple
    if templates_dir.join("aws-simple").exists() {
        templates.push(Template {
            id: "aws-simple".to_string(),
            name: "AWS Standard BYOVPC".to_string(),
            cloud: "aws".to_string(),
            description: "Secure baseline deployment with customer-managed VPC".to_string(),
            features: vec![
                "Customer-managed VPC (BYOVPC)".to_string(),
                "Security groups for traffic control".to_string(),
                "Private and public subnets".to_string(),
                "IAM roles and policies".to_string(),
                "S3 bucket configuration".to_string(),
                "Unity Catalog integration".to_string(),
            ],
        });
    }
    
    // Azure Simple
    if templates_dir.join("azure-simple").exists() {
        templates.push(Template {
            id: "azure-simple".to_string(),
            name: "Azure Standard VNet".to_string(),
            cloud: "azure".to_string(),
            description: "Secure baseline deployment with VNet injection".to_string(),
            features: vec![
                "Private networking with VNet injection".to_string(),
                "Network security groups".to_string(),
                "NAT gateway for outbound access".to_string(),
                "Azure resource group isolation".to_string(),
                "Production-ready security".to_string(),
                "Unity Catalog integration".to_string(),
            ],
        });
    }
    
    Ok(templates)
}

#[tauri::command]
pub fn get_template_variables(app: AppHandle, template_id: String) -> Result<Vec<TerraformVariable>, String> {
    // Sanitize template ID to prevent path traversal
    let safe_template_id = sanitize_template_id(&template_id)?;
    
    let templates_dir = get_templates_dir(&app)?;
    let variables_path = templates_dir.join(&safe_template_id).join("variables.tf");
    
    if !variables_path.exists() {
        return Err(format!("Template not found: {}", safe_template_id));
    }
    
    let content = fs::read_to_string(&variables_path).map_err(|e| e.to_string())?;
    let variables = terraform::parse_variables_tf(&content);
    
    Ok(variables)
}

#[tauri::command]
pub fn save_configuration(
    app: AppHandle,
    template_id: String,
    deployment_name: String,
    values: HashMap<String, serde_json::Value>,
    credentials: Option<CloudCredentials>,
) -> Result<String, String> {
    // Sanitize inputs to prevent path traversal
    let safe_deployment_name = sanitize_deployment_name(&deployment_name)?;
    let safe_template_id = sanitize_template_id(&template_id)?;
    
    let templates_dir = get_templates_dir(&app)?;
    let template_dir = templates_dir.join(&safe_template_id);
    let template_variables_path = template_dir.join("variables.tf");
    
    if !template_variables_path.exists() {
        return Err("Template not found".to_string());
    }
    
    // Create deployment folder with sanitized name
    let deployments_dir = get_deployments_dir(&app)?;
    let deployment_dir = deployments_dir.join(&safe_deployment_name);
    
    // If deployment folder already exists, use it (allows re-running)
    // Otherwise create and copy template files
    if !deployment_dir.exists() {
        fs::create_dir_all(&deployment_dir).map_err(|e| e.to_string())?;
        
        // Copy all template files to deployment folder
        copy_dir_all(&template_dir, &deployment_dir)?;
    }
    
    let tfvars_path = deployment_dir.join("terraform.tfvars");
    let variables_path = deployment_dir.join("variables.tf");
    
    // Merge credentials into values for terraform variables that need them
    let mut merged_values = values.clone();
    if let Some(creds) = credentials {
        // Databricks account ID is needed as a terraform variable
        if let Some(account_id) = creds.databricks_account_id {
            if !account_id.is_empty() {
                merged_values.insert(
                    "databricks_account_id".to_string(),
                    serde_json::Value::String(account_id),
                );
            }
        }
        // Databricks auth type for Terraform provider
        // "profile" -> "databricks-cli", "credentials" -> "oauth-m2m"
        let auth_type = match creds.databricks_auth_type.as_deref() {
            Some("profile") => "databricks-cli",
            _ => "oauth-m2m",
        };
        merged_values.insert(
            "databricks_auth_type".to_string(),
            serde_json::Value::String(auth_type.to_string()),
        );
        
        // Azure tenant ID is needed as a terraform variable for Azure templates
        if let Some(tenant_id) = &creds.azure_tenant_id {
            if !tenant_id.is_empty() {
                merged_values.insert(
                    "tenant_id".to_string(),
                    serde_json::Value::String(tenant_id.clone()),
                );
            }
        }
        // Azure subscription ID is also needed as a terraform variable for Azure templates
        if let Some(sub_id) = &creds.azure_subscription_id {
            if !sub_id.is_empty() {
                merged_values.insert(
                    "subscription_id".to_string(),
                    serde_json::Value::String(sub_id.clone()),
                );
                merged_values.insert(
                    "azure_subscription_id".to_string(),
                    serde_json::Value::String(sub_id.clone()),
                );
            }
        }
    }
    
    let variables_content = fs::read_to_string(&variables_path).map_err(|e| e.to_string())?;
    let variables = terraform::parse_variables_tf(&variables_content);
    
    let tfvars_content = terraform::generate_tfvars(&merged_values, &variables);
    fs::write(&tfvars_path, tfvars_content).map_err(|e| e.to_string())?;
    
    // Return the deployment path
    Ok(deployment_dir.to_string_lossy().to_string())
}

/// Helper to set env var from optional credential value
fn set_env_if_present(env_vars: &mut HashMap<String, String>, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        if !v.is_empty() {
            env_vars.insert(key.to_string(), v.clone());
        }
    }
}

/// Build environment variables HashMap from CloudCredentials
fn build_env_vars(credentials: &CloudCredentials) -> HashMap<String, String> {
    let mut env_vars = HashMap::new();
    
    // AWS credentials - use profile OR access keys
    if let Some(profile) = &credentials.aws_profile {
        if !profile.is_empty() {
            env_vars.insert("AWS_PROFILE".to_string(), profile.clone());
        }
    }
    // Access keys (used if no profile, or as override)
    set_env_if_present(&mut env_vars, "AWS_ACCESS_KEY_ID", &credentials.aws_access_key_id);
    set_env_if_present(&mut env_vars, "AWS_SECRET_ACCESS_KEY", &credentials.aws_secret_access_key);
    set_env_if_present(&mut env_vars, "AWS_SESSION_TOKEN", &credentials.aws_session_token);
    set_env_if_present(&mut env_vars, "AWS_DEFAULT_REGION", &credentials.aws_region);
    
    // Azure credentials
    set_env_if_present(&mut env_vars, "ARM_TENANT_ID", &credentials.azure_tenant_id);
    set_env_if_present(&mut env_vars, "ARM_SUBSCRIPTION_ID", &credentials.azure_subscription_id);
    set_env_if_present(&mut env_vars, "ARM_CLIENT_ID", &credentials.azure_client_id);
    set_env_if_present(&mut env_vars, "ARM_CLIENT_SECRET", &credentials.azure_client_secret);
    
    // Databricks credentials
    set_env_if_present(&mut env_vars, "DATABRICKS_ACCOUNT_ID", &credentials.databricks_account_id);
    
    // Handle Databricks auth based on auth_type
    let databricks_auth_type = credentials.databricks_auth_type.as_deref().unwrap_or("credentials");
    
    if databricks_auth_type == "profile" {
        // Use profile-based authentication
        set_env_if_present(&mut env_vars, "DATABRICKS_CONFIG_PROFILE", &credentials.databricks_profile);
    } else {
        // Use client credentials (service principal)
        set_env_if_present(&mut env_vars, "DATABRICKS_CLIENT_ID", &credentials.databricks_client_id);
        set_env_if_present(&mut env_vars, "DATABRICKS_CLIENT_SECRET", &credentials.databricks_client_secret);
    }
    
    env_vars
}

#[tauri::command]
pub async fn run_terraform_command(
    app: AppHandle,
    deployment_name: String,
    command: String,
    credentials: CloudCredentials,
) -> Result<(), String> {
    // Sanitize deployment name to prevent path traversal
    let safe_deployment_name = sanitize_deployment_name(&deployment_name)?;
    
    // Check if already running
    {
        let status = DEPLOYMENT_STATUS.lock().map_err(|e| e.to_string())?;
        if status.running {
            return Err("A deployment is already running".to_string());
        }
    }
    
    let deployments_dir = get_deployments_dir(&app)?;
    let deployment_dir = deployments_dir.join(&safe_deployment_name);
    
    if !deployment_dir.exists() {
        return Err("Deployment not found. Please save configuration first.".to_string());
    }
    
    // Build environment variables from credentials
    let env_vars = build_env_vars(&credentials);
    
    // Reset status
    {
        let mut status = DEPLOYMENT_STATUS.lock().map_err(|e| e.to_string())?;
        status.running = true;
        status.command = Some(format!("terraform {}", command));
        status.output = String::new();
        status.success = None;
        status.can_rollback = terraform::check_state_exists(&deployment_dir);
    }
    
    // Run terraform in background thread
    let status_clone = DEPLOYMENT_STATUS.clone();
    let process_clone = CURRENT_PROCESS.clone();
    let cmd = command.clone();
    let dir = deployment_dir.clone();
    
    std::thread::spawn(move || {
        match terraform::run_terraform(&cmd, &dir, env_vars) {
            Ok(mut child) => {
                // Store process ID
                if let Ok(mut proc) = process_clone.lock() {
                    *proc = Some(child.id());
                }
                
                // Read stdout and stderr concurrently to avoid deadlock
                // when one stream's buffer fills while we're reading the other
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                
                let status_for_stdout = status_clone.clone();
                let status_for_stderr = status_clone.clone();
                
                // Spawn thread for stdout
                let stdout_handle = stdout.map(|out| {
                    std::thread::spawn(move || {
                        let reader = BufReader::new(out);
                        for line in reader.lines().flatten() {
                            if let Ok(mut s) = status_for_stdout.lock() {
                                s.output.push_str(&line);
                                s.output.push('\n');
                            }
                        }
                    })
                });
                
                // Spawn thread for stderr
                let stderr_handle = stderr.map(|err| {
                    std::thread::spawn(move || {
                        let reader = BufReader::new(err);
                        for line in reader.lines().flatten() {
                            if let Ok(mut s) = status_for_stderr.lock() {
                                s.output.push_str(&line);
                                s.output.push('\n');
                            }
                        }
                    })
                });
                
                // Wait for both reader threads to complete
                if let Some(handle) = stdout_handle {
                    let _ = handle.join();
                }
                if let Some(handle) = stderr_handle {
                    let _ = handle.join();
                }
                
                // Wait for process to complete
                match child.wait() {
                    Ok(exit_status) => {
                        if let Ok(mut s) = status_clone.lock() {
                            s.running = false;
                            s.success = Some(exit_status.success());
                            s.can_rollback = terraform::check_state_exists(&dir);
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = status_clone.lock() {
                            s.running = false;
                            s.success = Some(false);
                            s.output.push_str(&format!("\nError: {}", e));
                        }
                    }
                }
                
                // Clear process ID
                if let Ok(mut proc) = process_clone.lock() {
                    *proc = None;
                }
            }
            Err(e) => {
                if let Ok(mut s) = status_clone.lock() {
                    s.running = false;
                    s.success = Some(false);
                    s.output = format!("Failed to start terraform: {}", e);
                }
            }
        }
    });
    
    Ok(())
}

#[tauri::command]
pub fn get_deployment_status() -> Result<DeploymentStatus, String> {
    let status = DEPLOYMENT_STATUS.lock().map_err(|e| e.to_string())?;
    Ok(status.clone())
}

#[tauri::command]
pub fn cancel_deployment() -> Result<(), String> {
    let proc_id = {
        let proc = CURRENT_PROCESS.lock().map_err(|e| e.to_string())?;
        *proc
    };
    
    if let Some(pid) = proc_id {
        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;
        }
        
        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;
        }
        
        // Update status
        if let Ok(mut status) = DEPLOYMENT_STATUS.lock() {
            status.running = false;
            status.success = Some(false);
            status.output.push_str("\n\nDeployment cancelled by user.");
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn rollback_deployment(
    app: AppHandle,
    deployment_name: String,
    credentials: CloudCredentials,
) -> Result<(), String> {
    // Rollback is essentially running terraform destroy
    run_terraform_command(app, deployment_name, "destroy".to_string(), credentials).await
}

#[tauri::command]
pub fn get_cloud_credentials(cloud: String) -> Result<CloudCredentials, String> {
    let mut creds = CloudCredentials {
        aws_profile: None,
        aws_access_key_id: None,
        aws_secret_access_key: None,
        aws_session_token: None,
        aws_region: None,
        azure_tenant_id: None,
        azure_subscription_id: None,
        azure_client_id: None,
        azure_client_secret: None,
        databricks_account_id: None,
        databricks_client_id: None,
        databricks_client_secret: None,
        databricks_profile: None,
        databricks_auth_type: None,
    };
    
    match cloud.as_str() {
        "aws" => {
            // Try to get from environment
            creds.aws_access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
            creds.aws_secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
            creds.aws_session_token = std::env::var("AWS_SESSION_TOKEN").ok();
            creds.aws_region = std::env::var("AWS_DEFAULT_REGION").ok()
                .or_else(|| std::env::var("AWS_REGION").ok());
        }
        "azure" => {
            creds.azure_tenant_id = std::env::var("ARM_TENANT_ID").ok();
            creds.azure_subscription_id = std::env::var("ARM_SUBSCRIPTION_ID").ok();
            creds.azure_client_id = std::env::var("ARM_CLIENT_ID").ok();
            creds.azure_client_secret = std::env::var("ARM_CLIENT_SECRET").ok();
        }
        _ => {}
    }
    
    // Databricks credentials - try environment first, then CLI config
    creds.databricks_account_id = std::env::var("DATABRICKS_ACCOUNT_ID").ok();
    creds.databricks_client_id = std::env::var("DATABRICKS_CLIENT_ID").ok();
    creds.databricks_client_secret = std::env::var("DATABRICKS_CLIENT_SECRET").ok();
    
    // If Databricks credentials not in env, try to read from CLI config (default profile)
    if creds.databricks_client_id.is_none() || creds.databricks_client_secret.is_none() {
        if let Some(cli_creds) = read_databricks_cli_config() {
            if creds.databricks_client_id.is_none() {
                creds.databricks_client_id = cli_creds.0;
            }
            if creds.databricks_client_secret.is_none() {
                creds.databricks_client_secret = cli_creds.1;
            }
            if creds.databricks_account_id.is_none() {
                creds.databricks_account_id = cli_creds.2;
            }
        }
    }
    
    Ok(creds)
}

/// Read Databricks CLI config from ~/.databrickscfg (default profile)
/// Returns (client_id, client_secret, account_id)
fn read_databricks_cli_config() -> Option<(Option<String>, Option<String>, Option<String>)> {
    let home = dirs::home_dir()?;
    let config_path = home.join(".databrickscfg");
    
    if !config_path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&config_path).ok()?;
    let mut client_id = None;
    let mut client_secret = None;
    let mut account_id = None;
    let mut in_default_section = false;
    
    for line in content.lines() {
        let line = line.trim();
        
        // Check for section headers
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len()-1];
            in_default_section = section.eq_ignore_ascii_case("default");
            continue;
        }
        
        // Parse key-value pairs in default section
        if in_default_section {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                
                match key.as_str() {
                    "client_id" => client_id = Some(value),
                    "client_secret" => client_secret = Some(value),
                    "account_id" => account_id = Some(value),
                    _ => {}
                }
            }
        }
    }
    
    // Only return if we found at least one credential
    if client_id.is_some() || client_secret.is_some() || account_id.is_some() {
        Some((client_id, client_secret, account_id))
    } else {
        None
    }
}

/// List AWS profiles from ~/.aws/config and ~/.aws/credentials
#[tauri::command]
pub fn get_aws_profiles() -> Vec<AwsProfile> {
    let mut profiles = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    if let Some(home) = dirs::home_dir() {
        // Parse ~/.aws/config
        let config_path = home.join(".aws").join("config");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with('[') && line.ends_with(']') {
                        let section = &line[1..line.len()-1];
                        // Config file uses "profile name" or just "default"
                        let name = if section.starts_with("profile ") {
                            section.strip_prefix("profile ").unwrap().to_string()
                        } else {
                            section.to_string()
                        };
                        if !seen.contains(&name) {
                            // Check if it's an SSO profile
                            let is_sso = content.contains(&format!("[profile {}]", name)) 
                                && content.contains("sso_start_url")
                                || (name == "default" && content.contains("sso_start_url"));
                            profiles.push(AwsProfile { name: name.clone(), is_sso });
                            seen.insert(name);
                        }
                    }
                }
            }
        }
        
        // Parse ~/.aws/credentials for additional profiles
        let creds_path = home.join(".aws").join("credentials");
        if creds_path.exists() {
            if let Ok(content) = fs::read_to_string(&creds_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with('[') && line.ends_with(']') {
                        let name = line[1..line.len()-1].to_string();
                        if !seen.contains(&name) {
                            profiles.push(AwsProfile { name: name.clone(), is_sso: false });
                            seen.insert(name);
                        }
                    }
                }
            }
        }
    }
    
    // Ensure "default" is first if it exists
    profiles.sort_by(|a, b| {
        if a.name == "default" { std::cmp::Ordering::Less }
        else if b.name == "default" { std::cmp::Ordering::Greater }
        else { a.name.cmp(&b.name) }
    });
    
    profiles
}

/// Get AWS identity for a profile using `aws sts get-caller-identity`
#[tauri::command]
pub fn get_aws_identity(profile: String) -> Result<AwsIdentity, String> {
    use std::process::Command;
    
    // Validate profile name to prevent CLI injection
    if !profile.is_empty() && !validate_aws_profile_name(&profile) {
        return Err("Invalid AWS profile name".to_string());
    }
    
    let aws_path = dependencies::find_aws_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("AWS CLI"))?;
    
    let mut cmd = Command::new(&aws_path);
    cmd.args(["sts", "get-caller-identity", "--output", "json"]);
    
    if !profile.is_empty() && profile != "default" {
        cmd.args(["--profile", &profile]);
    }
    
    let output = cmd.output().map_err(|e| format!("Failed to run AWS CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("expired") || stderr.contains("Token") {
            return Err(crate::errors::auth_expired("AWS"));
        }
        return Err(format!("Not authenticated: {}", stderr.trim()));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    Ok(AwsIdentity {
        account: json["Account"].as_str().unwrap_or("").to_string(),
        arn: json["Arn"].as_str().unwrap_or("").to_string(),
        user_id: json["UserId"].as_str().unwrap_or("").to_string(),
    })
}

/// Trigger AWS SSO login for a profile
#[tauri::command]
pub async fn aws_sso_login(profile: String) -> Result<String, String> {
    use std::process::Command;
    
    // Validate profile name to prevent CLI injection
    if !profile.is_empty() && !validate_aws_profile_name(&profile) {
        return Err("Invalid AWS profile name".to_string());
    }
    
    let aws_path = dependencies::find_aws_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("AWS CLI"))?;
    
    let mut cmd = Command::new(&aws_path);
    cmd.args(["sso", "login"]);
    
    if !profile.is_empty() && profile != "default" {
        cmd.args(["--profile", &profile]);
    }
    
    let output = cmd.output().map_err(|e| format!("Failed to run AWS CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("SSO login failed: {}", stderr.trim()));
    }
    
    Ok("SSO login initiated. Complete authentication in your browser.".to_string())
}

// ========== Azure CLI Commands ==========

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureSubscription {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub tenant_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureAccount {
    pub user: String,
    pub tenant_id: String,
    pub subscription_id: String,
    pub subscription_name: String,
}

/// Get Azure CLI login status using `az account show`
#[tauri::command]
pub fn get_azure_account() -> Result<AzureAccount, String> {
    use std::process::Command;
    
    let az_path = dependencies::find_azure_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("Azure CLI"))?;
    
    let output = Command::new(&az_path)
        .args(["account", "show", "--output", "json"])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("az login") || stderr.contains("not logged in") {
            return Err(crate::errors::not_logged_in("Azure"));
        }
        return Err(format!("Azure CLI error: {}", stderr.trim()));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let user = json["user"]["name"].as_str().unwrap_or("").to_string();
    
    Ok(AzureAccount {
        user,
        tenant_id: json["tenantId"].as_str().unwrap_or("").to_string(),
        subscription_id: json["id"].as_str().unwrap_or("").to_string(),
        subscription_name: json["name"].as_str().unwrap_or("").to_string(),
    })
}

/// Get list of Azure subscriptions
#[tauri::command]
pub fn get_azure_subscriptions() -> Result<Vec<AzureSubscription>, String> {
    use std::process::Command;
    
    let az_path = dependencies::find_azure_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("Azure CLI"))?;
    
    let output = Command::new(&az_path)
        .args(["account", "list", "--output", "json"])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Azure CLI error: {}", stderr.trim()));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let subscriptions: Vec<AzureSubscription> = json.iter().map(|sub| {
        AzureSubscription {
            id: sub["id"].as_str().unwrap_or("").to_string(),
            name: sub["name"].as_str().unwrap_or("").to_string(),
            is_default: sub["isDefault"].as_bool().unwrap_or(false),
            tenant_id: sub["tenantId"].as_str().unwrap_or("").to_string(),
        }
    }).collect();
    
    Ok(subscriptions)
}

/// Trigger Azure CLI login
#[tauri::command]
pub async fn azure_login() -> Result<String, String> {
    use std::process::Command;
    
    let az_path = dependencies::find_azure_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("Azure CLI"))?;
    
    let output = Command::new(&az_path)
        .args(["login"])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Azure login failed: {}", stderr.trim()));
    }
    
    Ok("Azure login initiated. Complete authentication in your browser.".to_string())
}

/// Set Azure subscription
#[tauri::command]
pub fn set_azure_subscription(subscription_id: String) -> Result<(), String> {
    use std::process::Command;
    
    // Validate subscription ID format to prevent CLI injection
    if !validate_azure_subscription_id(&subscription_id) {
        return Err("Invalid Azure subscription ID format".to_string());
    }
    
    let az_path = dependencies::find_azure_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("Azure CLI"))?;
    
    let output = Command::new(&az_path)
        .args(["account", "set", "--subscription", &subscription_id])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to set subscription: {}", stderr.trim()));
    }
    
    Ok(())
}

/// Resource group info
#[derive(Debug, Clone, serde::Serialize)]
pub struct AzureResourceGroup {
    pub name: String,
    pub location: String,
}

/// List Azure resource groups using `az group list`
#[tauri::command]
pub fn get_azure_resource_groups() -> Result<Vec<AzureResourceGroup>, String> {
    use std::process::Command;
    
    let az_path = dependencies::find_azure_cli_path()
        .ok_or_else(|| crate::errors::cli_not_found("Azure CLI"))?;
    
    let output = Command::new(&az_path)
        .args(["group", "list", "--output", "json"])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to list resource groups: {}", stderr.trim()));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse resource groups: {}", e))?;
    
    let groups: Vec<AzureResourceGroup> = json.as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|rg| AzureResourceGroup {
            name: rg["name"].as_str().unwrap_or("").to_string(),
            location: rg["location"].as_str().unwrap_or("").to_string(),
        })
        .collect();
    
    Ok(groups)
}

/// Get the path to the deployments parent directory
#[tauri::command]
pub fn get_deployments_folder(app: AppHandle) -> Result<String, String> {
    let deployments_dir = get_deployments_dir(&app)?;
    Ok(deployments_dir.to_string_lossy().to_string())
}

/// Open a folder in the system file manager
#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    use std::process::Command;
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

// Unity Catalog permission check types
#[derive(Debug, Serialize, Deserialize)]
pub struct MetastoreInfo {
    pub exists: bool,
    pub metastore_id: Option<String>,
    pub metastore_name: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UCPermissionCheck {
    pub metastore: MetastoreInfo,
    pub has_create_catalog: bool,
    pub has_create_external_location: bool,
    pub has_create_storage_credential: bool,
    pub can_create_catalog: bool,
    pub message: String,
}

/// Generate a message about metastore ownership for permission guidance
fn get_metastore_owner_info(metastore_owner: &str) -> String {
    let owner_is_group = !metastore_owner.contains('@');
    
    if owner_is_group {
        format!(
            "Metastore owned by group '{}'. You need the required permissions granted on this metastore.",
            metastore_owner
        )
    } else {
        format!(
            "Metastore owned by '{}'. You need the required permissions granted on this metastore.",
            metastore_owner
        )
    }
}

/// Check Unity Catalog permissions
#[tauri::command]
pub async fn check_uc_permissions(
    credentials: CloudCredentials,
    region: String,
) -> Result<UCPermissionCheck, String> {
    // Determine if we're using Azure or AWS
    let is_azure = credentials.azure_tenant_id.is_some();
    
    // Get account ID
    let account_id = credentials.databricks_account_id
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or("Databricks account ID is required")?;
    
    // Check if using service principal or profile
    let auth_type = credentials.databricks_auth_type.as_deref().unwrap_or("credentials");
    
    if auth_type == "profile" {
        // For profile auth, use Databricks CLI to list metastores
        let profile_name = credentials.databricks_profile.as_deref().unwrap_or("DEFAULT");
        
        // Try to list metastores using Databricks CLI
        let cli_path = dependencies::find_databricks_cli_path();
        
        if let Some(cli) = cli_path {
            let output = std::process::Command::new(&cli)
                .args(["account", "metastores", "list", "--output", "json", "-p", profile_name])
                .output();
            
            if let Ok(out) = output {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    
                    if let Ok(metastores_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let region_normalized = region.to_lowercase().replace(" ", "").replace("-", "");
                        
                        if let Some(arr) = metastores_json.as_array() {
                            // Find matching metastore by region
                            let matching_metastore = arr.iter().find(|m| {
                                let metastore_region = m["region"].as_str().unwrap_or("");
                                let metastore_region_normalized = metastore_region.to_lowercase().replace(" ", "").replace("-", "");
                                metastore_region_normalized == region_normalized
                            });
                            
                            if let Some(metastore) = matching_metastore {
                                let metastore_id = metastore["metastore_id"].as_str().unwrap_or("");
                                let metastore_name = metastore["name"].as_str().unwrap_or("");
                                let metastore_owner = metastore["owner"].as_str().unwrap_or("");
                                
                                let message = get_metastore_owner_info(metastore_owner);
                                
                                return Ok(UCPermissionCheck {
                                    metastore: MetastoreInfo {
                                        exists: true,
                                        metastore_id: Some(metastore_id.to_string()),
                                        metastore_name: Some(metastore_name.to_string()),
                                        region: Some(region),
                                    },
                                    has_create_catalog: false,
                                    has_create_external_location: false,
                                    has_create_storage_credential: false,
                                    can_create_catalog: false,
                                    message,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: no metastore found or couldn't check
        return Ok(UCPermissionCheck {
            metastore: MetastoreInfo {
                exists: false,
                metastore_id: None,
                metastore_name: None,
                region: Some(region),
            },
            has_create_catalog: true,
            has_create_external_location: true,
            has_create_storage_credential: true,
            can_create_catalog: true,
            message: "No metastore found in region. A new one will be created.".to_string(),
        });
    }
    
    // Get service principal credentials
    let client_id = credentials.databricks_client_id
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or("Client ID is required for permission check")?;
    
    let client_secret = credentials.databricks_client_secret
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or("Client Secret is required for permission check")?;
    
    // Determine accounts host
    let accounts_host = if is_azure {
        "accounts.azuredatabricks.net"
    } else {
        "accounts.cloud.databricks.com"
    };
    
    // Get OAuth token
    let token_url = format!(
        "https://{}/oidc/accounts/{}/v1/token",
        accounts_host, account_id
    );

    let client = reqwest::Client::new();
    
    let token_response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("scope", "all-apis"),
        ])
        .basic_auth(client_id, Some(client_secret))
        .send()
        .await
        .map_err(|e| format!("Failed to get OAuth token: {}", e))?;

    if !token_response.status().is_success() {
        return Err("Failed to authenticate with Databricks".to_string());
    }

    let token_json: serde_json::Value = token_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token: {}", e))?;

    let access_token = token_json["access_token"]
        .as_str()
        .ok_or("No access token in response")?;

    // List metastores
    let metastores_url = format!(
        "https://{}/api/2.1/unity-catalog/metastores",
        accounts_host
    );

    let metastores_response = client
        .get(&metastores_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to list metastores: {}", e))?;

    if !metastores_response.status().is_success() {
        // Can't list metastores, assume none exists
        return Ok(UCPermissionCheck {
            metastore: MetastoreInfo {
                exists: false,
                metastore_id: None,
                metastore_name: None,
                region: Some(region.clone()),
            },
            has_create_catalog: true,
            has_create_external_location: true,
            has_create_storage_credential: true,
            can_create_catalog: true,
            message: "No metastore found in region. A new one will be created.".to_string(),
        });
    }

    let metastores_json: serde_json::Value = metastores_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse metastores: {}", e))?;

    // Find metastore matching region
    let metastores = metastores_json["metastores"].as_array();
    let region_normalized = region.to_lowercase().replace(" ", "").replace("-", "");
    
    let matching_metastore = metastores.and_then(|arr| {
        arr.iter().find(|m| {
            let metastore_region = m["region"].as_str().unwrap_or("");
            let metastore_region_normalized = metastore_region.to_lowercase().replace(" ", "").replace("-", "");
            
            // Match on exact normalized region
            metastore_region_normalized == region_normalized
        })
    });

    if let Some(metastore) = matching_metastore {
        let metastore_id = metastore["metastore_id"].as_str().unwrap_or("");
        let metastore_name = metastore["name"].as_str().unwrap_or("");
        
        // Check permissions on this metastore
        let permissions_url = format!(
            "https://{}/api/2.1/unity-catalog/permissions/metastore/{}",
            accounts_host, metastore_id
        );

        let permissions_response = client
            .get(&permissions_url)
            .bearer_auth(access_token)
            .send()
            .await;

        let (has_create_catalog, has_create_external_location, has_create_storage_credential) = 
            if let Ok(resp) = permissions_response {
                if resp.status().is_success() {
                    if let Ok(perm_json) = resp.json::<serde_json::Value>().await {
                        let assignments = perm_json["privilege_assignments"].as_array();
                        let mut create_catalog = false;
                        let mut create_ext_loc = false;
                        let mut create_storage_cred = false;
                        
                        if let Some(arr) = assignments {
                            for assignment in arr {
                                if let Some(privileges) = assignment["privileges"].as_array() {
                                    for priv_val in privileges {
                                        let priv_str = priv_val.as_str().unwrap_or("");
                                        match priv_str {
                                            "CREATE_CATALOG" => create_catalog = true,
                                            "CREATE_EXTERNAL_LOCATION" => create_ext_loc = true,
                                            "CREATE_STORAGE_CREDENTIAL" => create_storage_cred = true,
                                            "ALL_PRIVILEGES" => {
                                                // Metastore admin has all privileges
                                                create_catalog = true;
                                                create_ext_loc = true;
                                                create_storage_cred = true;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        (create_catalog, create_ext_loc, create_storage_cred)
                    } else {
                        (false, false, false)
                    }
                } else {
                    // If we can't check permissions, assume we have them
                    // (the deployment will fail if we don't)
                    (true, true, true)
                }
            } else {
                (true, true, true)
            };

        let can_create = has_create_catalog && has_create_external_location && has_create_storage_credential;
        let message = if can_create {
            "You have the required permissions to create catalogs.".to_string()
        } else {
            let mut missing = Vec::new();
            if !has_create_catalog { missing.push("CREATE_CATALOG"); }
            if !has_create_storage_credential { missing.push("CREATE_STORAGE_CREDENTIAL"); }
            if !has_create_external_location { missing.push("CREATE_EXTERNAL_LOCATION"); }
            format!("Missing permissions: {}. Contact your Metastore Admin.", missing.join(", "))
        };

        Ok(UCPermissionCheck {
            metastore: MetastoreInfo {
                exists: true,
                metastore_id: Some(metastore_id.to_string()),
                metastore_name: Some(metastore_name.to_string()),
                region: Some(region),
            },
            has_create_catalog,
            has_create_external_location,
            has_create_storage_credential,
            can_create_catalog: can_create,
            message,
        })
    } else {
        // No metastore in region - will be created
        Ok(UCPermissionCheck {
            metastore: MetastoreInfo {
                exists: false,
                metastore_id: None,
                metastore_name: None,
                region: Some(region),
            },
            has_create_catalog: true,
            has_create_external_location: true,
            has_create_storage_credential: true,
            can_create_catalog: true,
            message: "No metastore found in region. A new one will be created.".to_string(),
        })
    }
}

