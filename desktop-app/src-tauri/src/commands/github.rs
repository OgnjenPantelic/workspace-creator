//! Git and GitHub integration commands.
//!
//! Provides local git repository initialization, remote connectivity checks,
//! push-to-remote functionality, GitHub OAuth device flow, and repository
//! creation for deployment directories.

use super::{debug_log, get_deployments_dir, http_client, sanitize_deployment_name};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

// ─── Types ──────────────────────────────────────────────────────────────────

/// Status of the git repository for a deployment.
#[derive(Debug, Serialize, Deserialize)]
pub struct GitRepoStatus {
    pub initialized: bool,
    pub has_remote: bool,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub commit_count: u32,
}

/// Result of a git operation (init, push, etc.).
#[derive(Debug, Serialize, Deserialize)]
pub struct GitOperationResult {
    pub success: bool,
    pub message: String,
}

/// Preview entry for a terraform variable in the tfvars.example preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TfVarPreviewEntry {
    pub name: String,
    pub value: String,
    pub is_sensitive: bool,
    pub placeholder: String,
}

/// GitHub OAuth device code response.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// GitHub OAuth poll result.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceAuthPollResult {
    pub status: String,
    pub access_token: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

/// GitHub auth status (persisted).
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubAuthStatus {
    pub authenticated: bool,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

/// GitHub repository creation result.
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub clone_url: String,
    pub html_url: String,
}

/// Persisted GitHub settings.
#[derive(Debug, Default, Serialize, Deserialize)]
struct GitHubSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
}

// ─── Constants ──────────────────────────────────────────────────────────────

const GITHUB_CLIENT_ID: &str = "Ov23li5N6OoUQV5Cg45d";

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Resolve the deployment directory path from its name.
fn resolve_deployment_dir(app: &AppHandle, deployment_name: &str) -> Result<PathBuf, String> {
    let safe_name = sanitize_deployment_name(deployment_name)?;
    let deployments_dir = get_deployments_dir(app)?;
    let deployment_dir = deployments_dir.join(&safe_name);

    if !deployment_dir.exists() {
        return Err(format!("Deployment directory not found: {}", safe_name));
    }

    Ok(deployment_dir)
}

/// Run a git command in the given directory, returning (stdout, stderr, success).
fn run_git(dir: &Path, args: &[&str]) -> Result<(String, String, bool), String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((stdout, stderr, output.status.success()))
}

/// Ensure .gitignore properly excludes *.tfvars before any git operations.
/// Appends the rules if they're missing (safety net for older templates).
fn ensure_tfvars_ignored(deployment_dir: &Path) -> Result<(), String> {
    let gitignore_path = deployment_dir.join(".gitignore");

    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    let has_tfvars_rule = content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "*.tfvars" || trimmed == "*.tfvars.json"
    });

    if !has_tfvars_rule {
        let addition = if content.is_empty() || content.ends_with('\n') {
            "# Terraform variable files (may contain secrets)\n*.tfvars\n*.tfvars.json\n!*.tfvars.example\n"
        } else {
            "\n\n# Terraform variable files (may contain secrets)\n*.tfvars\n*.tfvars.json\n!*.tfvars.example\n"
        };

        fs::write(&gitignore_path, format!("{}{}", content, addition))
            .map_err(|e| format!("Failed to update .gitignore: {}", e))?;

        debug_log!("[github] Appended *.tfvars rules to .gitignore");
    }

    Ok(())
}

// ─── Token Encryption ───────────────────────────────────────────────────────

fn get_github_keyfile_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("github-keyfile"))
}

