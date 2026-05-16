//! Lightweight character encoding detection for ingestion inputs.

/// Result from [`detect_encoding`].
#[derive(Debug, Clone, PartialEq)]
pub struct EncodingDetection {
    pub encoding: String,
    pub confidence: f32,
    pub bom: bool,
}

/// Detect the most likely text encoding from a byte sample.
///
/// The detector is deliberately small and deterministic. It identifies ASCII,
/// valid UTF-8, UTF BOMs, and common Western single-byte fallbacks.
pub fn detect_encoding(data: &[u8], sample_bytes: usize) -> EncodingDetection {
    let sample_len = data.len().min(sample_bytes);
    let sample = &data[..sample_len];

    if sample.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return detection("utf-8", 1.0, true);
    }
    if sample.starts_with(&[0xFF, 0xFE]) {
        return detection("utf-16le", 1.0, true);
    }
    if sample.starts_with(&[0xFE, 0xFF]) {
        return detection("utf-16be", 1.0, true);
    }
    if sample.is_empty() || sample.iter().all(|&b| b <= 0x7F) {
        return detection("ascii", 1.0, false);
    }
    if std::str::from_utf8(sample).is_ok() {
        return detection("utf-8", 0.99, false);
    }

    let c1_bytes = sample
        .iter()
        .filter(|&&b| (0x80..=0x9F).contains(&b))
        .count();
    let cp1252_printable = sample
        .iter()
        .filter(|&&b| is_windows_1252_punctuation(b))
        .count();
    let cp1252_undefined = sample
        .iter()
        .filter(|&&b| matches!(b, 0x81 | 0x8D | 0x8F | 0x90 | 0x9D))
        .count();

    if cp1252_printable > 0 && cp1252_undefined == 0 {
        let confidence = (0.72 + (cp1252_printable as f32 * 0.03)).min(0.92);
        return detection("windows-1252", confidence, false);
    }

    if c1_bytes > 0 {
        return detection("iso-8859-1", 0.62, false);
    }

    detection("iso-8859-1", 0.55, false)
}

fn detection(encoding: &str, confidence: f32, bom: bool) -> EncodingDetection {
    EncodingDetection {
        encoding: encoding.to_string(),
        confidence,
        bom,
    }
}

fn is_windows_1252_punctuation(byte: u8) -> bool {
    matches!(
        byte,
        0x80 | 0x82
            ..=0x8C | 0x8E | 0x91
            ..=0x9C | 0x9E | 0x9F
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ascii() {
        let detected = detect_encoding(b"plain ascii", 8192);
        assert_eq!(detected.encoding, "ascii");
        assert_eq!(detected.confidence, 1.0);
    }

    #[test]
    fn detects_utf8() {
        let detected = detect_encoding("snowman: \u{2603}".as_bytes(), 8192);
        assert_eq!(detected.encoding, "utf-8");
        assert!(detected.confidence > 0.9);
    }

    #[test]
    fn detects_utf8_bom() {
        let detected = detect_encoding(b"\xEF\xBB\xBFhello", 8192);
        assert_eq!(detected.encoding, "utf-8");
        assert!(detected.bom);
    }

    #[test]
    fn detects_windows_1252_punctuation() {
        let detected = detect_encoding(b"\x93quoted\x94", 8192);
        assert_eq!(detected.encoding, "windows-1252");
        assert!(detected.confidence >= 0.7);
    }

    #[test]
    fn detects_iso_8859_1_fallback() {
        let detected = detect_encoding(b"caf\xE9", 8192);
        assert_eq!(detected.encoding, "iso-8859-1");
    }
}
