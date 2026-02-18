//! AI assistant commands — Multi-provider LLM integration.
//!
//! Supports GitHub Models (free), OpenAI, and Claude via API keys.
//! The user provides their own API key, which is encrypted at rest using AES-256-GCM.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// ─── Static Knowledge Base ──────────────────────────────────────────────────

/// Embedded at compile time from resources/assistant-knowledge.md.
const KNOWLEDGE_BASE: &str = include_str!("../../resources/assistant-knowledge.md");

// ─── Provider Configuration ─────────────────────────────────────────────────

/// Supported LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProvider {
    GithubModels,
    Openai,
    Claude,
}

impl Default for LlmProvider {
    fn default() -> Self {
        LlmProvider::GithubModels
    }
}

// ─── Types ──────────────────────────────────────────────────────────────────

/// Chat message exchanged between user and assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Persisted assistant settings.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AssistantSettings {
    pub active_provider: LlmProvider,
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_api_key: Option<String>,
    pub github_model: Option<String>,
    pub cached_models: Option<Vec<(String, String)>>,
    pub models_cache_timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_history: Option<Vec<ChatMessage>>,
    #[serde(skip, default)]
    pub has_github_key: bool,
    #[serde(skip, default)]
    pub has_openai_key: bool,
    #[serde(skip, default)]
    pub has_claude_key: bool,
}

/// Response struct for assistant_get_settings that includes computed has_*_key flags.
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    #[serde(flatten)]
    settings: AssistantSettings,
    has_github_key: bool,
    has_openai_key: bool,
    has_claude_key: bool,
}

/// OpenAI-compatible chat completion response (used by GitHub Models and OpenAI).
#[derive(Debug, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    message: CompletionMessage,
}

#[derive(Debug, Deserialize)]
struct CompletionMessage {
    content: String,
}

/// Claude API message response.
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

/// OpenAI error response for parsing detailed error messages.
#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
}

/// GitHub Models catalog model entry.
#[derive(Debug, Deserialize)]
struct CatalogModel {
    id: String,
    name: String,
    #[serde(default)]
    publisher: Option<String>,
}

// ─── GitHub Models List ─────────────────────────────────────────────────────

/// Cache duration for fetched models (24 hours).
const MODELS_CACHE_DURATION_SECS: u64 = 86400;

// ─── Helpers ────────────────────────────────────────────────────────────────

// ─── Encryption Helpers ─────────────────────────────────────────────────────

/// Get the encryption key file path.
fn get_keyfile_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("assistant-keyfile"))
}

/// Get or create the encryption key for API keys.
fn get_or_create_encryption_key(app: &AppHandle) -> Result<[u8; 32], String> {
    let keyfile_path = get_keyfile_path(app)?;
    
    if keyfile_path.exists() {
        let key_bytes = fs::read(&keyfile_path).map_err(|e| e.to_string())?;
        if key_bytes.len() != 32 {
            return Err("Corrupted encryption key file".to_string());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    } else {
        // Generate a new 256-bit key
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        fs::write(&keyfile_path, &key).map_err(|e| format!("Failed to save encryption key: {}", e))?;
        Ok(key)
    }
}

/// Encrypt an API key using AES-256-GCM.
fn encrypt_key(plaintext: &str, enc_key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(enc_key.into());
    
    // Generate a random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    // Concatenate nonce + ciphertext and encode as base64, with prefix
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&combined);
    Ok(format!("enc:v1:{}", encoded))
}

/// Decrypt an API key using AES-256-GCM.
fn decrypt_key(encrypted: &str, enc_key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(enc_key.into());
    
    // Strip the "enc:v1:" prefix
    let encoded = encrypted.strip_prefix("enc:v1:")
        .ok_or_else(|| "Invalid encrypted key format: missing prefix".to_string())?;
    
    // Decode from base64
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Invalid encrypted key format: {}", e))?;
    
    if combined.len() < 12 {
        return Err("Invalid encrypted key: too short".to_string());
    }
    
    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    
    String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8 in decrypted key: {}", e))
}