fn get_or_create_github_key(app: &AppHandle) -> Result<[u8; 32], String> {
    let keyfile_path = get_github_keyfile_path(app)?;

    if keyfile_path.exists() {
        let key_bytes = fs::read(&keyfile_path).map_err(|e| e.to_string())?;
        if key_bytes.len() != 32 {
            return Err("Corrupted GitHub encryption key file".to_string());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    } else {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        fs::write(&keyfile_path, &key)
            .map_err(|e| format!("Failed to save GitHub encryption key: {}", e))?;
        Ok(key)
    }
}

fn encrypt_token(plaintext: &str, enc_key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(enc_key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&combined);
    Ok(format!("enc:v1:{}", encoded))
}

fn decrypt_token(encrypted: &str, enc_key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(enc_key.into());

    let encoded = encrypted
        .strip_prefix("enc:v1:")
        .ok_or_else(|| "Invalid encrypted token format".to_string())?;

    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Invalid encrypted token: {}", e))?;

    if combined.len() < 12 {
        return Err("Invalid encrypted token: too short".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8 in decrypted token: {}", e))
}

// ─── GitHub Settings I/O ────────────────────────────────────────────────────

fn get_github_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("github-settings.json"))
}

fn load_github_settings(app: &AppHandle) -> Result<GitHubSettings, String> {
    let path = get_github_settings_path(app)?;
    if !path.exists() {
        return Ok(GitHubSettings::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse GitHub settings: {}", e))
}

fn save_github_settings(app: &AppHandle, settings: &GitHubSettings) -> Result<(), String> {
    let path = get_github_settings_path(app)?;
    let content =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to save GitHub settings: {}", e))
}

/// Decrypt the stored GitHub token, returning None if missing or invalid.
fn get_decrypted_token(app: &AppHandle) -> Result<Option<String>, String> {
    let settings = load_github_settings(app)?;
    let encrypted = match settings.github_token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };
    let enc_key = get_or_create_github_key(app)?;
    match decrypt_token(&encrypted, &enc_key) {
        Ok(token) => Ok(Some(token)),
        Err(_) => Ok(None),
    }
}

// ─── Tfvars Parsing ─────────────────────────────────────────────────────────

/// Parse a terraform.tfvars file into a map of variable name -> raw value string.
fn parse_tfvars_file(content: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value_start)) = trimmed.split_once('=') {
            let key = key.trim().to_string();
            let value_start = value_start.trim();

            if value_start.starts_with('{') || value_start.starts_with('[') {
                let mut buf = value_start.to_string();
                let open = if value_start.starts_with('{') { '{' } else { '[' };
                let close = if open == '{' { '}' } else { ']' };
                let mut depth = value_start.matches(open).count() as i32
                    - value_start.matches(close).count() as i32;

                while depth > 0 {
                    if let Some(next) = lines.next() {
                        buf.push('\n');
                        buf.push_str(next.trim());
                        depth += next.matches(open).count() as i32;
                        depth -= next.matches(close).count() as i32;
                    } else {
                        break;
                    }
                }
                result.insert(key, buf);
            } else if value_start.starts_with('"') {
                let inner = value_start
                    .trim_start_matches('"')
                    .trim_end_matches('"');
                result.insert(key, inner.to_string());
            } else {
                result.insert(key, value_start.to_string());
            }
        }
    }

    result
}

// ─── Commands ───────────────────────────────────────────────────────────────

/// Get the git repository status for a deployment directory.
#[tauri::command]
pub fn git_get_status(app: AppHandle, deployment_name: String) -> Result<GitRepoStatus, String> {
    let dir = resolve_deployment_dir(&app, &deployment_name)?;
    let git_dir = dir.join(".git");

    if !git_dir.exists() {
        return Ok(GitRepoStatus {
            initialized: false,
            has_remote: false,
            remote_url: None,
            branch: None,
            commit_count: 0,
        });
    }

    let branch = run_git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .and_then(|(stdout, _, ok)| if ok { Some(stdout.trim().to_string()) } else { None });

    let remote_url = run_git(&dir, &["remote", "get-url", "origin"])
        .ok()
        .and_then(|(stdout, _, ok)| if ok { Some(stdout.trim().to_string()) } else { None });

    let commit_count = run_git(&dir, &["rev-list", "--count", "HEAD"])
        .ok()
        .and_then(|(stdout, _, ok)| {
            if ok { stdout.trim().parse::<u32>().ok() } else { None }
        })
        .unwrap_or(0);

    Ok(GitRepoStatus {
        initialized: true,
        has_remote: remote_url.is_some(),
        remote_url,
        branch,
        commit_count,
    })
}

