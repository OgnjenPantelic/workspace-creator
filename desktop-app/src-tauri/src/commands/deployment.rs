//! Terraform deployment, configuration, and lifecycle management commands.

use super::{
    copy_dir_all, get_deployments_dir, get_templates_dir, sanitize_deployment_name,
    sanitize_template_id, CloudCredentials,
};
use crate::dependencies::{self, DependencyStatus};
use crate::terraform::{self, DeploymentStatus, CURRENT_PROCESS, DEPLOYMENT_STATUS};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use tauri::AppHandle;

// ─── Helpers (deployment-local) ─────────────────────────────────────────────

/// Set an environment variable from an optional credential value.
fn set_env_if_present(env_vars: &mut HashMap<String, String>, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        if !v.is_empty() {
            env_vars.insert(key.to_string(), v.clone());
        }
    }
}

/// Build the environment variables map that Terraform needs from credentials.
fn build_env_vars(credentials: &CloudCredentials) -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    // AWS credentials
    if let Some(profile) = &credentials.aws_profile {
        if !profile.is_empty() {
            env_vars.insert("AWS_PROFILE".to_string(), profile.clone());
        }
    }
    set_env_if_present(&mut env_vars, "AWS_ACCESS_KEY_ID", &credentials.aws_access_key_id);
    set_env_if_present(&mut env_vars, "AWS_SECRET_ACCESS_KEY", &credentials.aws_secret_access_key);
    set_env_if_present(&mut env_vars, "AWS_SESSION_TOKEN", &credentials.aws_session_token);
    set_env_if_present(&mut env_vars, "AWS_DEFAULT_REGION", &credentials.aws_region);

    // Azure credentials
    set_env_if_present(&mut env_vars, "ARM_TENANT_ID", &credentials.azure_tenant_id);
    set_env_if_present(&mut env_vars, "ARM_SUBSCRIPTION_ID", &credentials.azure_subscription_id);
    set_env_if_present(&mut env_vars, "ARM_CLIENT_ID", &credentials.azure_client_id);
    set_env_if_present(&mut env_vars, "ARM_CLIENT_SECRET", &credentials.azure_client_secret);

    // GCP credentials
    let is_gcp = credentials.cloud.as_deref() == Some("gcp");

    if let Some(project_id) = &credentials.gcp_project_id {
        if !project_id.is_empty() {
            env_vars.insert("GOOGLE_PROJECT".to_string(), project_id.clone());
            env_vars.insert("GCLOUD_PROJECT".to_string(), project_id.clone());
            env_vars.insert("CLOUDSDK_CORE_PROJECT".to_string(), project_id.clone());
        }
    }

    let has_credentials_json = credentials
        .gcp_credentials_json
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    if has_credentials_json {
        set_env_if_present(&mut env_vars, "GOOGLE_CREDENTIALS", &credentials.gcp_credentials_json);
    } else {
        set_env_if_present(
            &mut env_vars,
            "GOOGLE_OAUTH_ACCESS_TOKEN",
            &credentials.gcp_oauth_token,
        );
    }

    // Databricks credentials
    set_env_if_present(
        &mut env_vars,
        "DATABRICKS_ACCOUNT_ID",
        &credentials.databricks_account_id,
    );

    let databricks_auth_type = credentials
        .databricks_auth_type
        .as_deref()
        .unwrap_or("credentials");

    let profile_has_sp_creds = databricks_auth_type == "profile"
        && credentials
            .databricks_client_id
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        && credentials
            .databricks_client_secret
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);

    let is_azure = credentials.cloud.as_deref() == Some("azure");

    if is_gcp {
        env_vars.insert("DATABRICKS_CONFIG_FILE".to_string(), "/dev/null".to_string());
    } else if !is_azure {
        if databricks_auth_type == "profile" && !profile_has_sp_creds {
            set_env_if_present(
                &mut env_vars,
                "DATABRICKS_CONFIG_PROFILE",
                &credentials.databricks_profile,
            );
        } else {
            set_env_if_present(
                &mut env_vars,
                "DATABRICKS_CLIENT_ID",
                &credentials.databricks_client_id,
            );
            set_env_if_present(
                &mut env_vars,
                "DATABRICKS_CLIENT_SECRET",
                &credentials.databricks_client_secret,
            );
        }
    }

    env_vars
}

