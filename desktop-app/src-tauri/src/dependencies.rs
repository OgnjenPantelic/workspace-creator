use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use which::which;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub required: bool,
    pub install_url: String,
}

/// Find terraform binary by checking common installation paths
/// macOS GUI apps don't inherit shell PATH, so we check explicit locations
pub fn find_terraform_path() -> Option<PathBuf> {
    // Determine the binary name based on OS
    #[cfg(target_os = "windows")]
    let binary_name = "terraform.exe";
    #[cfg(not(target_os = "windows"))]
    let binary_name = "terraform";

    // Check our own install directory first
    let app_install_path = get_terraform_install_path().join(binary_name);
    if app_install_path.exists() {
        return Some(app_install_path);
    }

    #[cfg(target_os = "windows")]
    {
        // Common installation paths on Windows
        let common_paths = [
            "C:\\Program Files\\Terraform\\terraform.exe",
            "C:\\Program Files (x86)\\Terraform\\terraform.exe",
            "C:\\HashiCorp\\Terraform\\terraform.exe",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Check in user's local app data
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join("Programs").join("Terraform").join("terraform.exe");
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Common installation paths on macOS/Linux
        let common_paths = [
            "/usr/local/bin/terraform",
            "/opt/homebrew/bin/terraform",      // Homebrew on Apple Silicon
            "/usr/bin/terraform",
            "/bin/terraform",
            "/opt/local/bin/terraform",         // MacPorts
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // Fall back to which (works if PATH is properly set)
    if let Ok(p) = which(binary_name) {
        return Some(p);
    }

    None
}

pub fn check_terraform() -> DependencyStatus {
    let mut status = DependencyStatus {
        name: "Terraform".to_string(),
        installed: false,
        version: None,
        required: true,
        install_url: "https://developer.hashicorp.com/terraform/install".to_string(),
    };

    if let Some(terraform_path) = find_terraform_path() {
        status.installed = true;
        if let Ok(output) = Command::new(&terraform_path).arg("version").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                // Extract version from "Terraform v1.x.x"
                if let Some(line) = stdout.lines().next() {
                    if let Some(version) = line.strip_prefix("Terraform v") {
                        status.version = Some(version.split_whitespace().next().unwrap_or(version).to_string());
                    }
                }
            }
        }
    }

    status
}

/// Find AWS CLI binary by checking common installation paths
pub fn find_aws_cli_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Common installation paths on Windows
        let common_paths = [
            "C:\\Program Files\\Amazon\\AWSCLIV2\\aws.exe",
            "C:\\Program Files (x86)\\Amazon\\AWSCLIV2\\aws.exe",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Fall back to which
        if let Ok(p) = which("aws") {
            return Some(p);
        }
        // Also try aws.cmd which is common on Windows
        if let Ok(p) = which("aws.cmd") {
            return Some(p);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Common installation paths on macOS/Linux
        let common_paths = [
            "/usr/local/bin/aws",
            "/opt/homebrew/bin/aws",           // Homebrew on Apple Silicon
            "/usr/bin/aws",
            "/bin/aws",
            "/opt/local/bin/aws",              // MacPorts
            "/Library/Frameworks/Python.framework/Versions/Current/bin/aws", // Python install
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Fall back to which (works if PATH is properly set)
        if let Ok(p) = which("aws") {
            return Some(p);
        }
    }

    None
}

pub fn check_aws_cli() -> DependencyStatus {
    let mut status = DependencyStatus {
        name: "AWS CLI".to_string(),
        installed: false,
        version: None,
        required: false,
        install_url: "https://aws.amazon.com/cli/".to_string(),
    };

    if let Some(aws_path) = find_aws_cli_path() {
        status.installed = true;
        if let Ok(output) = Command::new(&aws_path).arg("--version").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                status.version = Some(stdout.trim().to_string());
            }
        }
    }

    status
}