/// Build preview entries by cross-referencing variables.tf metadata with
/// the actual values in terraform.tfvars.
fn build_preview_entries(dir: &Path) -> Result<Vec<TfVarPreviewEntry>, String> {
    let variables_path = dir.join("variables.tf");
    let tfvars_path = dir.join("terraform.tfvars");

    if !variables_path.exists() {
        return Err("variables.tf not found in deployment directory".to_string());
    }
    if !tfvars_path.exists() {
        return Err("terraform.tfvars not found in deployment directory".to_string());
    }

    let variables_content = fs::read_to_string(&variables_path).map_err(|e| e.to_string())?;
    let variables = crate::terraform::parse_variables_tf(&variables_content);

    let tfvars_content = fs::read_to_string(&tfvars_path).map_err(|e| e.to_string())?;
    let tfvars_map = parse_tfvars_file(&tfvars_content);

    let entries = variables
        .iter()
        .filter_map(|var| {
            let value = tfvars_map.get(&var.name)?;
            let placeholder = format!("<{}>", var.name.replace('_', "-"));
            let sensitive_placeholder =
                format!("<SENSITIVE - set via TF_VAR_{}>", var.name);

            Some(TfVarPreviewEntry {
                name: var.name.clone(),
                value: if var.sensitive {
                    sensitive_placeholder.clone()
                } else {
                    value.clone()
                },
                is_sensitive: var.sensitive,
                placeholder: if var.sensitive {
                    sensitive_placeholder
                } else {
                    placeholder
                },
            })
        })
        .collect();

    Ok(entries)
}

/// Generate a preview of what terraform.tfvars.example will contain.
#[tauri::command]
pub fn preview_tfvars_example(
    app: AppHandle,
    deployment_name: String,
) -> Result<Vec<TfVarPreviewEntry>, String> {
    let dir = resolve_deployment_dir(&app, &deployment_name)?;
    build_preview_entries(&dir)
}

/// Write terraform.tfvars.example based on preview entries and the chosen mode.
fn write_tfvars_example(dir: &Path, entries: &[TfVarPreviewEntry], include_values: bool) -> Result<(), String> {
    let mut lines = Vec::new();

    for entry in entries {
        let display_value = if entry.is_sensitive {
            &entry.placeholder
        } else if include_values {
            &entry.value
        } else {
            &entry.placeholder
        };

        let needs_quotes = !display_value.starts_with('{')
            && !display_value.starts_with('[')
            && display_value != "true"
            && display_value != "false"
            && display_value.parse::<f64>().is_err();

        if needs_quotes {
            lines.push(format!("{} = \"{}\"", entry.name, display_value));
        } else {
            lines.push(format!("{} = {}", entry.name, display_value));
        }
    }

    let content = lines.join("\n") + "\n";
    fs::write(dir.join("terraform.tfvars.example"), content)
        .map_err(|e| format!("Failed to write terraform.tfvars.example: {}", e))
}

/// Initialize a git repository in the deployment directory with an initial commit.
/// Regenerates terraform.tfvars.example based on the user's chosen mode.
#[tauri::command]
pub fn git_init_repo(
    app: AppHandle,
    deployment_name: String,
    include_values: bool,
) -> Result<GitOperationResult, String> {
    let dir = resolve_deployment_dir(&app, &deployment_name)?;

    if dir.join(".git").exists() {
        return Ok(GitOperationResult {
            success: true,
            message: "Repository already initialized".to_string(),
        });
    }

    if dir.join("variables.tf").exists() && dir.join("terraform.tfvars").exists() {
        let entries = build_preview_entries(&dir)?;
        write_tfvars_example(&dir, &entries, include_values)?;
        debug_log!(
            "[github] Wrote terraform.tfvars.example (include_values={})",
            include_values
        );
    }

    ensure_tfvars_ignored(&dir)?;

    let (_, stderr, ok) = run_git(&dir, &["init", "-b", "main"])?;
    if !ok {
        return Err(format!("git init failed: {}", stderr));
    }

    let (_, stderr, ok) = run_git(&dir, &["add", "."])?;
    if !ok {
        return Err(format!("git add failed: {}", stderr));
    }

    // Verify terraform.tfvars is NOT staged (safety check)
    let (staged, _, _) = run_git(&dir, &["diff", "--cached", "--name-only"])?;
    if staged.lines().any(|f| f == "terraform.tfvars") {
        let _ = run_git(&dir, &["rm", "--cached", "terraform.tfvars"]);
        debug_log!("[github] Removed terraform.tfvars from staging — .gitignore may be stale");
    }

    let (_, stderr, ok) = run_git(
        &dir,
        &["commit", "-m", "Initial infrastructure deployment"],
    )?;
    if !ok {
        return Err(format!("git commit failed: {}", stderr));
    }

    debug_log!("[github] Initialized git repo at {:?}", dir);

    Ok(GitOperationResult {
        success: true,
        message: "Repository initialized with initial commit".to_string(),
    })
}

