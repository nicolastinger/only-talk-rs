use email_service::prelude::*;
use email_service::{EmailManager, ProviderConfig, AliyunConfig, Attachment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Email Service 附件发送示例 ===\n");

    let manager = EmailManager::builder()
        .provider("aliyun", ProviderConfig::Aliyun(AliyunConfig {
            enabled: true,
            priority: 100,
            access_key_id: "your_access_key_id".to_string(),
            access_key_secret: "your_access_key_secret".to_string(),
            region_id: "cn-hangzhou".to_string(),
            account_name: "noreply@yourdomain.com".to_string(),
            ..Default::default()
        }))
        .build()?;

    let text_content = "这是一个文本附件的内容。\n第二行内容。";
    let text_attachment = Attachment::new(
        "report.txt",
        text_content.as_bytes().to_vec(),
        "text/plain"
    );

    let html_content = r#"<html>
        <body>
            <h1>HTML报告</h1>
            <p>这是一个HTML附件。</p>
        </body>
    </html>"#;
    let html_attachment = Attachment::new(
        "report.html",
        html_content.as_bytes().to_vec(),
        "text/html"
    );

    let json_data = serde_json::json!({
        "name": "测试数据",
        "items": [1, 2, 3],
        "timestamp": "2024-01-01T00:00:00Z"
    });
    let json_attachment = Attachment::new(
        "data.json",
        serde_json::to_string_pretty(&json_data)?.into_bytes(),
        "application/json"
    );

    let auto_attachment = Attachment::from_bytes(
        "auto_detect.bin",
        vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
    );
    println!("自动检测的MIME类型: {}", auto_attachment.content_type);

    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com")?)
        .to(EmailAddress::new("recipient@example.com")?)
        .subject("带附件的测试邮件")
        .text_body("请查收附件中的报告文件。")
        .html_body(r#"
            <html>
                <body>
                    <h1>附件邮件</h1>
                    <p>请查收附件中的报告文件。</p>
                    <ul>
                        <li>report.txt - 文本报告</li>
                        <li>report.html - HTML报告</li>
                        <li>data.json - JSON数据</li>
                    </ul>
                </body>
            </html>
        "#)
        .with_attachment(text_attachment)
        .with_attachment(html_attachment)
        .with_attachment(json_attachment)
        .with_attachment(auto_attachment)
        .build()?;

    println!("邮件信息:");
    println!("  附件数量: {}", email.attachments.len());
    for attachment in &email.attachments {
        println!("  - {} ({} bytes, {})", 
            attachment.filename, 
            attachment.size(),
            attachment.content_type
        );
    }
    println!("  总附件大小: {} bytes", email.total_attachment_size());

    println!("\n发送邮件...");
    match manager.send(&email).await {
        Ok(result) => {
            println!("发送成功!");
            println!("  消息ID: {:?}", result.message_id);
        }
        Err(e) => {
            println!("发送失败: {}", e);
        }
    }

    Ok(())
}
