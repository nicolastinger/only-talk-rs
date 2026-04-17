//! 邮箱地址定义
//!
//! 本模块定义了 [`EmailAddress`] 结构体，用于表示邮件地址。

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use once_cell::sync::Lazy;

/// 邮箱地址正则表达式
///
/// 使用 `once_cell::Lazy` 延迟初始化，避免重复编译。
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    ).expect("Invalid email regex")
});

/// 邮箱地址
///
/// 表示一个邮箱地址，可选包含显示名称。
///
/// # 格式
///
/// - 纯地址: `user@example.com`
/// - 带名称: `张三 <user@example.com>`
///
/// # 验证
///
/// 创建时会自动验证邮箱格式，不符合规范会返回错误。
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
    /// 创建邮箱地址
    ///
    /// # 参数
    ///
    /// - `address`: 邮箱地址字符串
    ///
    /// # 错误
    ///
    /// 如果地址格式无效，返回 [`EmailError::InvalidEmailAddress`]。
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
        Ok(Self {
            address,
            name: None,
        })
    }

    /// 创建带名称的邮箱地址
    ///
    /// # 参数
    ///
    /// - `address`: 邮箱地址字符串
    /// - `name`: 显示名称
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
    pub fn with_name(address: impl Into<String>, name: impl Into<String>) -> crate::error::EmailResult<Self> {
        let address = address.into();
        if !Self::is_valid(&address) {
            return Err(crate::error::EmailError::InvalidEmailAddress(address));
        }
        Ok(Self {
            address,
            name: Some(name.into()),
        })
    }

    /// 获取邮箱地址
    ///
    /// 返回纯邮箱地址，不包含显示名称。
    pub fn address(&self) -> &str {
        &self.address
    }

    /// 获取显示名称
    ///
    /// 如果设置了名称，返回 `Some(name)`，否则返回 `None`。
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 验证邮箱地址格式
    ///
    /// 使用正则表达式验证邮箱格式是否正确。
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

    /// 获取邮箱域名
    ///
    /// 返回 `@` 符号后面的域名部分。
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

    /// 获取邮箱本地部分
    ///
    /// 返回 `@` 符号前面的用户名部分。
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