/// Check if the current git user can access a remote URL.
/// Uses `git ls-remote` as a lightweight connectivity + auth check.
/// Note: empty repos have no HEAD, so we omit `--exit-code` and check stderr instead.
#[tauri::command]
pub fn git_check_remote(app: AppHandle, deployment_name: String, remote_url: String) -> Result<GitOperationResult, String> {
    let dir = resolve_deployment_dir(&app, &deployment_name)?;

    let (stdout, stderr, ok) = run_git(&dir, &["ls-remote", &remote_url])?;

    // Success: either refs were listed, or the repo is empty (no output but no error)
    if ok {
        let ref_count = stdout.lines().filter(|l| !l.is_empty()).count();
        let msg = if ref_count > 0 {
            "Remote is accessible".to_string()
        } else {
            "Remote is accessible (empty repository)".to_string()
        };
        return Ok(GitOperationResult { success: true, message: msg });
    }

    // Failure: classify the error
    let stderr_lower = stderr.to_lowercase();
    let hint = if stderr_lower.contains("authentication failed")
        || stderr_lower.contains("could not read username")
        || stderr_lower.contains("permission denied")
        || stderr_lower.contains("invalid credentials")
    {
        "Authentication failed. Set up SSH keys, run 'gh auth login', or use a Personal Access Token.".to_string()
    } else if stderr_lower.contains("not found")
        || stderr_lower.contains("does not appear to be a git repository")
        || stderr_lower.contains("repository not found")
    {
        "Repository not found. Check the URL and your access permissions.".to_string()
    } else {
        format!("Could not connect to remote: {}", stderr.trim())
    };

    Ok(GitOperationResult {
        success: false,
        message: hint,
    })
}

/// Add a remote and push the repository.
#[tauri::command]
pub fn git_push_to_remote(
    app: AppHandle,
    deployment_name: String,
    remote_url: String,
) -> Result<GitOperationResult, String> {
    let dir = resolve_deployment_dir(&app, &deployment_name)?;

    if !dir.join(".git").exists() {
        return Err("Repository not initialized. Run git init first.".to_string());
    }

    // Check if origin already exists
    let (_, _, has_origin) = run_git(&dir, &["remote", "get-url", "origin"])?;

    if has_origin {
        // Update existing remote
        let (_, stderr, ok) = run_git(&dir, &["remote", "set-url", "origin", &remote_url])?;
        if !ok {
            return Err(format!("Failed to update remote: {}", stderr));
        }
    } else {
        let (_, stderr, ok) = run_git(&dir, &["remote", "add", "origin", &remote_url])?;
        if !ok {
            return Err(format!("Failed to add remote: {}", stderr));
        }
    }

    let (_, stderr, ok) = run_git(&dir, &["push", "-u", "origin", "main"])?;
    if !ok {
        // Provide actionable error messages
        if stderr.contains("Authentication failed")
            || stderr.contains("could not read Username")
            || stderr.contains("Permission denied")
        {
            return Err("Push failed: authentication error. Set up SSH keys, run 'gh auth login', or use a Personal Access Token.".to_string());
        }
        return Err(format!("Push failed: {}", stderr));
    }

    debug_log!("[github] Pushed to remote: {}", remote_url);

    Ok(GitOperationResult {
        success: true,
        message: format!("Pushed to {}", remote_url),
    })
}

