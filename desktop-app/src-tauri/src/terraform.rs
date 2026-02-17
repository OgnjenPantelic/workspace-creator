use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformVariable {
    pub name: String,
    pub description: String,
    pub var_type: String,
    pub default: Option<String>,
    pub required: bool,
    pub sensitive: bool,
    pub validation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStatus {
    pub running: bool,
    pub command: Option<String>,
    pub output: String,
    pub success: Option<bool>,
    pub can_rollback: bool,
}

impl Default for DeploymentStatus {
    fn default() -> Self {
        Self {
            running: false,
            command: None,
            output: String::new(),
            success: None,
            can_rollback: false,
        }
    }
}

lazy_static::lazy_static! {
    pub static ref DEPLOYMENT_STATUS: Arc<Mutex<DeploymentStatus>> = Arc::new(Mutex::new(DeploymentStatus::default()));
    pub static ref CURRENT_PROCESS: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
}

pub fn parse_variables_tf(content: &str) -> Vec<TerraformVariable> {
    let mut variables = Vec::new();
    let mut current_var: Option<TerraformVariable> = None;
    let mut in_variable_block = false;
    let mut brace_count = 0;
    let mut current_description = String::new();
    let mut current_type = String::from("string");
    let mut current_default: Option<String> = None;
    let mut is_sensitive = false;
    let mut current_validation: Option<String> = None;
    
    // Track multiline default value parsing
    let mut in_multiline_default = false;
    let mut default_brace_count = 0;
    let mut default_bracket_count = 0;
    let mut multiline_default_buffer = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Start of variable block
        if !in_variable_block && trimmed.starts_with("variable ") && trimmed.contains('{') {
            in_variable_block = true;
            brace_count = 1;
            
            // Extract variable name
            if let Some(name_start) = trimmed.find('"') {
                if let Some(name_end) = trimmed[name_start + 1..].find('"') {
                    let name = &trimmed[name_start + 1..name_start + 1 + name_end];
                    current_var = Some(TerraformVariable {
                        name: name.to_string(),
                        description: String::new(),
                        var_type: "string".to_string(),
                        default: None,
                        required: true,
                        sensitive: false,
                        validation: None,
                    });
                }
            }
            current_description.clear();
            current_type = String::from("string");
            current_default = None;
            is_sensitive = false;
            current_validation = None;
            in_multiline_default = false;
            default_brace_count = 0;
            default_bracket_count = 0;
            multiline_default_buffer.clear();
            continue;
        }

        if in_variable_block {
            // Parse multiline default values (maps/lists) by tracking brace/bracket balance
            if in_multiline_default {
                multiline_default_buffer.push_str(trimmed);
                multiline_default_buffer.push(' ');
                
                default_brace_count += trimmed.matches('{').count() as i32;
                default_brace_count -= trimmed.matches('}').count() as i32;
                default_bracket_count += trimmed.matches('[').count() as i32;
                default_bracket_count -= trimmed.matches(']').count() as i32;
                
                // Check if multiline default is complete
                if default_brace_count <= 0 && default_bracket_count <= 0 {
                    in_multiline_default = false;
                    // For complex defaults (maps/lists), just mark as having a default
                    // We don't need to parse the actual value for the UI
                    current_default = Some(multiline_default_buffer.trim().to_string());
                }
                
                // Still count braces for the variable block
                brace_count += trimmed.matches('{').count() as i32;
                brace_count -= trimmed.matches('}').count() as i32;
            } else {
                // Count braces for variable block
                brace_count += trimmed.matches('{').count() as i32;
                brace_count -= trimmed.matches('}').count() as i32;

                // Parse attributes (only at brace_count == 1, i.e., top level of variable)
                if brace_count >= 1 {
                    if trimmed.starts_with("description") {
                        if let Some(val) = extract_string_value(trimmed) {
                            current_description = val;
                        }
                    } else if trimmed.starts_with("type") {
                        if let Some(val) = extract_type_value(trimmed) {
                            current_type = val;
                        }
                    } else if trimmed.starts_with("default") {
                        // Check if this is a multiline default
                        let after_eq = trimmed.split_once('=').map(|(_, v)| v.trim()).unwrap_or("");
                        
                        if after_eq.starts_with('{') || after_eq.starts_with('[') {
                            // Count opening/closing braces/brackets on this line
                            let open_braces = after_eq.matches('{').count() as i32;
                            let close_braces = after_eq.matches('}').count() as i32;
                            let open_brackets = after_eq.matches('[').count() as i32;
                            let close_brackets = after_eq.matches(']').count() as i32;
                            
                            if open_braces > close_braces || open_brackets > close_brackets {
                                // Multiline default starts here
                                in_multiline_default = true;
                                default_brace_count = open_braces - close_braces;
                                default_bracket_count = open_brackets - close_brackets;
                                multiline_default_buffer = after_eq.to_string();
                                multiline_default_buffer.push(' ');
                            } else {
                                // Single-line complex default
                                current_default = Some(after_eq.to_string());
                            }
                        } else {
                            // Simple default value
                            current_default = extract_default_value(trimmed);
                        }
                    } else if trimmed.starts_with("sensitive") && trimmed.contains("true") {
                        is_sensitive = true;
                    } else if trimmed.starts_with("condition") {
                        if let Some(val) = extract_string_value(line) {
                            current_validation = Some(val);
                        }
                    }
                }
            }

            // End of variable block
            if brace_count == 0 && !in_multiline_default {
                if let Some(mut var) = current_var.take() {
                    var.description = current_description.clone();
                    var.var_type = current_type.clone();
                    var.default = current_default.clone();
                    var.required = current_default.is_none();
                    var.sensitive = is_sensitive;
                    var.validation = current_validation.clone();
                    variables.push(var);
                }
                in_variable_block = false;
            }
        }
    }

    variables
}

