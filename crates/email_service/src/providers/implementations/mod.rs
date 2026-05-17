mod aliyun;
mod tencent;
mod aws_ses;
mod smtp;

pub use aliyun::AliyunEmailProvider;
pub use tencent::TencentEmailProvider;
pub use aws_ses::AwsSesEmailProvider;
pub use smtp::SmtpEmailProvider;