/// Check if a string is an encrypted key (has the "enc:v1:" prefix).
fn is_encrypted(value: &str) -> bool {
    value.starts_with("enc:v1:")
}

// ─── File I/O Helpers ───────────────────────────────────────────────────────

/// Create an HTTP client with timeout and required headers.
fn http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .user_agent("DatabricksDeployer/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Resolve the assistant settings file path.
fn get_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("assistant-settings.json"))
}

/// Load settings from disk, returning defaults if file doesn't exist.
/// Automatically migrates plaintext keys to encrypted format on first load.
fn load_settings(app: &AppHandle) -> Result<AssistantSettings, String> {
    let path = get_settings_path(app)?;
    if !path.exists() {
        return Ok(AssistantSettings::default());
    }
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut settings: AssistantSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse assistant settings: {}", e))?;
    
    // Migrate plaintext keys to encrypted format
    let enc_key = get_or_create_encryption_key(app)?;
    let mut needs_save = false;
    
    if let Some(ref key) = settings.github_api_key {
        if !is_encrypted(key) {
            settings.github_api_key = Some(encrypt_key(key, &enc_key)?);
            needs_save = true;
        }
    }
    
    if let Some(ref key) = settings.openai_api_key {
        if !is_encrypted(key) {
            settings.openai_api_key = Some(encrypt_key(key, &enc_key)?);
            needs_save = true;
        }
    }
    
    if let Some(ref key) = settings.claude_api_key {
        if !is_encrypted(key) {
            settings.claude_api_key = Some(encrypt_key(key, &enc_key)?);
            needs_save = true;
        }
    }
    
    // Save migrated settings
    if needs_save {
        save_settings_to_disk(app, &settings)?;
    }
    
    Ok(settings)
}

/// Save settings to disk.
fn save_settings_to_disk(app: &AppHandle, settings: &AssistantSettings) -> Result<(), String> {
    let path = get_settings_path(app)?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to save settings: {}", e))
}

/// Assemble the full system prompt from the three layers.
fn build_system_prompt(screen_context: &str, state_metadata: &str) -> String {
    let mut prompt = String::with_capacity(KNOWLEDGE_BASE.len() + 512);
    prompt.push_str(KNOWLEDGE_BASE);
    prompt.push_str("\n\n# Current Screen Context\n\n");
    prompt.push_str(screen_context);
    if !state_metadata.is_empty() {
        prompt.push_str("\n\n# Current App State\n\n");
        prompt.push_str(state_metadata);
    }
    prompt
}

/// Validate an API key by making a test request to the provider's API.
async fn validate_api_key(
    provider: &LlmProvider,
    api_key: &str,
    client: &reqwest::Client,
) -> Result<(), String> {
    match provider {
        LlmProvider::GithubModels => {
            let response = client
                .post("https://models.github.ai/inference/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .json(&serde_json::json!({
                    "model": "openai/gpt-4o-mini",
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to validate GitHub token: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                
                if status.as_u16() == 429 {
                    return Err("Rate limit exceeded. Please wait a moment and try again.".to_string());
                }
                
                if status.as_u16() == 403 || status.as_u16() == 401 {
                    return Err("GitHub token is invalid or missing 'models:read' permission. Please create a Fine-grained Personal Access Token with Account permissions → Models → Read-only access.".to_string());
                }
                
                return Err(format!("Invalid GitHub token ({}): {}", status, body));
            }
            Ok(())
        }
        LlmProvider::Openai => {
            let response = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": "gpt-4o-mini",
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to validate OpenAI token: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                
                if status.as_u16() == 429 {
                    // Try to parse OpenAI's detailed error message
                    if let Ok(error_response) = serde_json::from_str::<OpenAIError>(&body) {
                        return Err(error_response.error.message);
                    }
                    return Err("Rate limit or quota exceeded. Please check your OpenAI account.".to_string());
                }
                
                return Err(format!("Invalid OpenAI API key ({}): {}", status, body));
            }
            Ok(())
        }
        LlmProvider::Claude => {
            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": "claude-3-5-haiku-latest",
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to validate Claude token: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                
                if status.as_u16() == 429 {
                    return Err("Rate limit exceeded. Please wait a moment and try again.".to_string());
                }
                
                return Err(format!("Invalid Claude API key ({}): {}", status, body));
            }
            Ok(())
        }
    }
}

