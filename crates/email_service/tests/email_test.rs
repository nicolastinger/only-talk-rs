use email_service::{Email, EmailAddress, EmailPriority, Attachment};

#[test]
fn test_email_builder_basic() {
    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("Test Subject")
        .text_body("Test body")
        .build();

    assert!(email.is_ok());
    let email = email.unwrap();
    
    assert!(!email.id.is_empty());
    assert_eq!(email.subject, "Test Subject");
    assert_eq!(email.text_body, Some("Test body".to_string()));
    assert_eq!(email.to.len(), 1);
}

#[test]
fn test_email_builder_with_multiple_recipients() {
    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient1@example.com").unwrap())
        .to(EmailAddress::new("recipient2@example.com").unwrap())
        .cc(EmailAddress::new("cc@example.com").unwrap())
        .bcc(EmailAddress::new("bcc@example.com").unwrap())
        .subject("Test")
        .text_body("Body")
        .build()
        .unwrap();

    assert_eq!(email.to.len(), 2);
    assert_eq!(email.cc.len(), 1);
    assert_eq!(email.bcc.len(), 1);
    assert_eq!(email.total_recipients(), 4);
}

#[test]
fn test_email_builder_with_html() {
    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("HTML Email")
        .text_body("Plain text")
        .html_body("<html><body>HTML content</body></html>")
        .build()
        .unwrap();

    assert!(email.is_html());
    assert!(email.text_body.is_some());
    assert!(email.html_body.is_some());
}

#[test]
fn test_email_builder_with_attachments() {
    let attachment = Attachment::new(
        "test.txt",
        b"Hello World".to_vec(),
        "text/plain"
    );

    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("With Attachment")
        .text_body("See attachment")
        .attachment(attachment)
        .build()
        .unwrap();

    assert!(email.has_attachments());
    assert_eq!(email.attachments.len(), 1);
    assert_eq!(email.total_attachment_size(), 11);
}

#[test]
fn test_email_builder_with_priority() {
    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("Urgent")
        .text_body("High priority email")
        .priority(EmailPriority::Urgent)
        .build()
        .unwrap();

    assert_eq!(email.priority, EmailPriority::Urgent);
}

#[test]
fn test_email_builder_with_tags() {
    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("Tagged Email")
        .text_body("Body")
        .tag("campaign", "welcome")
        .tag("version", "v1")
        .build()
        .unwrap();

    assert_eq!(email.tags.get("campaign"), Some(&"welcome".to_string()));
    assert_eq!(email.tags.get("version"), Some(&"v1".to_string()));
}

#[test]
fn test_email_builder_missing_from() {
    let result = Email::builder()
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("Test")
        .text_body("Body")
        .build();

    assert!(result.is_err());
}

#[test]
fn test_email_builder_missing_recipients() {
    let result = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .subject("Test")
        .text_body("Body")
        .build();

    assert!(result.is_err());
}

#[test]
fn test_email_builder_missing_body() {
    let result = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("Test")
        .build();

    assert!(result.is_err());
}

#[test]
fn test_email_validation() {
    let valid_email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("recipient@example.com").unwrap())
        .subject("Valid")
        .text_body("Body")
        .build()
        .unwrap();

    assert!(valid_email.validate().is_ok());
}

#[test]
fn test_email_all_recipients() {
    let email = Email::builder()
        .from(EmailAddress::new("sender@example.com").unwrap())
        .to(EmailAddress::new("to1@example.com").unwrap())
        .to(EmailAddress::new("to2@example.com").unwrap())
        .cc(EmailAddress::new("cc@example.com").unwrap())
        .bcc(EmailAddress::new("bcc@example.com").unwrap())
        .subject("Test")
        .text_body("Body")
        .build()
        .unwrap();

    let all = email.all_recipients();
    assert_eq!(all.len(), 4);
}
