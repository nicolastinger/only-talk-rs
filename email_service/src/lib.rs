pub mod error;
pub mod config;
pub mod models;
pub mod providers;
pub mod manager;
pub mod pool;

pub use error::{EmailError, EmailResult};
pub use config::{
    EmailServiceConfig, RetryConfig, PoolConfig, RateLimitConfig,
    ProviderConfig, AliyunConfig, TencentConfig, AwsSesConfig, SmtpConfig,
};
pub use models::{
    Email, EmailAddress, EmailBuilder, EmailPriority,
    Attachment, ContentDisposition,
    SendResult, SendStatus, RecipientResult, ErrorInfo, ErrorCategory,
};
pub use providers::{
    EmailProvider, BoxedEmailProvider,
    RetryStrategy, ExponentialBackoff, FixedInterval, NoRetry, AdaptiveRetry, CircuitBreaker,
    AliyunEmailProvider, TencentEmailProvider, AwsSesEmailProvider, SmtpEmailProvider,
};
pub use manager::{
    EmailManager, EmailManagerBuilder,
    ProviderPool, ProviderSelector, SelectionStrategy,
};

pub mod prelude {
    pub use crate::error::{EmailError, EmailResult};
    pub use crate::config::{
        EmailServiceConfig, RetryConfig, PoolConfig,
        ProviderConfig, AliyunConfig, TencentConfig, AwsSesConfig, SmtpConfig,
    };
    pub use crate::models::{
        Email, EmailAddress, EmailBuilder, EmailPriority,
        Attachment, SendResult, SendStatus,
    };
    pub use crate::providers::EmailProvider;
    pub use crate::manager::EmailManager;
}