use once_cell::sync::Lazy;
use regex::Regex;

pub static PASSWORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z\d]{14,}$").unwrap()
});