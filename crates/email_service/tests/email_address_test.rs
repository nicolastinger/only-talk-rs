use email_service::{EmailAddress, EmailError};

#[test]
fn test_valid_email_address() {
    let addr = EmailAddress::new("test@example.com");
    assert!(addr.is_ok());

    let addr = addr.unwrap();
    assert_eq!(addr.address(), "test@example.com");
    assert_eq!(addr.domain(), Some("example.com"));
    assert_eq!(addr.local_part(), Some("test"));
    assert!(addr.name().is_none());
}

#[test]
fn test_valid_email_with_name() {
    let addr = EmailAddress::with_name("test@example.com", "Test User");
    assert!(addr.is_ok());

    let addr = addr.unwrap();
    assert_eq!(addr.name(), Some("Test User"));
    assert_eq!(addr.to_string(), "Test User <test@example.com>");
}

#[test]
fn test_invalid_email_address() {
    let result = EmailAddress::new("invalid-email");
    assert!(result.is_err());

    if let Err(EmailError::InvalidEmailAddress(email)) = result {
        assert_eq!(email, "invalid-email");
    } else {
        panic!("Expected InvalidEmailAddress error");
    }
}

#[test]
fn test_email_validation() {
    assert!(EmailAddress::is_valid("test@example.com"));
    assert!(EmailAddress::is_valid("user.name@example.com"));
    assert!(EmailAddress::is_valid("user+tag@example.com"));
    assert!(!EmailAddress::is_valid("invalid"));
    assert!(!EmailAddress::is_valid("invalid@"));
    assert!(!EmailAddress::is_valid("@example.com"));
}

#[test]
fn test_email_address_display() {
    let addr = EmailAddress::new("test@example.com").unwrap();
    assert_eq!(format!("{}", addr), "test@example.com");

    let addr_with_name = EmailAddress::with_name("test@example.com", "Test").unwrap();
    assert_eq!(format!("{}", addr_with_name), "Test <test@example.com>");
}

#[test]
fn test_email_address_try_from() {
    let addr: EmailAddress = "test@example.com".try_into().unwrap();
    assert_eq!(addr.address(), "test@example.com");

    let result: Result<EmailAddress, _> = "invalid".try_into();
    assert!(result.is_err());
}
