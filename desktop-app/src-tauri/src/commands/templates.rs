//! Template management commands — setup, listing, variable parsing.

use super::{
    copy_dir_all, get_templates_dir, sanitize_template_id, Template, INTERNAL_VARIABLES,
    TEMPLATES_VERSION,
};
use crate::terraform;
use std::fs;
use tauri::{AppHandle, Manager};

/// Copy bundled templates into app-data on first run (or version change).
pub fn setup_templates(app: &AppHandle) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let templates_dir = app_data_dir.join("templates");
    let version_file = app_data_dir.join(".templates_version");

    // Check if we need to update templates
    let needs_update = if templates_dir.exists() {
        match fs::read_to_string(&version_file) {
            Ok(version) => version.trim() != TEMPLATES_VERSION,
            Err(_) => true,
        }
    } else {
        true
    };

    if !needs_update {
        return Ok(());
    }

    // Remove old templates to copy fresh ones (version changed or first run)
    if templates_dir.exists() {
        fs::remove_dir_all(&templates_dir)
            .map_err(|e| format!("Failed to remove old templates: {}", e))?;
    }

    fs::create_dir_all(&templates_dir).map_err(|e| e.to_string())?;

    // Copy embedded templates
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;

    let source_templates = resource_dir.join("templates");

    // Try resource dir first (production), then fall back to dev location
    let templates_source = if source_templates.exists() {
        source_templates
    } else {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut search_path = exe_path.parent();

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
            None => {
                return Err(
                    "Templates not found in resource dir or src-tauri directory".to_string(),
                )
            }
        }
    };

    copy_dir_all(&templates_source, &templates_dir)?;

    // Write version file
    fs::write(&version_file, TEMPLATES_VERSION)
        .map_err(|e| format!("Failed to write version: {}", e))?;

    Ok(())
}

/// Clear cached templates and force refresh.
#[tauri::command]
pub fn clear_templates_cache(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let templates_dir = app_data_dir.join("templates");
    let version_file = app_data_dir.join(".templates_version");

    if templates_dir.exists() {
        fs::remove_dir_all(&templates_dir)
            .map_err(|e| format!("Failed to remove templates: {}", e))?;
    }

    if version_file.exists() {
        fs::remove_file(&version_file)
            .map_err(|e| format!("Failed to remove version file: {}", e))?;
    }

    setup_templates(&app)?;

    Ok("Templates cache cleared and refreshed".to_string())
}

/// List available deployment templates.
#[tauri::command]
pub fn get_templates(app: AppHandle) -> Result<Vec<Template>, String> {
    let templates_dir = get_templates_dir(&app)?;
    let mut templates = Vec::new();

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

    if templates_dir.join("gcp-simple").exists() {
        templates.push(Template {
            id: "gcp-simple".to_string(),
            name: "GCP Standard BYOVPC".to_string(),
            cloud: "gcp".to_string(),
            description: "Secure baseline deployment with customer-managed VPC".to_string(),
            features: vec![
                "Customer-managed VPC (BYOVPC)".to_string(),
                "Cloud NAT for outbound access".to_string(),
                "Service account authentication".to_string(),
                "Metastore auto-detection/creation".to_string(),
                "Production-ready security".to_string(),
                "Unity Catalog integration".to_string(),
            ],
        });
    }

    Ok(templates)
}

/// Parse and return the Terraform variables for a given template.
#[tauri::command]
pub fn get_template_variables(
    app: AppHandle,
    template_id: String,
) -> Result<Vec<terraform::TerraformVariable>, String> {
    let safe_template_id = sanitize_template_id(&template_id)?;

    let templates_dir = get_templates_dir(&app)?;
    let variables_path = templates_dir.join(&safe_template_id).join("variables.tf");

    if !variables_path.exists() {
        return Err(format!("Template not found: {}", safe_template_id));
    }

    let content = fs::read_to_string(&variables_path).map_err(|e| e.to_string())?;
    let variables = terraform::parse_variables_tf(&content);

    // Filter out internal variables that are automatically set by the app
    let filtered_variables: Vec<terraform::TerraformVariable> = variables
        .into_iter()
        .filter(|v| !INTERNAL_VARIABLES.contains(&v.name.as_str()))
        .collect();

    Ok(filtered_variables)
}
