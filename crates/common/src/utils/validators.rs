use once_cell::sync::Lazy;
use regex::Regex;

use crate::utils::fatal_error::fatal_panic;

pub static PASSWORD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z\d]{14,}$")
        .unwrap_or_else(|e| fatal_panic(&format!("构建密码正则失败: {}", e)))
});

pub static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .unwrap_or_else(|e| fatal_panic(&format!("构建邮箱正则失败: {}", e)))
});
