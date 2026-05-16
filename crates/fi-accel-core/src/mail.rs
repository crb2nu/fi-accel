use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MailError {
    #[error("message is not valid UTF-8-compatible mail data")]
    InvalidMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub filename: Option<String>,
    pub content_type: String,
    pub sha256: String,
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedEmail {
    pub headers: BTreeMap<String, String>,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
}

pub fn parse_eml(data: &[u8]) -> Result<ParsedEmail, MailError> {
    let text = String::from_utf8_lossy(data);
    let (headers, body) = split_headers_body(&text).ok_or(MailError::InvalidMessage)?;
    let headers = parse_headers(headers);
    let mut parsed = ParsedEmail {
        headers: headers.clone(),
        body_text: String::new(),
        body_html: None,
        attachments: Vec::new(),
    };
    parse_part(&headers, body.as_bytes(), &mut parsed);
    Ok(parsed)
}

pub fn parse_mbox(data: &[u8]) -> Result<Vec<ParsedEmail>, MailError> {
    let text = String::from_utf8_lossy(data);
    let mut messages = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if line.starts_with("From ") && !current.trim().is_empty() {
            messages.push(parse_eml(current.as_bytes())?);
            current.clear();
            continue;
        }
        if !line.starts_with("From ") || !current.is_empty() {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() {
        messages.push(parse_eml(current.as_bytes())?);
    }

    Ok(messages)
}

fn parse_part(headers: &BTreeMap<String, String>, body: &[u8], parsed: &mut ParsedEmail) {
    let content_type = header(headers, "content-type").unwrap_or("text/plain");

    if content_type.to_ascii_lowercase().starts_with("multipart/") {
        if let Some(boundary) = parameter(content_type, "boundary") {
            for part in split_multipart(body, &boundary) {
                let part_text = String::from_utf8_lossy(&part);
                if let Some((part_headers, part_body)) = split_headers_body(&part_text) {
                    let part_headers = parse_headers(part_headers);
                    parse_part(&part_headers, part_body.as_bytes(), parsed);
                }
            }
        }
        return;
    }

    let transfer_encoding = header(headers, "content-transfer-encoding").unwrap_or("");
    let decoded = decode_transfer(body, transfer_encoding);
    let content_disposition = header(headers, "content-disposition").unwrap_or("");
    let filename =
        parameter(content_disposition, "filename").or_else(|| parameter(content_type, "name"));

    if content_disposition
        .to_ascii_lowercase()
        .contains("attachment")
        || filename.is_some()
    {
        let mut hasher = Sha256::new();
        hasher.update(&decoded);
        parsed.attachments.push(AttachmentInfo {
            filename,
            content_type: media_type(content_type).to_string(),
            sha256: format!("{:x}", hasher.finalize()),
            size: decoded.len(),
        });
        return;
    }

    let body_text = String::from_utf8_lossy(&decoded).trim().to_string();
    let media_type = media_type(content_type);
    if media_type.eq_ignore_ascii_case("text/html") {
        parsed.body_html = Some(body_text);
    } else if media_type.eq_ignore_ascii_case("text/plain") && parsed.body_text.is_empty() {
        parsed.body_text = body_text;
    }
}

fn split_headers_body(text: &str) -> Option<(&str, &str)> {
    text.split_once("\r\n\r\n")
        .or_else(|| text.split_once("\n\n"))
}

fn parse_headers(headers: &str) -> BTreeMap<String, String> {
    let mut parsed = BTreeMap::new();
    let mut current_key: Option<String> = None;

    for line in headers.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(key) = &current_key {
                let entry = parsed.entry(key.clone()).or_insert_with(String::new);
                entry.push(' ');
                entry.push_str(line.trim());
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let normalized = key.trim().to_ascii_lowercase();
            parsed.insert(normalized.clone(), value.trim().to_string());
            current_key = Some(normalized);
        }
    }

    parsed
}

fn header<'a>(headers: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    headers.get(&key.to_ascii_lowercase()).map(String::as_str)
}

fn media_type(value: &str) -> &str {
    value.split(';').next().unwrap_or(value).trim()
}

