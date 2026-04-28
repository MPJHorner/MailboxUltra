use base64::Engine;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One captured email, with the parsed envelope, the parsed structure, and the
/// raw RFC 822 bytes preserved verbatim.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: Uuid,
    pub received_at: DateTime<Utc>,
    /// The address given in `MAIL FROM:`. May differ from the `From:` header.
    pub envelope_from: String,
    /// The addresses given via `RCPT TO:`. May differ from `To:` / `Cc:`.
    pub envelope_to: Vec<String>,
    pub remote_addr: String,
    /// Was AUTH PLAIN / AUTH LOGIN used on this session.
    pub authenticated: bool,
    /// Parsed `From:` header (display + address).
    pub from: Option<EmailAddress>,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub subject: Option<String>,
    /// All headers in original order. Each entry is a `(name, value)` pair.
    pub headers: Vec<(String, String)>,
    /// Plain-text body, if the message contained a `text/plain` part.
    pub text: Option<String>,
    /// HTML body, if the message contained a `text/html` part.
    pub html: Option<String>,
    pub attachments: Vec<Attachment>,
    pub size: usize,
    /// Raw RFC 822 bytes, exactly as received over the wire.
    #[serde(skip_serializing)]
    pub raw: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailAddress {
    pub name: Option<String>,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Attachment {
    pub filename: Option<String>,
    pub content_type: String,
    pub size: usize,
    /// Bytes, base64-encoded for JSON. The raw download endpoint streams the
    /// decoded bytes back.
    #[serde(rename = "data_base64")]
    pub data_base64: String,
    #[serde(skip_serializing)]
    pub data: Bytes,
}

impl Message {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Parse a raw RFC 822 byte buffer (the DATA segment of an SMTP transaction
/// after dot-unstuffing) plus the SMTP envelope into a [`Message`].
///
/// All parsing is best-effort. When the input is not valid MIME we still hand
/// back a `Message` with whatever fields we could extract; the raw bytes are
/// always preserved.
pub fn parse_message(
    raw: Bytes,
    envelope_from: String,
    envelope_to: Vec<String>,
    remote_addr: String,
    authenticated: bool,
) -> Message {
    let id = Uuid::new_v4();
    let size = raw.len();
    let now = Utc::now();

    use mail_parser::MimeHeaders;
    let parsed = mail_parser::MessageParser::default().parse(raw.as_ref());
    let mut from = None;
    let mut to = Vec::new();
    let mut cc = Vec::new();
    let mut subject = None;
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut text = None;
    let mut html = None;
    let mut attachments = Vec::new();

    if let Some(msg) = parsed {
        for h in msg.headers() {
            let name = h.name().to_string();
            let value = render_header_value(h.value());
            headers.push((name, value));
        }
        from = msg.from().and_then(|a| address_first(a));
        to = msg.to().map(address_list).unwrap_or_default();
        cc = msg.cc().map(address_list).unwrap_or_default();
        subject = msg.subject().map(|s| s.to_string());
        text = msg.body_text(0).map(|s| s.into_owned());
        html = msg.body_html(0).map(|s| s.into_owned());

        for att in msg.attachments() {
            let bytes_slice: &[u8] = match &att.body {
                mail_parser::PartType::Binary(b) | mail_parser::PartType::InlineBinary(b) => {
                    b.as_ref()
                }
                mail_parser::PartType::Text(t) | mail_parser::PartType::Html(t) => t.as_bytes(),
                _ => continue,
            };
            let data = Bytes::copy_from_slice(bytes_slice);
            let filename = att.attachment_name().map(|s| s.to_string());
            let content_type = att
                .content_type()
                .map(|c| {
                    let mut s = String::new();
                    s.push_str(c.ctype());
                    if let Some(sub) = c.subtype() {
                        s.push('/');
                        s.push_str(sub);
                    }
                    s
                })
                .unwrap_or_else(|| "application/octet-stream".into());
            let size = data.len();
            let data_base64 = base64::engine::general_purpose::STANDARD.encode(&data);
            attachments.push(Attachment {
                filename,
                content_type,
                size,
                data_base64,
                data,
            });
        }
    }

    Message {
        id,
        received_at: now,
        envelope_from,
        envelope_to,
        remote_addr,
        authenticated,
        from,
        to,
        cc,
        subject,
        headers,
        text,
        html,
        attachments,
        size,
        raw,
    }
}

fn address_first(addr: &mail_parser::Address<'_>) -> Option<EmailAddress> {
    addr.iter().next().map(|a| EmailAddress {
        name: a.name().map(|s| s.to_string()),
        address: a.address().unwrap_or("").to_string(),
    })
}

fn address_list(addr: &mail_parser::Address<'_>) -> Vec<EmailAddress> {
    addr.iter()
        .map(|a| EmailAddress {
            name: a.name().map(|s| s.to_string()),
            address: a.address().unwrap_or("").to_string(),
        })
        .collect()
}

fn render_header_value(value: &mail_parser::HeaderValue<'_>) -> String {
    match value {
        mail_parser::HeaderValue::Text(t) => t.to_string(),
        mail_parser::HeaderValue::TextList(list) => list
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        mail_parser::HeaderValue::DateTime(dt) => dt.to_rfc822(),
        mail_parser::HeaderValue::Address(addr) => addr
            .iter()
            .map(|a| match (a.name(), a.address()) {
                (Some(n), Some(addr)) => format!("\"{n}\" <{addr}>"),
                (None, Some(addr)) => addr.to_string(),
                (Some(n), None) => n.to_string(),
                (None, None) => String::new(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        mail_parser::HeaderValue::ContentType(ct) => {
            let mut s = ct.ctype().to_string();
            if let Some(sub) = ct.subtype() {
                s.push('/');
                s.push_str(sub);
            }
            if let Some(attrs) = ct.attributes() {
                for (k, v) in attrs.iter() {
                    s.push_str("; ");
                    s.push_str(k);
                    s.push('=');
                    s.push_str(v);
                }
            }
            s
        }
        mail_parser::HeaderValue::Received(_) | mail_parser::HeaderValue::Empty => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(s: &str) -> Bytes {
        Bytes::copy_from_slice(s.replace('\n', "\r\n").as_bytes())
    }

    #[test]
    fn parses_a_simple_text_email() {
        let r = raw("From: \"Alice\" <alice@example.com>\n\
             To: bob@example.com\n\
             Subject: Hello\n\
             Date: Mon, 28 Apr 2026 12:00:00 +0000\n\
             Content-Type: text/plain; charset=utf-8\n\
             \n\
             hello world\n");
        let msg = parse_message(
            r.clone(),
            "alice@example.com".into(),
            vec!["bob@example.com".into()],
            "127.0.0.1:1".into(),
            false,
        );
        assert_eq!(msg.subject.as_deref(), Some("Hello"));
        assert_eq!(
            msg.from.as_ref().unwrap().address,
            "alice@example.com".to_string()
        );
        assert_eq!(msg.from.as_ref().unwrap().name.as_deref(), Some("Alice"));
        assert_eq!(msg.to.len(), 1);
        assert_eq!(msg.to[0].address, "bob@example.com".to_string());
        assert!(msg.text.unwrap().contains("hello world"));
        assert!(!msg.authenticated);
    }

    #[test]
    fn parses_html_and_text_alternative() {
        let body = "From: a@x\n\
             To: b@x\n\
             Subject: Mixed\n\
             MIME-Version: 1.0\n\
             Content-Type: multipart/alternative; boundary=BOUND\n\
             \n\
             --BOUND\n\
             Content-Type: text/plain\n\
             \n\
             plain body\n\
             --BOUND\n\
             Content-Type: text/html\n\
             \n\
             <p>html body</p>\n\
             --BOUND--\n";
        let msg = parse_message(
            raw(body),
            "a@x".into(),
            vec!["b@x".into()],
            "1.1.1.1:1".into(),
            false,
        );
        assert!(msg.text.unwrap().contains("plain body"));
        assert!(msg.html.unwrap().contains("html body"));
    }

    #[test]
    fn parses_attachment() {
        let body = "From: a@x\n\
             To: b@x\n\
             Subject: With attachment\n\
             MIME-Version: 1.0\n\
             Content-Type: multipart/mixed; boundary=BOUND\n\
             \n\
             --BOUND\n\
             Content-Type: text/plain\n\
             \n\
             see attachment\n\
             --BOUND\n\
             Content-Type: application/pdf; name=\"r.pdf\"\n\
             Content-Disposition: attachment; filename=\"r.pdf\"\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             aGVsbG8gd29ybGQ=\n\
             --BOUND--\n";
        let msg = parse_message(
            raw(body),
            "a@x".into(),
            vec!["b@x".into()],
            "1.1.1.1:1".into(),
            false,
        );
        assert_eq!(msg.attachments.len(), 1);
        let att = &msg.attachments[0];
        assert_eq!(att.filename.as_deref(), Some("r.pdf"));
        assert_eq!(att.content_type, "application/pdf");
        assert_eq!(&att.data[..], b"hello world");
        assert_eq!(att.size, 11);
    }

    #[test]
    fn header_lookup_is_case_insensitive() {
        let r = raw("From: a@x\nTo: b@x\nSubject: hi\n\nbody\n");
        let msg = parse_message(r, "a@x".into(), vec!["b@x".into()], "1:1".into(), false);
        assert!(msg.header("from").is_some());
        assert!(msg.header("FROM").is_some());
        assert!(msg.header("nope").is_none());
    }

    #[test]
    fn keeps_raw_even_when_unparseable() {
        let r = Bytes::from_static(b"not a real email at all");
        let msg = parse_message(
            r.clone(),
            "a@x".into(),
            vec!["b@x".into()],
            "1:1".into(),
            false,
        );
        assert_eq!(msg.raw, r);
        assert_eq!(msg.size, r.len());
    }

    #[test]
    fn email_address_serialises_round_trip() {
        let a = EmailAddress {
            name: Some("Alice".into()),
            address: "alice@example.com".into(),
        };
        let s = serde_json::to_string(&a).unwrap();
        let back: EmailAddress = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn message_serialises_size_and_skips_raw() {
        let r = raw("From: a@x\nTo: b@x\nSubject: hi\n\nbody\n");
        let msg = parse_message(r, "a@x".into(), vec!["b@x".into()], "1:1".into(), false);
        let v: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert!(v["size"].as_u64().unwrap() > 0);
        assert!(v.get("raw").is_none());
    }
}
