mod aliyun;
mod aws_ses;
mod smtp;
mod tencent;

pub use aliyun::AliyunEmailProvider;
pub use aws_ses::AwsSesEmailProvider;
pub use smtp::SmtpEmailProvider;
pub use tencent::TencentEmailProvider;