/// Read Databricks CLI config from `~/.databrickscfg` (default profile).
/// Returns `(client_id, client_secret, account_id)`.
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

        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            in_default_section = section.eq_ignore_ascii_case("default");
            continue;
        }

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

    if client_id.is_some() || client_secret.is_some() || account_id.is_some() {
        Some((client_id, client_secret, account_id))
    } else {
        None
    }
}

// ─── Tauri Commands ─────────────────────────────────────────────────────────

/// Check which CLI dependencies are installed.
#[tauri::command]
pub fn check_dependencies() -> HashMap<String, DependencyStatus> {
    let mut deps = HashMap::new();

    deps.insert("terraform".to_string(), dependencies::check_terraform());
    deps.insert("git".to_string(), dependencies::check_git());
    deps.insert("aws".to_string(), dependencies::check_aws_cli());
    deps.insert("azure".to_string(), dependencies::check_azure_cli());
    deps.insert("gcloud".to_string(), dependencies::check_gcloud_cli());
    deps.insert(
        "databricks".to_string(),
        dependencies::check_databricks_cli(),
    );

    deps
}

/// Download and install Terraform.
#[tauri::command]
pub async fn install_terraform() -> Result<String, String> {
    let url = dependencies::get_terraform_download_url();
    let install_dir = dependencies::get_terraform_install_path();

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download Terraform: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("terraform.zip");

    fs::write(&zip_path, &bytes).map_err(|e| format!("Failed to write zip: {}", e))?;

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

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = outfile.metadata().map_err(|e| e.to_string())?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&outpath, perms).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(format!(
        "Terraform installed to {}",
        install_dir.display()
    ))
}