/// Find Azure CLI binary by checking common installation paths
pub fn find_azure_cli_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Common installation paths on Windows
        let common_paths = [
            "C:\\Program Files\\Microsoft SDKs\\Azure\\CLI2\\wbin\\az.cmd",
            "C:\\Program Files (x86)\\Microsoft SDKs\\Azure\\CLI2\\wbin\\az.cmd",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Fall back to which
        if let Ok(p) = which("az") {
            return Some(p);
        }
        if let Ok(p) = which("az.cmd") {
            return Some(p);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Common installation paths on macOS/Linux
        let common_paths = [
            "/usr/local/bin/az",
            "/opt/homebrew/bin/az",            // Homebrew on Apple Silicon
            "/usr/bin/az",
            "/bin/az",
            "/opt/local/bin/az",               // MacPorts
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Fall back to which (works if PATH is properly set)
        if let Ok(p) = which("az") {
            return Some(p);
        }
    }

    None
}

pub fn check_azure_cli() -> DependencyStatus {
    let mut status = DependencyStatus {
        name: "Azure CLI".to_string(),
        installed: false,
        version: None,
        required: false,
        install_url: "https://docs.microsoft.com/en-us/cli/azure/install-azure-cli".to_string(),
    };

    if let Some(az_path) = find_azure_cli_path() {
        status.installed = true;
        if let Ok(output) = Command::new(&az_path).arg("--version").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(line) = stdout.lines().next() {
                    status.version = Some(line.trim().to_string());
                }
            }
        }
    }

    status
}

/// Find gcloud CLI binary by checking common installation paths
pub fn find_gcloud_cli_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let binary_name = "gcloud.cmd";
    #[cfg(not(target_os = "windows"))]
    let binary_name = "gcloud";

    // Check user's home directory for gcloud SDK install
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "windows")]
        let sdk_path = home.join("AppData").join("Local").join("Google").join("Cloud SDK").join("google-cloud-sdk").join("bin").join("gcloud.cmd");
        #[cfg(not(target_os = "windows"))]
        let sdk_path = home.join("google-cloud-sdk").join("bin").join("gcloud");
        
        if sdk_path.exists() {
            return Some(sdk_path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Common installation paths on Windows
        let common_paths = [
            "C:\\Program Files\\Google\\Cloud SDK\\google-cloud-sdk\\bin\\gcloud.cmd",
            "C:\\Program Files (x86)\\Google\\Cloud SDK\\google-cloud-sdk\\bin\\gcloud.cmd",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Common installation paths on macOS/Linux
        let common_paths = [
            "/usr/local/bin/gcloud",
            "/opt/homebrew/bin/gcloud",        // Homebrew on Apple Silicon
            "/usr/bin/gcloud",
            "/bin/gcloud",
            "/opt/local/bin/gcloud",           // MacPorts
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // Fall back to which (works if PATH is properly set)
    if let Ok(p) = which(binary_name) {
        return Some(p);
    }

    None
}

pub fn check_gcloud_cli() -> DependencyStatus {
    let mut status = DependencyStatus {
        name: "Google Cloud CLI".to_string(),
        installed: false,
        version: None,
        required: false,
        install_url: "https://cloud.google.com/sdk/docs/install".to_string(),
    };

    if let Some(gcloud_path) = find_gcloud_cli_path() {
        status.installed = true;
        if let Ok(output) = Command::new(&gcloud_path).arg("--version").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(line) = stdout.lines().next() {
                    status.version = Some(line.trim().to_string());
                }
            }
        }
    }

    status
}

#[cfg(target_os = "macos")]
pub fn get_terraform_download_url() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "https://releases.hashicorp.com/terraform/1.9.8/terraform_1.9.8_darwin_arm64.zip"
    } else {
        "https://releases.hashicorp.com/terraform/1.9.8/terraform_1.9.8_darwin_amd64.zip"
    }
}

#[cfg(target_os = "windows")]
pub fn get_terraform_download_url() -> &'static str {
    "https://releases.hashicorp.com/terraform/1.9.8/terraform_1.9.8_windows_amd64.zip"
}

#[cfg(target_os = "linux")]
pub fn get_terraform_download_url() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "https://releases.hashicorp.com/terraform/1.9.8/terraform_1.9.8_linux_arm64.zip"
    } else {
        "https://releases.hashicorp.com/terraform/1.9.8/terraform_1.9.8_linux_amd64.zip"
    }
}

pub fn get_terraform_install_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        let bin_dir = home.join(".databricks-deployer").join("bin");
        std::fs::create_dir_all(&bin_dir).ok();
        bin_dir
    } else {
        std::path::PathBuf::from(".")
    }
}
