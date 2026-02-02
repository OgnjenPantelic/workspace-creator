use crate::dependencies::{self, DependencyStatus};
use crate::terraform::{self, DeploymentStatus, TerraformVariable, CURRENT_PROCESS, DEPLOYMENT_STATUS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
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
const TEMPLATES_VERSION: &str = "2.2.0";

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
    deps.insert("aws".to_string(), dependencies::check_aws_cli());
    deps.insert("azure".to_string(), dependencies::check_azure_cli());
    deps.insert("gcloud".to_string(), dependencies::check_gcloud_cli());
    
    deps
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
    set_env_if_present(&mut env_vars, "DATABRICKS_CLIENT_ID", &credentials.databricks_client_id);
    set_env_if_present(&mut env_vars, "DATABRICKS_CLIENT_SECRET", &credentials.databricks_client_secret);
    
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
        .ok_or_else(|| "AWS CLI not found".to_string())?;
    
    let mut cmd = Command::new(&aws_path);
    cmd.args(["sts", "get-caller-identity", "--output", "json"]);
    
    if !profile.is_empty() && profile != "default" {
        cmd.args(["--profile", &profile]);
    }
    
    let output = cmd.output().map_err(|e| format!("Failed to run AWS CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("expired") || stderr.contains("Token") {
            return Err("Session expired. Please login again.".to_string());
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
        .ok_or_else(|| "AWS CLI not found".to_string())?;
    
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
        .ok_or_else(|| "Azure CLI not found".to_string())?;
    
    let output = Command::new(&az_path)
        .args(["account", "show", "--output", "json"])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("az login") || stderr.contains("not logged in") {
            return Err("Not logged in. Please run 'az login' first.".to_string());
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
        .ok_or_else(|| "Azure CLI not found".to_string())?;
    
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
        .ok_or_else(|| "Azure CLI not found".to_string())?;
    
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
        .ok_or_else(|| "Azure CLI not found".to_string())?;
    
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
        .ok_or_else(|| "Azure CLI not found".to_string())?;
    
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