/// Save deployment configuration (copy template + generate `terraform.tfvars`).
#[tauri::command]
pub fn save_configuration(
    app: AppHandle,
    template_id: String,
    deployment_name: String,
    values: HashMap<String, serde_json::Value>,
    credentials: Option<CloudCredentials>,
) -> Result<String, String> {
    let safe_deployment_name = sanitize_deployment_name(&deployment_name)?;
    let safe_template_id = sanitize_template_id(&template_id)?;

    let templates_dir = get_templates_dir(&app)?;
    let template_dir = templates_dir.join(&safe_template_id);
    let template_variables_path = template_dir.join("variables.tf");

    if !template_variables_path.exists() {
        return Err("Template not found".to_string());
    }

    let deployments_dir = get_deployments_dir(&app)?;
    let deployment_dir = deployments_dir.join(&safe_deployment_name);

    if !deployment_dir.exists() {
        fs::create_dir_all(&deployment_dir).map_err(|e| e.to_string())?;
        copy_dir_all(&template_dir, &deployment_dir)?;
    }

    let tfvars_path = deployment_dir.join("terraform.tfvars");
    let variables_path = deployment_dir.join("variables.tf");

    // Merge credentials into values for terraform variables that need them
    let mut merged_values = values.clone();
    if let Some(creds) = credentials {
        if let Some(account_id) = creds.databricks_account_id {
            if !account_id.is_empty() {
                merged_values.insert(
                    "databricks_account_id".to_string(),
                    serde_json::Value::String(account_id),
                );
        }
    }

    // Map UI auth type to Terraform databricks_auth_type: azure-cli (Azure Identity),
    // oauth-m2m (service principal), databricks-cli (OAuth/SSO profile)
    let auth_type = match creds.databricks_auth_type.as_deref() {
            Some("profile") => {
                // Check if this is Azure identity mode
                if creds.cloud.as_deref() == Some("azure") && creds.azure_databricks_use_identity == Some(true) {
                    "azure-cli"
                } else {
                    let has_sp_creds = creds
                        .databricks_client_id
                        .as_ref()
                        .map(|s| !s.is_empty())
                        .unwrap_or(false)
                        && creds
                            .databricks_client_secret
                            .as_ref()
                            .map(|s| !s.is_empty())
                            .unwrap_or(false);
                    if has_sp_creds {
                        "oauth-m2m"
                    } else {
                        "databricks-cli"
                    }
                }
            }
            _ => "oauth-m2m",
        };
        merged_values.insert(
            "databricks_auth_type".to_string(),
            serde_json::Value::String(auth_type.to_string()),
        );

        // Azure-specific Databricks variables
        if creds.cloud.as_deref() == Some("azure") {
            if auth_type == "oauth-m2m" {
                if let Some(client_id) = &creds.databricks_client_id {
                    if !client_id.is_empty() {
                        merged_values.insert(
                            "databricks_client_id".to_string(),
                            serde_json::Value::String(client_id.clone()),
                        );
                    }
                }
                if let Some(client_secret) = &creds.databricks_client_secret {
                    if !client_secret.is_empty() {
                        merged_values.insert(
                            "databricks_client_secret".to_string(),
                            serde_json::Value::String(client_secret.clone()),
                        );
                    }
                }
            } else if auth_type == "databricks-cli" {
                // Only write profile for databricks-cli, not for azure-cli
                if let Some(profile) = &creds.databricks_profile {
                    if !profile.is_empty() {
                        merged_values.insert(
                            "databricks_profile".to_string(),
                            serde_json::Value::String(profile.clone()),
                        );
                    }
                }
            }
        }

        if let Some(tenant_id) = &creds.azure_tenant_id {
            if !tenant_id.is_empty() {
                merged_values.insert(
                    "tenant_id".to_string(),
                    serde_json::Value::String(tenant_id.clone()),
                );
            }
        }
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

        // GCP-specific variables
        if creds.cloud.as_deref() == Some("gcp") {
            if let Some(project_id) = &creds.gcp_project_id {
                if !project_id.is_empty() {
                    merged_values.insert(
                        "google_project_name".to_string(),
                        serde_json::Value::String(project_id.clone()),
                    );
                }
            }

            let has_credentials_json = creds
                .gcp_credentials_json
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            let has_sa_email = creds
                .gcp_service_account_email
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);

            let gcp_auth_method = if has_credentials_json {
                "credentials"
            } else {
                "impersonation"
            };

            merged_values.insert(
                "gcp_auth_method".to_string(),
                serde_json::Value::String(gcp_auth_method.to_string()),
            );

            if has_sa_email {
                merged_values.insert(
                    "google_service_account_email".to_string(),
                    serde_json::Value::String(creds.gcp_service_account_email.clone().unwrap()),
                );
            }

            if has_credentials_json {
                merged_values.insert(
                    "google_credentials_json".to_string(),
                    serde_json::Value::String(creds.gcp_credentials_json.clone().unwrap()),
                );
            }
        }
    }

    let variables_content = fs::read_to_string(&variables_path).map_err(|e| e.to_string())?;
    let variables = terraform::parse_variables_tf(&variables_content);

    let tfvars_content = terraform::generate_tfvars(&merged_values, &variables);
    fs::write(&tfvars_path, tfvars_content).map_err(|e| e.to_string())?;

    Ok(deployment_dir.to_string_lossy().to_string())
}

