use once_cell::sync::Lazy;
use regex::Regex;
use crate::utils::fatal_error::fatal_panic;

pub static PASSWORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z\d]{14,}$").unwrap_or_else(|e| fatal_panic(&format!("Failed to build password regex: {}", e)))
});