fn parameter(value: &str, name: &str) -> Option<String> {
    let prefix = format!("{}=", name.to_ascii_lowercase());
    for part in value.split(';').skip(1) {
        let trimmed = part.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with(&prefix) {
            let raw = trimmed.split_once('=')?.1.trim();
            return Some(raw.trim_matches('"').to_string());
        }
    }
    None
}

fn split_multipart(body: &[u8], boundary: &str) -> Vec<Vec<u8>> {
    let marker = format!("--{}", boundary);
    let closing = format!("--{}--", boundary);
    let text = String::from_utf8_lossy(body);
    let mut parts = Vec::new();

    for segment in text.split(&marker).skip(1) {
        if segment.starts_with("--") || segment.starts_with(&closing) {
            break;
        }
        let trimmed = segment.trim_start_matches(['\r', '\n']).trim_end();
        if !trimmed.is_empty() {
            parts.push(trimmed.as_bytes().to_vec());
        }
    }

    parts
}

fn decode_transfer(body: &[u8], encoding: &str) -> Vec<u8> {
    match encoding.trim().to_ascii_lowercase().as_str() {
        "base64" => decode_base64(body).unwrap_or_else(|| body.to_vec()),
        "quoted-printable" => decode_quoted_printable(body),
        _ => body.to_vec(),
    }
}

fn decode_quoted_printable(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        if body[i] == b'=' && i + 2 < body.len() {
            if body[i + 1] == b'\r' && body[i + 2] == b'\n' {
                i += 3;
                continue;
            }
            if body[i + 1] == b'\n' {
                i += 2;
                continue;
            }
            if let (Some(high), Some(low)) = (hex_value(body[i + 1]), hex_value(body[i + 2])) {
                out.push((high << 4) | low);
                i += 3;
                continue;
            }
        }
        out.push(body[i]);
        i += 1;
    }
    out
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn decode_base64(body: &[u8]) -> Option<Vec<u8>> {
    let clean: Vec<u8> = body
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut len = 0;

    for byte in clean {
        chunk[len] = byte;
        len += 1;
        if len == 4 {
            decode_base64_chunk(&chunk, &mut out)?;
            len = 0;
        }
    }

    if len != 0 {
        return None;
    }

    Some(out)
}

fn decode_base64_chunk(chunk: &[u8; 4], out: &mut Vec<u8>) -> Option<()> {
    let a = base64_value(chunk[0])?;
    let b = base64_value(chunk[1])?;
    let c = if chunk[2] == b'=' {
        0
    } else {
        base64_value(chunk[2])?
    };
    let d = if chunk[3] == b'=' {
        0
    } else {
        base64_value(chunk[3])?
    };

    out.push((a << 2) | (b >> 4));
    if chunk[2] != b'=' {
        out.push((b << 4) | (c >> 2));
    }
    if chunk[3] != b'=' {
        out.push((c << 6) | d);
    }
    Some(())
}

fn base64_value(value: u8) -> Option<u8> {
    match value {
        b'A'..=b'Z' => Some(value - b'A'),
        b'a'..=b'z' => Some(value - b'a' + 26),
        b'0'..=b'9' => Some(value - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multipart_email() {
        let message = b"Subject: Hi\nContent-Type: multipart/alternative; boundary=\"x\"\n\n--x\nContent-Type: text/plain\n\nHello text\n--x\nContent-Type: text/html\n\n<p>Hello</p>\n--x--\n";
        let parsed = parse_eml(message).unwrap();
        assert_eq!(parsed.headers.get("subject").unwrap(), "Hi");
        assert_eq!(parsed.body_text, "Hello text");
        assert_eq!(parsed.body_html.as_deref(), Some("<p>Hello</p>"));
    }

    #[test]
    fn records_attachment_metadata() {
        let message = b"Content-Type: multipart/mixed; boundary=\"x\"\n\n--x\nContent-Type: text/plain\n\nBody\n--x\nContent-Type: text/plain; name=\"a.txt\"\nContent-Disposition: attachment; filename=\"a.txt\"\nContent-Transfer-Encoding: base64\n\nYXR0YWNo\n--x--\n";
        let parsed = parse_eml(message).unwrap();
        assert_eq!(parsed.body_text, "Body");
        assert_eq!(parsed.attachments.len(), 1);
        assert_eq!(parsed.attachments[0].filename.as_deref(), Some("a.txt"));
        assert_eq!(parsed.attachments[0].size, 6);
    }
}
