//! Email address definition
//!
//! This module defines the [`EmailAddress`] struct for representing email addresses.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Email address regex pattern
///
/// Uses `once_cell::Lazy` for lazy initialization to avoid recompilation.
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .unwrap_or_else(|e| panic!("email regex compilation failed: {}", e))
});

/// Email address
///
/// Represents an email address, optionally with a display name.
///
/// # Format
///
/// - Plain address: `user@example.com`
/// - With name: `Name <user@example.com>`
///
/// # Validation
///
/// Format is validated on creation; invalid addresses return an error.
///
/// # 示例
///
/// ```rust
/// use email_service::models::EmailAddress;
///
/// // 纯地址
/// let addr = EmailAddress::new("user@example.com")?;
///
/// // 带显示名称
/// let addr = EmailAddress::with_name("user@example.com", "张三")?;
///
/// // 获取地址和名称
/// assert_eq!(addr.address(), "user@example.com");
/// assert_eq!(addr.name(), Some("张三"));
///
/// // 显示格式
/// assert_eq!(addr.to_string(), "张三 <user@example.com>");
/// # Ok::<(), email_service::error::EmailError>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EmailAddress {
    /// 邮箱地址
    address: String,

    /// 显示名称（可选）
    name: Option<String>,
}

impl EmailAddress {
    /// Create email address
    ///
    /// # Arguments
    ///
    /// - `address`: Email address string
    ///
    /// # Errors
    ///
    /// Returns [`EmailError::InvalidEmailAddress`] if the address format is invalid.
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::EmailAddress;
    ///
    /// let addr = EmailAddress::new("user@example.com")?;
    /// assert_eq!(addr.address(), "user@example.com");
    /// assert_eq!(addr.name(), None);
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn new(address: impl Into<String>) -> crate::error::EmailResult<Self> {
        let address = address.into();
        if !Self::is_valid(&address) {
            return Err(crate::error::EmailError::InvalidEmailAddress(address));
        }
        Ok(Self { address, name: None })
    }

    /// Create email address with display name
    ///
    /// # Arguments
    ///
    /// - `address`: Email address string
    /// - `name`: Display name
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::EmailAddress;
    ///
    /// let addr = EmailAddress::with_name("user@example.com", "张三")?;
    /// assert_eq!(addr.to_string(), "张三 <user@example.com>");
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn with_name(
        address: impl Into<String>,
        name: impl Into<String>,
    ) -> crate::error::EmailResult<Self> {
        let address = address.into();
        if !Self::is_valid(&address) {
            return Err(crate::error::EmailError::InvalidEmailAddress(address));
        }
        Ok(Self { address, name: Some(name.into()) })
    }

    /// Get email address
    ///
    /// Returns the plain email address without display name.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get display name
    ///
    /// Returns `Some(name)` if set, otherwise `None`.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Validate email address format
    ///
    /// Uses regex to verify whether the email format is valid.
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::EmailAddress;
    ///
    /// assert!(EmailAddress::is_valid("user@example.com"));
    /// assert!(EmailAddress::is_valid("user.name@example.co.uk"));
    /// assert!(!EmailAddress::is_valid("invalid-email"));
    /// assert!(!EmailAddress::is_valid("@example.com"));
    /// ```
    pub fn is_valid(email: &str) -> bool {
        EMAIL_REGEX.is_match(email)
    }

    /// Get email domain
    ///
    /// Returns the domain part after the `@` symbol.
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::EmailAddress;
    ///
    /// let addr = EmailAddress::new("user@example.com")?;
    /// assert_eq!(addr.domain(), Some("example.com"));
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn domain(&self) -> Option<&str> {
        self.address.split('@').nth(1)
    }

    /// Get email local part
    ///
    /// Returns the username part before the `@` symbol.
    ///
    /// # 示例
    ///
    /// ```rust
    /// use email_service::models::EmailAddress;
    ///
    /// let addr = EmailAddress::new("user@example.com")?;
    /// assert_eq!(addr.local_part(), Some("user"));
    /// # Ok::<(), email_service::error::EmailError>(())
    /// ```
    pub fn local_part(&self) -> Option<&str> {
        self.address.split('@').next()
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{} <{}>", name, self.address),
            None => write!(f, "{}", self.address),
        }
    }
}

impl From<EmailAddress> for String {
    fn from(addr: EmailAddress) -> String {
        addr.to_string()
    }
}

impl TryFrom<&str> for EmailAddress {
    type Error = crate::error::EmailError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        EmailAddress::new(value)
    }
}

impl TryFrom<String> for EmailAddress {
    type Error = crate::error::EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        EmailAddress::new(value)
    }
}
