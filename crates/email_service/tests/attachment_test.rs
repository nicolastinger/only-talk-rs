use email_service::{Attachment, ContentDisposition};

#[test]
fn test_attachment_basic() {
    let attachment = Attachment::new(
        "test.txt",
        b"Hello World".to_vec(),
        "text/plain"
    );

    assert_eq!(attachment.filename, "test.txt");
    assert_eq!(attachment.size(), 11);
    assert_eq!(attachment.content_type, "text/plain");
    assert_eq!(attachment.disposition, ContentDisposition::Attachment);
    assert!(attachment.content_id.is_none());
}

#[test]
fn test_attachment_with_content_id() {
    let attachment = Attachment::new("image.png", vec![1, 2, 3], "image/png")
        .with_content_id("img001");

    assert_eq!(attachment.content_id, Some("img001".to_string()));
}

#[test]
fn test_attachment_inline() {
    let attachment = Attachment::inline("logo.png", vec![1, 2, 3], "image/png");

    assert_eq!(attachment.disposition, ContentDisposition::Inline);
}

#[test]
fn test_attachment_from_bytes() {
    let pdf_bytes = b"%PDF-1.4\n%test".to_vec();
    let attachment = Attachment::from_bytes("document.pdf", pdf_bytes);

    assert_eq!(attachment.filename, "document.pdf");
    assert_eq!(attachment.content_type, "application/pdf");
}

#[test]
fn test_attachment_detect_content_type_pdf() {
    let pdf_header = b"%PDF-1.4".to_vec();
    let attachment = Attachment::from_bytes("file.bin", pdf_header);
    assert_eq!(attachment.content_type, "application/pdf");
}

#[test]
fn test_attachment_detect_content_type_png() {
    let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let attachment = Attachment::from_bytes("file.bin", png_header);
    assert_eq!(attachment.content_type, "image/png");
}

#[test]
fn test_attachment_detect_content_type_jpeg() {
    let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let attachment = Attachment::from_bytes("file.bin", jpeg_header);
    assert_eq!(attachment.content_type, "image/jpeg");
}

#[test]
fn test_attachment_detect_content_type_by_extension() {
    let text_attachment = Attachment::from_bytes("file.txt", b"content".to_vec());
    assert_eq!(text_attachment.content_type, "text/plain");

    let html_attachment = Attachment::from_bytes("file.html", b"<html>".to_vec());
    assert_eq!(html_attachment.content_type, "text/html");

    let json_attachment = Attachment::from_bytes("file.json", b"{}".to_vec());
    assert_eq!(json_attachment.content_type, "application/json");
}

#[test]
fn test_attachment_from_base64() {
    let base64_content = "SGVsbG8gV29ybGQ="; // "Hello World" in base64
    let attachment = Attachment::from_base64("test.txt", base64_content);

    assert!(attachment.is_ok());
    let attachment = attachment.unwrap();
    assert_eq!(attachment.content, b"Hello World");
}

#[test]
fn test_attachment_from_invalid_base64() {
    let invalid_base64 = "!!!invalid!!!";
    let result = Attachment::from_base64("test.txt", invalid_base64);
    assert!(result.is_err());
}

#[test]
fn test_content_disposition_display() {
    assert_eq!(format!("{}", ContentDisposition::Attachment), "attachment");
    assert_eq!(format!("{}", ContentDisposition::Inline), "inline");
}

#[test]
fn test_content_disposition_default() {
    let disposition = ContentDisposition::default();
    assert_eq!(disposition, ContentDisposition::Attachment);
}

#[test]
fn test_attachment_mime_type() {
    let attachment = Attachment::new("test.txt", vec![], "text/plain; charset=utf-8");
    let mime = attachment.mime_type();
    assert!(mime.is_ok());
}
