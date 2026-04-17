use email_service::prelude::*;
use email_service::{ProviderConfig, AliyunConfig, TencentConfig, RetryConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Email Service 基础示例 ===\n");

    let manager = EmailManager::builder()
        .default_provider("aliyun")
        .provider("aliyun", ProviderConfig::Aliyun(AliyunConfig {
            enabled: true,
            priority: 100,
            access_key_id: "xxx".to_string(),
            access_key_secret: "xxx".to_string(),
            region_id: "cn-hangzhou".to_string(),
            account_name: "xxxx".to_string(),
            from_alias: Some("系统通知".to_string()),
            ..Default::default()
        }))
        // .provider("tencent", ProviderConfig::Tencent(TencentConfig {
        //     enabled: true,
        //     priority: 90,
        //     secret_id: "your_secret_id".to_string(),
        //     secret_key: "your_secret_key".to_string(),
        //     region: "ap-guangzhou".to_string(),
        //     sender: "noreply@yourdomain.com".to_string(),
        //     from_alias: Some("系统通知".to_string()),
        //     ..Default::default()
        // }))
        .retry_config(RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            multiplier: 2.0,
            jitter: true,
        })
        .build()?;

    println!("已注册的服务商: {:?}", manager.list_providers());

    let email = Email::builder()
        .from(EmailAddress::new("xxxx")?)
        .to(EmailAddress::with_name("904934103@qq.com", "收件人")?)
        .cc(EmailAddress::new("2737484812@qq.com")?)
        .subject("测试邮件 - Email Service")
        .text_body("这是一封测试邮件的纯文本内容。")
        .html_body(r#"
            <html>
                <body>
                    <h1>测试邮件</h1>
                    <p>这是一封测试邮件的HTML内容。</p>
                </body>
            </html>
        "#)
        .tag("type", "test")
        .tag("env", "development")
        .build()?;

    println!("\n邮件信息:");
    println!("  ID: {}", email.id);
    println!("  发件人: {}", email.from);
    println!("  收件人: {:?}", email.to.iter().map(|a| a.to_string()).collect::<Vec<_>>());
    println!("  主题: {}", email.subject);

    println!("\n尝试发送邮件...");
    match manager.send(&email).await {
        Ok(result) => {
            if result.is_success() {
                println!("发送成功!");
                println!("  服务商: {}", result.provider);
                println!("  消息ID: {:?}", result.message_id);
                println!("  状态: {}", result.status);
            } else {
                println!("发送失败!");
                println!("  服务商: {}", result.provider);
                println!("  状态: {}", result.status);
                if let Some(error) = &result.error {
                    println!("  错误代码: {}", error.code);
                    println!("  错误信息: {}", error.message);
                    println!("  错误类别: {}", error.category);
                    println!("  可重试: {}", error.retryable);
                }
            }
        }
        Err(e) => {
            println!("发送异常: {}", e);
        }
    }

    println!("\n尝试带故障转移发送...");
    match manager.send_with_fallback(&email).await {
        Ok(result) => {
            println!("发送成功!");
            println!("  服务商: {}", result.provider);
        }
        Err(e) => {
            println!("所有服务商都失败: {}", e);
        }
    }

    println!("\n健康检查...");
    let health = manager.health_check().await;
    for (provider, is_healthy) in &health {
        println!("  {}: {}", provider, if *is_healthy { "健康" } else { "不健康" });
    }

    Ok(())
}