// ─── GitHub OAuth Device Flow ───────────────────────────────────────────────

/// Start the GitHub OAuth device flow. Returns a user code for the user to enter on github.com.
#[tauri::command]
pub async fn github_device_auth_start() -> Result<DeviceCodeResponse, String> {
    let client = http_client()?;

    let params = [
        ("client_id", GITHUB_CLIENT_ID),
        ("scope", "repo"),
    ];

    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to start device flow: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("GitHub device auth failed ({}): {}", status, body));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse device code response: {}", e))?;

    Ok(DeviceCodeResponse {
        device_code: body["device_code"]
            .as_str()
            .ok_or("Missing device_code")?
            .to_string(),
        user_code: body["user_code"]
            .as_str()
            .ok_or("Missing user_code")?
            .to_string(),
        verification_uri: body["verification_uri"]
            .as_str()
            .ok_or("Missing verification_uri")?
            .to_string(),
        expires_in: body["expires_in"].as_u64().unwrap_or(900),
        interval: body["interval"].as_u64().unwrap_or(5),
    })
}

/// Poll GitHub for the access token after the user has entered the device code.
#[tauri::command]
pub async fn github_device_auth_poll(
    app: AppHandle,
    device_code: String,
) -> Result<DeviceAuthPollResult, String> {
    let client = http_client()?;

    let params = [
        ("client_id", GITHUB_CLIENT_ID),
        ("device_code", device_code.as_str()),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];

    let resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Poll request failed: {}", e))?;

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse poll response: {}", e))?;

    if let Some(error) = body.get("error").and_then(|e| e.as_str()) {
        return match error {
            "authorization_pending" => Ok(DeviceAuthPollResult {
                status: "pending".to_string(),
                access_token: None,
                username: None,
                avatar_url: None,
            }),
            "slow_down" => Ok(DeviceAuthPollResult {
                status: "slow_down".to_string(),
                access_token: None,
                username: None,
                avatar_url: None,
            }),
            "expired_token" => Ok(DeviceAuthPollResult {
                status: "expired".to_string(),
                access_token: None,
                username: None,
                avatar_url: None,
            }),
            "access_denied" => Ok(DeviceAuthPollResult {
                status: "denied".to_string(),
                access_token: None,
                username: None,
                avatar_url: None,
            }),
            _ => Err(format!("OAuth error: {}", error)),
        };
    }

    let access_token = body["access_token"]
        .as_str()
        .ok_or("Missing access_token in success response")?
        .to_string();

    // Fetch user info
    let user_resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "DatabricksDeployer/1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user info: {}", e))?;

    let user: serde_json::Value = user_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))?;

    let username = user["login"].as_str().map(|s| s.to_string());
    let avatar_url = user["avatar_url"].as_str().map(|s| s.to_string());

    // Persist token
    let enc_key = get_or_create_github_key(&app)?;
    let encrypted = encrypt_token(&access_token, &enc_key)?;
    let mut settings = load_github_settings(&app)?;
    settings.github_token = Some(encrypted);
    settings.github_username = username.clone();
    save_github_settings(&app, &settings)?;

    debug_log!("[github] OAuth token saved for user {:?}", username);

    Ok(DeviceAuthPollResult {
        status: "success".to_string(),
        access_token: None, // never send token to frontend
        username,
        avatar_url,
    })
}

