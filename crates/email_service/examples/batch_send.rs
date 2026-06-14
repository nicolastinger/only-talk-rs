use email_service::prelude::*;
use email_service::{EmailManager, ProviderConfig, SmtpConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Email Service 批量发送示例 ===\n");

    let manager = EmailManager::builder()
        .default_provider("smtp")
        .provider(
            "smtp",
            ProviderConfig::Smtp(SmtpConfig {
                enabled: true,
                priority: 100,
                host: "smtp.example.com".to_string(),
                port: 587,
                username: "your_username".to_string(),
                password: "your_password".to_string(),
                from_email: "noreply@example.com".to_string(),
                from_alias: Some("批量通知".to_string()),
                use_tls: true,
                use_starttls: true,
                ..Default::default()
            }),
        )
        .build()?;

    let recipients = vec![
        ("user1@example.com", "用户1"),
        ("user2@example.com", "用户2"),
        ("user3@example.com", "用户3"),
    ];

    let mut emails = Vec::new();

    for (email_addr, name) in recipients {
        let email = Email::builder()
            .from(EmailAddress::new("noreply@example.com")?)
            .to(EmailAddress::with_name(email_addr, name)?)
            .subject("批量测试邮件")
            .html_body(format!(
                r#"
                <html>
                    <body>
                        <h1>你好, {}!</h1>
                        <p>这是一封批量发送的测试邮件。</p>
                    </body>
                </html>
            "#,
                name
            ))
            .tag("batch", "true")
            .build()?;

        emails.push(email);
    }

    println!("准备发送 {} 封邮件...\n", emails.len());

    let results = manager.send_batch(&emails).await;

    let mut success_count = 0;
    let mut fail_count = 0;

    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(send_result) => {
                success_count += 1;
                println!("邮件 {} 发送成功 - 服务商: {}", i + 1, send_result.provider);
            }
            Err(e) => {
                fail_count += 1;
                println!("邮件 {} 发送失败: {}", i + 1, e);
            }
        }
    }

    println!("\n发送统计:");
    println!("  成功: {}", success_count);
    println!("  失败: {}", fail_count);
    println!("  总计: {}", emails.len());

    Ok(())
}