/// Call an OpenAI-compatible chat completions API (GitHub Models or OpenAI).
async fn call_openai_compatible(
    url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    message: &str,
    history: &[ChatMessage],
    client: &reqwest::Client,
    provider_name: &str,
) -> Result<String, String> {
    // Build messages array: system prompt + history + new user message
    let mut messages: Vec<serde_json::Value> = Vec::with_capacity(history.len() + 2);

    messages.push(serde_json::json!({
        "role": "system",
        "content": system_prompt,
    }));

    for msg in history {
        messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": message,
    }));

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.05,
        "max_tokens": 1024,
    });

    let mut request = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json");

    // GitHub Models requires additional headers
    if provider_name == "GitHub Models" {
        request = request
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
    }

    let response = request
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call {} API: {}", provider_name, e))?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();

        if status.as_u16() == 429 {
            // Try to parse OpenAI's detailed error message for OpenAI provider
            if provider_name == "OpenAI" || provider_name == "GitHub Models" {
                if let Ok(error_response) = serde_json::from_str::<OpenAIError>(&body) {
                    return Err(error_response.error.message);
                }
            }
            return Err("Rate limit reached. Please wait a moment and try again.".to_string());
        }

        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(format!("{} token expired or invalid. Please disconnect and reconnect.", provider_name));
        }

        return Err(format!("{} API error ({}): {}", provider_name, status, body));
    }

    let completion: CompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let reply = completion
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "No response from the assistant.".to_string());

    Ok(reply)
}

