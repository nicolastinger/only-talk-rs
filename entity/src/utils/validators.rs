use once_cell::sync::Lazy;
use regex::Regex;

pub static PASSWORD_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z\d]{14,}$").expect("构建正则表达式失败"));
