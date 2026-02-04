/// Standardized error message helpers
/// Only includes functions that are actually used in the codebase

/// CLI not found errors
pub fn cli_not_found(cli_name: &str) -> String {
    format!("{} not found. Please install it first.", cli_name)
}

/// Authentication session expired
pub fn auth_expired(provider: &str) -> String {
    format!("{} session expired. Please login again.", provider)
}

/// Not logged in to a provider
pub fn not_logged_in(provider: &str) -> String {
    format!("Not logged in to {}. Please login first.", provider)
}