/// Call the Claude API for chat completions.
async fn call_claude(
    api_key: &str,
    system_prompt: &str,
    message: &str,
    history: &[ChatMessage],
    client: &reqwest::Client,
) -> Result<String, String> {
    // Claude uses a different message format - system is separate
    let mut claude_messages: Vec<serde_json::Value> = Vec::with_capacity(history.len() + 1);

    for msg in history {
        claude_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }

    claude_messages.push(serde_json::json!({
        "role": "user",
        "content": message,
    }));

    let body = serde_json::json!({
        "model": "claude-3-5-haiku-latest",
        "system": system_prompt,
        "messages": claude_messages,
        "temperature": 0.05,
        "max_tokens": 1024,
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call Claude API: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();

        if status.as_u16() == 429 {
            return Err("Rate limit reached. Please wait a moment and try again.".to_string());
        }

        if status.as_u16() == 401 {
            return Err("Claude API key expired or invalid. Please disconnect and reconnect.".to_string());
        }

        return Err(format!("Claude API error ({}): {}", status, body));
    }

    let claude_response: ClaudeResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let reply = claude_response
        .content
        .first()
        .map(|c| c.text.clone())
        .unwrap_or_else(|| "No response from the assistant.".to_string());

    Ok(reply)
}

// ─── Tauri Commands ─────────────────────────────────────────────────────────

/// Save an API key for the specified provider.
///
/// Validates the key by making a lightweight test request to the provider's API.
#[tauri::command]
pub async fn assistant_save_token(
    provider: String,
    api_key: String,
    app: AppHandle,
) -> Result<(), String> {
    // Parse provider string to enum
    let provider_enum: LlmProvider = serde_json::from_str(&format!("\"{}\"", provider))
        .map_err(|_| format!("Unknown provider: {}", provider))?;

    // Validate the API key by making a simple test request
    let client = http_client(15)?;
    validate_api_key(&provider_enum, &api_key, &client).await?;

    // Load existing settings to preserve cache and model selection
    let mut settings = load_settings(&app).unwrap_or_default();
    
    // Only clear cache if switching providers
    let switching_providers = settings.active_provider != provider_enum;
    
    settings.active_provider = provider_enum.clone();
    settings.configured = true;
    
    // Encrypt the API key before saving
    let enc_key = get_or_create_encryption_key(&app)?;
    let encrypted_key = encrypt_key(&api_key, &enc_key)?;
    
    // Save to provider-specific field
    match provider_enum {
        LlmProvider::GithubModels => settings.github_api_key = Some(encrypted_key),
        LlmProvider::Openai => settings.openai_api_key = Some(encrypted_key),
        LlmProvider::Claude => settings.claude_api_key = Some(encrypted_key),
    }
    
    // Clear provider-specific data only when switching
    if switching_providers {
        settings.github_model = None;
        settings.cached_models = None;
        settings.models_cache_timestamp = None;
    }
    
    save_settings_to_disk(&app, &settings)?;
    Ok(())
}

/// Send a message to the AI assistant and get a response.
///
/// Assembles the system prompt from the knowledge base, screen context, and state
/// metadata, then calls the appropriate provider's API based on saved settings.
#[tauri::command]
pub async fn assistant_chat(
    message: String,
    screen_context: String,
    state_metadata: String,
    history: Vec<ChatMessage>,
    app: AppHandle,
) -> Result<String, String> {
    let settings = load_settings(&app)?;

    let encrypted_key = match settings.active_provider {
        LlmProvider::GithubModels => settings.github_api_key,
        LlmProvider::Openai => settings.openai_api_key,
        LlmProvider::Claude => settings.claude_api_key,
    }.ok_or("Assistant not configured. Please connect your API key first.")?;
    
    // Decrypt the API key
    let enc_key = get_or_create_encryption_key(&app)?;
    let api_key = decrypt_key(&encrypted_key, &enc_key)?;

    let system_prompt = build_system_prompt(&screen_context, &state_metadata);
    let client = http_client(60)?;

    match settings.active_provider {
        LlmProvider::GithubModels => {
            let model = settings.github_model.as_deref().unwrap_or("openai/gpt-4o-mini");
            call_openai_compatible(
                "https://models.github.ai/inference/chat/completions",
                &api_key,
                model,
                &system_prompt,
                &message,
                &history,
                &client,
                "GitHub Models",
            ).await
        }
        LlmProvider::Openai => {
            call_openai_compatible(
                "https://api.openai.com/v1/chat/completions",
                &api_key,
                "gpt-4o-mini",
                &system_prompt,
                &message,
                &history,
                &client,
                "OpenAI",
            ).await
        }
        LlmProvider::Claude => {
            call_claude(
                &api_key,
                &system_prompt,
                &message,
                &history,
                &client,
            ).await
        }
    }
}

/// Load saved assistant settings.
/// Returns settings with encrypted keys stripped and has_* booleans computed.
#[tauri::command]
pub fn assistant_get_settings(app: AppHandle) -> Result<SettingsResponse, String> {
    let mut settings = load_settings(&app)?;
    
    // Compute has_* booleans
    let has_github_key = settings.github_api_key.is_some();
    let has_openai_key = settings.openai_api_key.is_some();
    let has_claude_key = settings.claude_api_key.is_some();
    
    // Strip encrypted keys before sending to frontend
    settings.github_api_key = None;
    settings.openai_api_key = None;
    settings.claude_api_key = None;
    
    Ok(SettingsResponse {
        settings,
        has_github_key,
        has_openai_key,
        has_claude_key,
    })
}

/// Switch to a different provider without deleting keys.
#[tauri::command]
pub fn assistant_switch_provider(app: AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app)?;
    settings.configured = false;
    settings.chat_history = None; // Clear chat history when switching
    save_settings_to_disk(&app, &settings)
}