/// Get the current GitHub authentication status.
/// Validates the stored token with the GitHub API.
#[tauri::command]
pub async fn github_get_auth(app: AppHandle) -> Result<GitHubAuthStatus, String> {
    let token = match get_decrypted_token(&app)? {
        Some(t) => t,
        None => {
            return Ok(GitHubAuthStatus {
                authenticated: false,
                username: None,
                avatar_url: None,
            })
        }
    };

    let client = http_client()?;
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "DatabricksDeployer/1.0")
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let user: serde_json::Value = r
                .json()
                .await
                .map_err(|e| format!("Failed to parse user info: {}", e))?;

            Ok(GitHubAuthStatus {
                authenticated: true,
                username: user["login"].as_str().map(|s| s.to_string()),
                avatar_url: user["avatar_url"].as_str().map(|s| s.to_string()),
            })
        }
        Ok(_) => {
            // Token is invalid/revoked — clear it
            let mut settings = load_github_settings(&app)?;
            settings.github_token = None;
            settings.github_username = None;
            save_github_settings(&app, &settings)?;
            debug_log!("[github] Stored token is invalid, cleared");

            Ok(GitHubAuthStatus {
                authenticated: false,
                username: None,
                avatar_url: None,
            })
        }
        Err(_) => {
            // Network error — report cached state if available
            let settings = load_github_settings(&app)?;
            Ok(GitHubAuthStatus {
                authenticated: settings.github_token.is_some(),
                username: settings.github_username,
                avatar_url: None,
            })
        }
    }
}

/// Clear the stored GitHub token.
#[tauri::command]
pub fn github_logout(app: AppHandle) -> Result<(), String> {
    let mut settings = load_github_settings(&app)?;
    settings.github_token = None;
    settings.github_username = None;
    save_github_settings(&app, &settings)?;
    debug_log!("[github] Logged out from GitHub");
    Ok(())
}

// ─── GitHub Repo Creation ───────────────────────────────────────────────────

