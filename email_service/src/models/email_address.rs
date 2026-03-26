use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use once_cell::sync::Lazy;

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    ).expect("Invalid email regex")
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EmailAddress {
    address: String,
    name: Option<String>,
}

impl EmailAddress {
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

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn is_valid(email: &str) -> bool {
        EMAIL_REGEX.is_match(email)
    }

    pub fn domain(&self) -> Option<&str> {
        self.address.split('@').nth(1)
    }

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