fn extract_string_value(line: &str) -> Option<String> {
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].rfind('"') {
            return Some(line[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

fn extract_type_value(line: &str) -> Option<String> {
    let line = line.trim();
    if let Some(idx) = line.find('=') {
        let type_part = line[idx + 1..].trim();
        return Some(type_part.to_string());
    }
    None
}

fn extract_default_value(line: &str) -> Option<String> {
    let line = line.trim();
    if let Some(idx) = line.find('=') {
        let value_part = line[idx + 1..].trim();
        // Handle quoted strings
        if value_part.starts_with('"') && value_part.ends_with('"') {
            return Some(value_part[1..value_part.len() - 1].to_string());
        }
        // Handle other values
        if !value_part.is_empty() && value_part != "{" && value_part != "[" {
            return Some(value_part.to_string());
        }
    }
    None
}

pub fn generate_tfvars(values: &HashMap<String, serde_json::Value>, variables: &[TerraformVariable]) -> String {
    let mut lines = Vec::new();
    
    for var in variables {
        if let Some(value) = values.get(&var.name) {
            // Skip empty strings for required variables (no default)
            if let serde_json::Value::String(s) = value {
                if s.trim().is_empty() && var.default.is_none() {
                    continue;
                }
            }
            
            let var_type = var.var_type.to_lowercase();
            
            let formatted = match value {
                serde_json::Value::String(s) => {
                    // Check if the variable type is map or list and try to parse the string
                    if var_type.starts_with("map") || var_type.contains("map(") {
                        // Try to parse as JSON object, otherwise output as empty map
                        if let Ok(obj) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(s) {
                            format_map(&var.name, &obj)
                        } else if s.trim().is_empty() || s.trim() == "{}" {
                            format!("{} = {{}}", var.name)
                        } else {
                            format!("{} = \"{}\"", var.name, s)
                        }
                    } else if var_type.starts_with("list") || var_type.contains("list(") {
                        // Try to parse as JSON array, otherwise output as empty list
                        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(s) {
                            format_list(&var.name, &arr)
                        } else if s.trim().is_empty() || s.trim() == "[]" {
                            format!("{} = []", var.name)
                        } else {
                            format!("{} = \"{}\"", var.name, s)
                        }
                    } else if var_type == "bool" {
                        // Handle boolean strings - output without quotes
                        let bool_val = s.to_lowercase();
                        if bool_val == "true" || bool_val == "false" {
                            format!("{} = {}", var.name, bool_val)
                        } else {
                            format!("{} = \"{}\"", var.name, s)
                        }
                    } else {
                        format!("{} = \"{}\"", var.name, s)
                    }
                }
                serde_json::Value::Bool(b) => format!("{} = {}", var.name, b),
                serde_json::Value::Number(n) => format!("{} = {}", var.name, n),
                serde_json::Value::Array(arr) => format_list(&var.name, arr),
                serde_json::Value::Object(obj) => format_map(&var.name, obj),
                _ => continue,
            };
            lines.push(formatted);
        }
    }
    
    lines.join("\n")
}

fn format_list(name: &str, arr: &[serde_json::Value]) -> String {
    // Check if list contains objects (for list(object(...)) types)
    let has_objects = arr.iter().any(|v| matches!(v, serde_json::Value::Object(_)));
    
    if has_objects {
        // Format as list of objects with proper HCL syntax
        let items: Vec<String> = arr.iter()
            .filter_map(|v| {
                if let serde_json::Value::Object(obj) = v {
                    let fields: Vec<String> = obj.iter()
                        .filter_map(|(k, v)| {
                            match v {
                                serde_json::Value::String(s) => Some(format!("    {} = \"{}\"", k, s)),
                                serde_json::Value::Number(n) => Some(format!("    {} = {}", k, n)),
                                serde_json::Value::Bool(b) => Some(format!("    {} = {}", k, b)),
                                _ => None,
                            }
                        })
                        .collect();
                    Some(format!("  {{\n{}\n  }}", fields.join("\n")))
                } else {
                    None
                }
            })
            .collect();
        format!("{} = [\n{}\n]", name, items.join(",\n"))
    } else {
        // Simple list of primitives
        let items: Vec<String> = arr.iter()
            .map(|v| match v {
                serde_json::Value::String(s) => format!("\"{}\"", s),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => "null".to_string(),
            })
            .collect();
        format!("{} = [{}]", name, items.join(", "))
    }
}

fn format_map(name: &str, obj: &serde_json::Map<String, serde_json::Value>) -> String {
    if obj.is_empty() {
        return format!("{} = {{}}", name);
    }
    let mut obj_lines = vec![format!("{} = {{", name)];
    for (k, v) in obj {
        match v {
            serde_json::Value::String(s) => obj_lines.push(format!("  \"{}\" = \"{}\"", k, s)),
            serde_json::Value::Number(n) => obj_lines.push(format!("  \"{}\" = {}", k, n)),
            serde_json::Value::Bool(b) => obj_lines.push(format!("  \"{}\" = {}", k, b)),
            _ => {}
        }
    }
    obj_lines.push("}".to_string());
    obj_lines.join("\n")
}

pub fn run_terraform(
    command: &str,
    working_dir: &PathBuf,
    env_vars: HashMap<String, String>,
) -> Result<Child, String> {
    let terraform_path = get_terraform_path();
    
    let args: Vec<&str> = match command {
        "init" => vec!["init", "-no-color"],
        "plan" => vec!["plan", "-no-color"],
        "apply" => vec!["apply", "-auto-approve", "-no-color"],
        "destroy" => vec!["destroy", "-auto-approve", "-no-color"],
        _ => return Err(format!("Unknown command: {}", command)),
    };

    let mut cmd = Command::new(&terraform_path);
    cmd.args(&args)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Extend PATH to include common installation locations (macOS GUI apps have minimal PATH)
    let install_dir = crate::dependencies::get_terraform_install_path();
    let current_path = std::env::var("PATH").unwrap_or_default();
    
    #[cfg(target_os = "windows")]
    let extended_path = format!(
        "{};{}",
        install_dir.to_string_lossy(),
        current_path
    );
    
    #[cfg(not(target_os = "windows"))]
    let extended_path = format!(
        "{}:/usr/local/bin:/opt/homebrew/bin:/opt/local/bin:{}",
        install_dir.to_string_lossy(),
        current_path
    );
    
    cmd.env("PATH", extended_path);

    cmd.spawn().map_err(|e| e.to_string())
}

fn get_terraform_path() -> String {
    // Reuse the path finding logic from dependencies module
    crate::dependencies::find_terraform_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "terraform".to_string())
}

pub fn check_state_exists(working_dir: &PathBuf) -> bool {
    let state_file = working_dir.join("terraform.tfstate");
    if state_file.exists() {
        if let Ok(content) = fs::read_to_string(&state_file) {
            // Check if state has resources
            return content.contains("\"resources\"") && content.contains("\"type\"");
        }
    }
    false
}

