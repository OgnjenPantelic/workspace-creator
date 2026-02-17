//! Command handlers for the Tauri desktop application.
//!
//! This module is split into submodules by cloud provider and feature area:
//! - [`aws`] - AWS authentication and permission checking
//! - [`azure`] - Azure authentication and permission checking
//! - [`databricks`] - Databricks authentication and Unity Catalog permissions
//! - [`deployment`] - Terraform deployment, configuration, and lifecycle management
//! - [`gcp`] - GCP authentication, permission checking, and service account management
//! - [`templates`] - Template setup, listing, and variable parsing

pub mod aws;
pub mod azure;
pub mod databricks;
pub mod deployment;
pub mod gcp;
pub mod templates;

// Re-export all commands so lib.rs can reference them as commands::function_name
pub use aws::*;
pub use azure::*;
pub use databricks::*;
pub use deployment::*;
pub use gcp::*;
pub use templates::*;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Debug logging macro — only emits output in debug builds.
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!($($arg)*);
    };
}
pub(crate) use debug_log;

// ─── Shared Types ───────────────────────────────────────────────────────────

/// Terraform deployment template descriptor.
#[derive(Debug, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub cloud: String,
    pub description: String,
    pub features: Vec<String>,
}

/// Cloud provider and Databricks credentials bundle.
///
/// Passed between the frontend and Rust backend to authenticate
/// Terraform runs and CLI operations.
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
    pub azure_databricks_use_identity: Option<bool>,
    pub azure_account_email: Option<String>,
    // GCP
    pub gcp_project_id: Option<String>,
    pub gcp_credentials_json: Option<String>,
    pub gcp_use_adc: Option<bool>,
    pub gcp_oauth_token: Option<String>,
    pub gcp_service_account_email: Option<String>,
    // Databricks
    pub databricks_account_id: Option<String>,
    pub databricks_client_id: Option<String>,
    pub databricks_client_secret: Option<String>,
    pub databricks_profile: Option<String>,
    pub databricks_auth_type: Option<String>,
    // Cloud identifier
    pub cloud: Option<String>,
}

/// Result of a cloud provider permission check.
#[derive(Debug, Serialize, Deserialize)]
pub struct CloudPermissionCheck {
    pub has_all_permissions: bool,
    pub checked_permissions: Vec<String>,
    pub missing_permissions: Vec<String>,
    pub message: String,
    /// `true` = soft warning (can continue), `false` = hard block.
    pub is_warning: bool,
}

/// Unity Catalog metastore info.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetastoreInfo {
    pub exists: bool,
    pub metastore_id: Option<String>,
    pub metastore_name: Option<String>,
    pub region: Option<String>,
}

/// Unity Catalog permission check result.
#[derive(Debug, Serialize, Deserialize)]
pub struct UCPermissionCheck {
    pub metastore: MetastoreInfo,
    pub has_create_catalog: bool,
    pub has_create_external_location: bool,
    pub has_create_storage_credential: bool,
    pub can_create_catalog: bool,
    pub message: String,
}

// ─── Constants ──────────────────────────────────────────────────────────────

/// Increment when embedded templates change to trigger a refresh.
pub(crate) const TEMPLATES_VERSION: &str = "2.40.1";

/// Variables that are automatically set by the app and hidden from the UI form.
pub(crate) const INTERNAL_VARIABLES: &[&str] = &[
    "gcp_auth_method",
    "google_credentials_json",
];

// ─── Helper Functions ───────────────────────────────────────────────────────

/// Recursively copy a directory tree. Used for templates and deployments.
pub(crate) fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
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

/// Resolve the app-data templates directory.
pub(crate) fn get_templates_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("templates"))
}

/// Resolve (and create) the app-data deployments directory.
pub(crate) fn get_deployments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let deployments_dir = app_data_dir.join("deployments");
    fs::create_dir_all(&deployments_dir).map_err(|e| e.to_string())?;
    Ok(deployments_dir)
}

/// Sanitize deployment name to prevent path traversal attacks.
/// Only allows alphanumeric characters, hyphens, and underscores.
pub(crate) fn sanitize_deployment_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Deployment name cannot be empty".to_string());
    }

    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    if sanitized.is_empty() {
        return Err("Deployment name contains no valid characters".to_string());
    }
    if sanitized.starts_with('-') {
        return Err("Deployment name cannot start with a hyphen".to_string());
    }
    if sanitized.len() > 200 {
        return Err("Deployment name is too long (max 200 characters)".to_string());
    }

    Ok(sanitized)
}

/// Sanitize template ID to prevent path traversal attacks.
pub(crate) fn sanitize_template_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err("Template ID cannot be empty".to_string());
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err("Invalid template ID".to_string());
    }
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Template ID contains invalid characters".to_string());
    }
    Ok(id.to_string())
}
