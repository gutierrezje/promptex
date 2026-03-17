/// Sanitize prompt text so it is safe to embed in Markdown renderers.
///
/// This targets common breakages such as HTML tags (`</details>`) and
/// multi-line JSON blobs that can escape blockquote formatting.
pub fn sanitize_for_markdown(text: &str) -> String {
    if !needs_sanitize(text) {
        return text.to_string();
    }

    let mut out = text.replace("\r\n", "\n").replace('\r', "\n");
    out = escape_html(&out);
    out = escape_fence_prefix(out);
    out.replace('\n', "\\n")
}

fn needs_sanitize(text: &str) -> bool {
    let trimmed = text.trim_start();
    text.contains('\n')
        || text.contains('\r')
        || text.contains("```")
        || text.contains("~~~")
        || text.contains("<details")
        || text.contains("</details")
        || text.contains("<summary")
        || text.contains("</summary")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_fence_prefix(mut text: String) -> String {
    let first_non_ws = text
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| i);

    if let Some(idx) = first_non_ws {
        if text[idx..].starts_with("```") {
            text.replace_range(idx..idx + 3, "\\`\\`\\`");
        } else if text[idx..].starts_with("~~~") {
            text.replace_range(idx..idx + 3, "\\~\\~\\~");
        }
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_returns_safe_text_unchanged() {
        let input = "simple prompt text";
        assert_eq!(sanitize_for_markdown(input), input);
    }

    #[test]
    fn sanitize_escapes_html_and_newlines() {
        let input = "notice anything odd\n</details>\n{\"a\":1}";
        let out = sanitize_for_markdown(input);
        assert!(!out.contains('\n'));
        assert!(out.contains("\\n"));
        assert!(out.contains("&lt;/details&gt;"));
    }

    #[test]
    fn sanitize_escapes_code_fence_prefix() {
        let input = "```json\n{\"a\":1}\n```";
        let out = sanitize_for_markdown(input);
        assert!(!out.trim_start().starts_with("```"));
        assert!(out.contains("\\n"));
    }
}