/// Reconnect to a provider using an already-saved API key.
#[tauri::command]
pub fn assistant_reconnect(provider: String, app: AppHandle) -> Result<(), String> {
    let provider_enum: LlmProvider = serde_json::from_str(&format!("\"{}\"", provider))
        .map_err(|_| format!("Unknown provider: {}", provider))?;
    
    let mut settings = load_settings(&app)?;
    
    // Verify key exists for this provider
    let has_key = match provider_enum {
        LlmProvider::GithubModels => settings.github_api_key.is_some(),
        LlmProvider::Openai => settings.openai_api_key.is_some(),
        LlmProvider::Claude => settings.claude_api_key.is_some(),
    };
    
    if !has_key {
        return Err("No saved key for this provider.".to_string());
    }
    
    settings.active_provider = provider_enum;
    settings.configured = true;
    save_settings_to_disk(&app, &settings)
}

/// Delete the API key for a specific provider.
#[tauri::command]
pub fn assistant_delete_provider_key(provider: String, app: AppHandle) -> Result<(), String> {
    let provider_enum: LlmProvider = serde_json::from_str(&format!("\"{}\"", provider))
        .map_err(|_| format!("Unknown provider: {}", provider))?;
    
    let mut settings = load_settings(&app)?;
    
    match provider_enum {
        LlmProvider::GithubModels => {
            settings.github_api_key = None;
            settings.github_model = None;
            settings.cached_models = None;
            settings.models_cache_timestamp = None;
        },
        LlmProvider::Openai => settings.openai_api_key = None,
        LlmProvider::Claude => settings.claude_api_key = None,
    }
    
    // If deleting active provider, mark as unconfigured
    if settings.active_provider == provider_enum {
        settings.configured = false;
    }
    
    save_settings_to_disk(&app, &settings)
}

/// Delete all API keys and reset settings.
#[tauri::command]
pub fn assistant_delete_all_keys(app: AppHandle) -> Result<(), String> {
    let settings = AssistantSettings::default();
    save_settings_to_disk(&app, &settings)
}

/// Get available GitHub Models (fetches from API, caches for 24 hours).
#[tauri::command]
pub async fn assistant_get_available_models(app: AppHandle) -> Result<Vec<(String, String)>, String> {
    let mut settings = load_settings(&app)?;
    
    // Check if cache is valid (exists and not expired)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let cache_valid = settings.cached_models.is_some() 
        && settings.models_cache_timestamp
            .map(|ts| now - ts < MODELS_CACHE_DURATION_SECS)
            .unwrap_or(false);
    
    if cache_valid {
        return Ok(settings.cached_models.unwrap());
    }
    
    // Fetch from API
    let encrypted_token = settings.github_api_key.as_ref()
        .ok_or("No GitHub API key available")?;
    
    // Decrypt the token
    let enc_key = get_or_create_encryption_key(&app)?;
    let token = decrypt_key(encrypted_token, &enc_key)?;
    
    let client = http_client(15)?;
    let response = client
        .get("https://models.github.ai/catalog/models")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models catalog: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch models catalog ({}): {}", status, body));
    }
    
    let models: Vec<CatalogModel> = response.json().await
        .map_err(|e| format!("Failed to parse models catalog: {}", e))?;
    
    // Convert to (id, display_name) tuples
    let model_list: Vec<(String, String)> = models
        .into_iter()
        .map(|m| {
            let display = if let Some(pub_name) = m.publisher {
                format!("{} ({})", m.name, pub_name)
            } else {
                m.name
            };
            (m.id, display)
        })
        .collect();
    
    // Cache in settings
    settings.cached_models = Some(model_list.clone());
    settings.models_cache_timestamp = Some(now);
    save_settings_to_disk(&app, &settings)?;
    
    Ok(model_list)
}

/// Update the selected GitHub Model.
#[tauri::command]
pub fn assistant_update_model(model: String, app: AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app)?;
    settings.github_model = Some(model);
    save_settings_to_disk(&app, &settings)
}

/// Save chat history to disk.
#[tauri::command]
pub fn assistant_save_history(messages: Vec<ChatMessage>, app: AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app)?;
    settings.chat_history = Some(messages);
    save_settings_to_disk(&app, &settings)
}

/// Clear chat history from disk.
#[tauri::command]
pub fn assistant_clear_history(app: AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app)?;
    settings.chat_history = None;
    save_settings_to_disk(&app, &settings)
}