/// Run a Terraform command (init, apply, destroy, etc.) in a background thread.
#[tauri::command]
pub async fn run_terraform_command(
    app: AppHandle,
    deployment_name: String,
    command: String,
    credentials: CloudCredentials,
) -> Result<(), String> {
    let safe_deployment_name = sanitize_deployment_name(&deployment_name)?;

    // Check if a Terraform deployment is already in progress
    {
        let proc = CURRENT_PROCESS.lock().map_err(|e| e.to_string())?;
        if let Some(pid) = *proc {
            #[cfg(unix)]
            {
                use std::process::Command;
                let output = Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output();
                if output.is_ok() && output.unwrap().status.success() {
                    return Err("A deployment is already running".to_string());
                }
            }
            #[cfg(windows)]
            {
                let status = DEPLOYMENT_STATUS.lock().map_err(|e| e.to_string())?;
                if status.running {
                    return Err("A deployment is already running".to_string());
                }
            }
        }
    }

    let deployments_dir = get_deployments_dir(&app)?;
    let deployment_dir = deployments_dir.join(&safe_deployment_name);

    if !deployment_dir.exists() {
        return Err("Deployment not found. Please save configuration first.".to_string());
    }

    let env_vars = build_env_vars(&credentials);

    // Reset deployment status before starting Terraform
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
                if let Ok(mut proc) = process_clone.lock() {
                    *proc = Some(child.id());
                }

                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let status_for_stdout = status_clone.clone();
                let status_for_stderr = status_clone.clone();

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

                if let Some(handle) = stdout_handle {
                    let _ = handle.join();
                }
                if let Some(handle) = stderr_handle {
                    let _ = handle.join();
                }

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

/// Get current deployment status.
#[tauri::command]
pub fn get_deployment_status() -> Result<DeploymentStatus, String> {
    let status = DEPLOYMENT_STATUS.lock().map_err(|e| e.to_string())?;
    Ok(status.clone())
}

/// Reset deployment status to default.
#[tauri::command]
pub fn reset_deployment_status() -> Result<(), String> {
    let mut status = DEPLOYMENT_STATUS.lock().map_err(|e| e.to_string())?;
    *status = DeploymentStatus::default();
    Ok(())
}

/// Cancel a running deployment by killing the Terraform process.
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

        if let Ok(mut status) = DEPLOYMENT_STATUS.lock() {
            status.running = false;
            status.success = Some(false);
            status.output.push_str("\n\nDeployment cancelled by user.");
        }
    }

    Ok(())
}

/// Rollback a deployment (runs `terraform destroy`).
#[tauri::command]
pub async fn rollback_deployment(
    app: AppHandle,
    deployment_name: String,
    credentials: CloudCredentials,
) -> Result<(), String> {
    run_terraform_command(app, deployment_name, "destroy".to_string(), credentials).await
}

/// Read cloud credentials from environment / CLI config.
#[tauri::command]
pub fn get_cloud_credentials(cloud: String) -> Result<CloudCredentials, String> {
    let mut creds = CloudCredentials {
        cloud: Some(cloud.clone()),
        ..Default::default()
    };

    match cloud.as_str() {
        "aws" => {
            creds.aws_access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
            creds.aws_secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
            creds.aws_session_token = std::env::var("AWS_SESSION_TOKEN").ok();
            creds.aws_region = std::env::var("AWS_DEFAULT_REGION")
                .ok()
                .or_else(|| std::env::var("AWS_REGION").ok());
        }
        "azure" => {
            creds.azure_tenant_id = std::env::var("ARM_TENANT_ID").ok();
            creds.azure_subscription_id = std::env::var("ARM_SUBSCRIPTION_ID").ok();
            creds.azure_client_id = std::env::var("ARM_CLIENT_ID").ok();
            creds.azure_client_secret = std::env::var("ARM_CLIENT_SECRET").ok();
        }
        "gcp" => {
            creds.gcp_project_id = std::env::var("GOOGLE_PROJECT")
                .ok()
                .or_else(|| std::env::var("GCLOUD_PROJECT").ok());
            creds.gcp_credentials_json = std::env::var("GOOGLE_CREDENTIALS").ok();
            creds.gcp_use_adc = Some(creds.gcp_credentials_json.is_none());
        }
        _ => {}
    }

    // Databricks credentials — environment then CLI config
    creds.databricks_account_id = std::env::var("DATABRICKS_ACCOUNT_ID").ok();
    creds.databricks_client_id = std::env::var("DATABRICKS_CLIENT_ID").ok();
    creds.databricks_client_secret = std::env::var("DATABRICKS_CLIENT_SECRET").ok();

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

/// Get the path to the deployments parent directory.
#[tauri::command]
pub fn get_deployments_folder(app: AppHandle) -> Result<String, String> {
    let deployments_dir = get_deployments_dir(&app)?;
    Ok(deployments_dir.to_string_lossy().to_string())
}

/// Open a folder in the system file manager.
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

/// Open a URL in the system default browser.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}