/// Create a new GitHub repository and push the deployment code to it.
#[tauri::command]
pub async fn github_create_repo(
    app: AppHandle,
    deployment_name: String,
    repo_name: String,
    private: bool,
    description: String,
) -> Result<GitHubRepo, String> {
    let token = get_decrypted_token(&app)?
        .ok_or_else(|| "Not authenticated with GitHub. Connect first.".to_string())?;

    let client = http_client()?;

    let body = serde_json::json!({
        "name": repo_name,
        "private": private,
        "description": description,
        "auto_init": false,
    });

    let resp = client
        .post("https://api.github.com/user/repos")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "DatabricksDeployer/1.0")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to create repository: {}", e))?;

    let status = resp.status();
    let resp_body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if !status.is_success() {
        let msg = if status.as_u16() == 422 {
            let errors = resp_body["errors"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|e| e["message"].as_str())
                .unwrap_or("name already exists on this account");
            format!(
                "A repository with this name already exists. Choose a different name. ({})",
                errors
            )
        } else if status.as_u16() == 403 {
            "GitHub token doesn't have permission to create repos. Reconnect to GitHub.".to_string()
        } else {
            format!(
                "Failed to create repository: {}",
                resp_body["message"].as_str().unwrap_or("Unknown error")
            )
        };
        return Err(msg);
    }

    let clone_url = resp_body["clone_url"]
        .as_str()
        .ok_or("Missing clone_url in response")?
        .to_string();
    let html_url = resp_body["html_url"]
        .as_str()
        .ok_or("Missing html_url in response")?
        .to_string();

    // Push using token-authenticated URL for this push only, then reset to clean URL
    let dir = resolve_deployment_dir(&app, &deployment_name)?;

    if !dir.join(".git").exists() {
        return Err("Repository not initialized. Run git init first.".to_string());
    }

    let owner = resp_body["owner"]["login"]
        .as_str()
        .ok_or("Missing owner in response")?;
    let authenticated_url = format!(
        "https://x-access-token:{}@github.com/{}/{}.git",
        token, owner, repo_name
    );

    // Set authenticated remote, push, then reset to clean URL
    let (_, _, has_origin) = run_git(&dir, &["remote", "get-url", "origin"])?;
    if has_origin {
        let (_, stderr, ok) =
            run_git(&dir, &["remote", "set-url", "origin", &authenticated_url])?;
        if !ok {
            return Err(format!("Failed to set remote: {}", stderr));
        }
    } else {
        let (_, stderr, ok) =
            run_git(&dir, &["remote", "add", "origin", &authenticated_url])?;
        if !ok {
            return Err(format!("Failed to add remote: {}", stderr));
        }
    }

    let (_, stderr, ok) = run_git(&dir, &["push", "-u", "origin", "main"])?;

    // Always reset to clean URL regardless of push success
    let _ = run_git(&dir, &["remote", "set-url", "origin", &clone_url]);

    if !ok {
        return Err(format!("Repository created but push failed: {}", stderr));
    }

    debug_log!("[github] Created and pushed to {}", html_url);

    Ok(GitHubRepo {
        clone_url,
        html_url,
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_tfvars_ignored_creates_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        ensure_tfvars_ignored(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains("*.tfvars"));
        assert!(content.contains("*.tfvars.json"));
        assert!(content.contains("!*.tfvars.example"));
    }

    #[test]
    fn ensure_tfvars_ignored_appends_to_existing() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), ".terraform/\n").unwrap();

        ensure_tfvars_ignored(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.starts_with(".terraform/"));
        assert!(content.contains("*.tfvars"));
    }

    #[test]
    fn ensure_tfvars_ignored_skips_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let original = "*.tfvars\n*.tfvars.json\n";
        fs::write(dir.path().join(".gitignore"), original).unwrap();

        ensure_tfvars_ignored(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(content, original);
    }

    // ── parse_tfvars_file ────────────────────────────────────────────────

    #[test]
    fn parse_tfvars_simple_strings() {
        let content = r#"
region = "us-east-1"
prefix = "my-workspace"
"#;
        let map = parse_tfvars_file(content);
        assert_eq!(map.get("region").unwrap(), "us-east-1");
        assert_eq!(map.get("prefix").unwrap(), "my-workspace");
    }

    #[test]
    fn parse_tfvars_booleans_and_numbers() {
        let content = r#"
enable_logging = true
instance_count = 3
"#;
        let map = parse_tfvars_file(content);
        assert_eq!(map.get("enable_logging").unwrap(), "true");
        assert_eq!(map.get("instance_count").unwrap(), "3");
    }

    #[test]
    fn parse_tfvars_multiline_map() {
        let content = r#"
tags = {
  env  = "prod"
  team = "data"
}
"#;
        let map = parse_tfvars_file(content);
        let tags = map.get("tags").unwrap();
        assert!(tags.contains("env"));
        assert!(tags.contains("prod"));
    }

    #[test]
    fn parse_tfvars_skips_comments() {
        let content = r#"
# This is a comment
region = "us-east-1"
"#;
        let map = parse_tfvars_file(content);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("region").unwrap(), "us-east-1");
    }

    // ── write_tfvars_example ─────────────────────────────────────────────

    #[test]
    fn write_tfvars_example_include_values() {
        let dir = tempfile::tempdir().unwrap();
        let entries = vec![
            TfVarPreviewEntry {
                name: "region".into(),
                value: "us-east-1".into(),
                is_sensitive: false,
                placeholder: "<region>".into(),
            },
            TfVarPreviewEntry {
                name: "password".into(),
                value: "<SENSITIVE - set via TF_VAR_password>".into(),
                is_sensitive: true,
                placeholder: "<SENSITIVE - set via TF_VAR_password>".into(),
            },
        ];

        write_tfvars_example(dir.path(), &entries, true).unwrap();
        let content = fs::read_to_string(dir.path().join("terraform.tfvars.example")).unwrap();
        assert!(content.contains("region = \"us-east-1\""));
        assert!(content.contains("password = \"<SENSITIVE - set via TF_VAR_password>\""));
    }

    #[test]
    fn write_tfvars_example_placeholders_only() {
        let dir = tempfile::tempdir().unwrap();
        let entries = vec![TfVarPreviewEntry {
            name: "region".into(),
            value: "us-east-1".into(),
            is_sensitive: false,
            placeholder: "<region>".into(),
        }];

        write_tfvars_example(dir.path(), &entries, false).unwrap();
        let content = fs::read_to_string(dir.path().join("terraform.tfvars.example")).unwrap();
        assert!(content.contains("region = \"<region>\""));
        assert!(!content.contains("us-east-1"));
    }
}
