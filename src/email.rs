use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE, Engine};

/// Decode base64url-encoded email content
pub fn decode_email_content(data: &str) -> Result<String> {
    let decoded = URL_SAFE.decode(data)?;
    Ok(String::from_utf8(decoded)?)
}

/// Check if content contains HTML tags
#[allow(dead_code)]
pub fn is_html_content(content: &str) -> bool {
    let content = content.trim();
    if content.is_empty() {
        return false;
    }

    let html_tags = [
        "<html", "<body", "<p", "<div", "<span", "<h1", "<h2", "<h3", "<h4", "<h5", "<h6",
        "<strong", "<b", "<em", "<i", "<u", "<a", "<img", "<br", "<hr", "<ul", "<ol", "<li",
        "<table", "<tr", "<td", "<th",
    ];

    let content_lower = content.to_lowercase();
    html_tags.iter().any(|tag| content_lower.contains(tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_html_content() {
        assert!(is_html_content("<p>Hello</p>"));
        assert!(is_html_content("<div>Content</div>"));
        assert!(!is_html_content("Plain text"));
        assert!(!is_html_content(""));
    }
}
