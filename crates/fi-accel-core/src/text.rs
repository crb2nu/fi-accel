//! Text extraction helpers.

use regex::{Captures, Regex};

/// Convert HTML into readable plain text.
///
/// This intentionally targets deterministic extraction for ingestion pipelines:
/// scripts/styles are dropped, common block tags become paragraph boundaries,
/// entities are decoded, and links can optionally keep their destination.
pub fn html_to_text(html: &str, preserve_links: bool) -> String {
    let mut text = remove_tag_block(html, "script");
    text = remove_tag_block(&text, "style");
    text = remove_tag_block(&text, "noscript");
    text = replace_links(&text, preserve_links);

    let br_re = Regex::new(r"(?i)<\s*br\s*/?\s*>").unwrap();
    text = br_re.replace_all(&text, "\n").into_owned();

    let block_re = Regex::new(
        r"(?ix)</?\s*(address|article|aside|blockquote|body|caption|dd|details|dialog|div|dl|dt|fieldset|figcaption|figure|footer|form|h[1-6]|head|header|hr|html|li|main|nav|ol|p|pre|section|table|tbody|td|tfoot|th|thead|tr|ul)\b[^>]*>",
    )
    .unwrap();
    text = block_re.replace_all(&text, "\n\n").into_owned();

    let tag_re = Regex::new(r"(?s)<[^>]+>").unwrap();
    text = tag_re.replace_all(&text, "").into_owned();
    normalize_text(&decode_entities(&text))
}

fn remove_tag_block(html: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<\s*{}\b[^>]*>.*?</\s*{}\s*>", tag, tag);
    Regex::new(&pattern)
        .unwrap()
        .replace_all(html, "")
        .into_owned()
}

fn replace_links(html: &str, preserve_links: bool) -> String {
    let link_re = Regex::new(r"(?is)<\s*a\b([^>]*)>(.*?)</\s*a\s*>").unwrap();
    let href_re = Regex::new(r#"(?is)\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).unwrap();

    link_re
        .replace_all(html, |caps: &Captures<'_>| {
            let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let label = html_fragment_to_inline_text(caps.get(2).map(|m| m.as_str()).unwrap_or(""));
            if !preserve_links {
                return label;
            }

            let href = href_re.captures(attrs).and_then(|href_caps| {
                href_caps
                    .get(1)
                    .or_else(|| href_caps.get(2))
                    .or_else(|| href_caps.get(3))
                    .map(|m| decode_entities(m.as_str()).trim().to_string())
            });

            match href {
                Some(url) if !url.is_empty() && !label.is_empty() => format!("{} ({})", label, url),
                Some(url) if !url.is_empty() => url,
                _ => label,
            }
        })
        .into_owned()
}

fn html_fragment_to_inline_text(fragment: &str) -> String {
    let tag_re = Regex::new(r"(?s)<[^>]+>").unwrap();
    normalize_inline(&decode_entities(&tag_re.replace_all(fragment, " ")))
}

fn normalize_text(text: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();

    for raw_line in text.replace('\r', "\n").lines() {
        let line = normalize_inline(raw_line);
        if line.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }

    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }

    paragraphs.join("\n\n")
}

fn normalize_inline(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(text: &str) -> String {
    let entity_re = Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[A-Za-z][A-Za-z0-9]+);").unwrap();
    entity_re
        .replace_all(text, |caps: &Captures<'_>| decode_entity(&caps[1]))
        .into_owned()
}

fn decode_entity(entity: &str) -> String {
    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("&{};", entity));
    }

    if let Some(decimal) = entity.strip_prefix('#') {
        return decimal
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("&{};", entity));
    }

    match entity {
        "amp" => "&",
        "apos" => "'",
        "copy" => "(c)",
        "gt" => ">",
        "hellip" => "...",
        "ldquo" | "rdquo" | "quot" => "\"",
        "lsquo" | "rsquo" => "'",
        "lt" => "<",
        "mdash" | "ndash" => "-",
        "nbsp" => " ",
        "reg" => "(r)",
        _ => return format!("&{};", entity),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_decodes_entities_and_blocks() {
        let html = "<h1>Title &amp; More</h1><p>Hello&nbsp;<b>world</b>.</p><div>Next</div>";
        assert_eq!(
            html_to_text(html, false),
            "Title & More\n\nHello world.\n\nNext"
        );
    }

    #[test]
    fn html_to_text_drops_script_and_style() {
        let html = "<style>body{}</style><p>Keep</p><script>alert(1)</script><p>Me</p>";
        assert_eq!(html_to_text(html, false), "Keep\n\nMe");
    }

    #[test]
    fn html_to_text_preserves_links_when_requested() {
        let html =
            r#"<p>Read <a href="https://example.test?a=1&amp;b=2"><span>the docs</span></a>.</p>"#;
        assert_eq!(
            html_to_text(html, true),
            "Read the docs (https://example.test?a=1&b=2)."
        );
        assert_eq!(html_to_text(html, false), "Read the docs.");
    }

    #[test]
    fn html_to_text_decodes_numeric_entities() {
        assert_eq!(html_to_text("<p>&#65;&#x42;</p>", false), "AB");
    }
}
