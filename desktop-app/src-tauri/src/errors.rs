//! Standardized error message helpers.
//!
//! Centralises user-facing error strings so that wording stays consistent
//! across all cloud providers and CLI interactions.

/// CLI tool not found on the system.
pub fn cli_not_found(cli_name: &str) -> String {
    format!("{} not found. Please install it first.", cli_name)
}

/// Authentication session expired and needs renewal.
pub fn auth_expired(provider: &str) -> String {
    format!("{} session expired. Please login again.", provider)
}

/// User is not logged in to the given provider.
pub fn not_logged_in(provider: &str) -> String {
    format!("Not logged in to {}. Please login first.", provider)
}
