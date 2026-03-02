//! Privacy-first redaction of sensitive values before journal writes.
//!
//! Patterns are applied in order. Each match is replaced with a
//! `[REDACTED:<kind>]` placeholder so the prompt remains readable
//! while secrets are removed.

use regex::Regex;
use std::sync::OnceLock;

/// Patterns ordered from most-specific to least-specific to avoid
/// partial matches being clobbered by a broader rule.
static PATTERNS: &[(&str, &str)] = &[
    // PEM private key blocks (multiline)
    (
        "private_key",
        r"(?s)-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----.*?-----END (?:RSA |EC |DSA )?PRIVATE KEY-----",
    ),
    // JWTs — three base64url segments (match before generic token= rule)
    ("jwt", r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+"),
    // Known API key prefixes
    (
        "api_key",
        r"(?:sk-proj-|sk-|ghp_|ghs_|glpat-|AKIA[A-Z0-9]{12}|xoxb-|xoxp-)[A-Za-z0-9_\-]{16,}",
    ),
    // Bearer token values in auth headers
    ("bearer_token", r"(?i)Bearer\s+[A-Za-z0-9_\-\.]{20,}"),
    // key=value / key: value credentials
    (
        "credential",
        r#"(?i)(?:password|passwd|secret|token|auth|api[_\-]?key)\s*[:=]\s*["']?[^\s"',\]]{8,}["']?"#,
    ),
    // Email addresses
    ("email", r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}"),
];

static COMPILED: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();

fn compiled_patterns() -> &'static [(&'static str, Regex)] {
    COMPILED.get_or_init(|| {
        PATTERNS
            .iter()
            .map(|(kind, pat)| (*kind, Regex::new(pat).expect("redaction pattern is valid")))
            .collect()
    })
}

/// Redact sensitive values from `text`.
///
/// Returns the sanitised string and the kinds of values that were found,
/// so callers can warn the user without revealing the actual values.
pub fn redact(text: &str) -> (String, Vec<&'static str>) {
    let mut result = text.to_string();
    let mut kinds = Vec::new();

    for (kind, re) in compiled_patterns() {
        if re.is_match(&result) {
            kinds.push(*kind);
            result = re
                .replace_all(&result, format!("[REDACTED:{kind}]").as_str())
                .into_owned();
        }
    }

    (result, kinds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_sensitive_data_unchanged() {
        let input = "implement JWT expiry validation in src/auth.rs";
        let (out, kinds) = redact(input);
        assert_eq!(out, input);
        assert!(kinds.is_empty());
    }

    #[test]
    fn test_redacts_openai_api_key() {
        let input = "use this key: sk-abcdefghijklmnopqrstuvwxyz123456";
        let (out, kinds) = redact(input);
        assert!(!out.contains("sk-abc"));
        assert!(out.contains("[REDACTED:api_key]"));
        assert_eq!(kinds, vec!["api_key"]);
    }

    #[test]
    fn test_redacts_github_token() {
        let input = "token is ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabc123";
        let (out, kinds) = redact(input);
        assert!(out.contains("[REDACTED:api_key]"));
        assert_eq!(kinds, vec!["api_key"]);
    }

    #[test]
    fn test_redacts_jwt() {
        let jwt =
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let input = format!("auth header: {jwt}");
        let (out, kinds) = redact(&input);
        assert!(!out.contains("eyJ"));
        assert!(out.contains("[REDACTED:jwt]"));
        assert_eq!(kinds[0], "jwt");
    }

    #[test]
    fn test_redacts_password_credential() {
        let input = "connect with password=hunter2_secret_abc";
        let (out, kinds) = redact(input);
        assert!(!out.contains("hunter2"));
        assert!(out.contains("[REDACTED:credential]"));
        assert_eq!(kinds[0], "credential");
    }

    #[test]
    fn test_redacts_bearer_token() {
        let input = "curl -H 'Authorization: Bearer abcdefghijklmnopqrstuvwxyz1234567890'";
        let (out, kinds) = redact(input);
        assert!(!out.contains("abcdefghijk"));
        assert!(out.contains("[REDACTED:bearer_token]"));
        assert!(!kinds.is_empty());
    }

    #[test]
    fn test_redacts_email_address() {
        let input = "yes, my email is user@example.com";
        let (out, kinds) = redact(input);
        assert!(!out.contains("user@example.com"));
        assert!(out.contains("[REDACTED:email]"));
        assert_eq!(kinds[0], "email");
    }

    #[test]
    fn test_multiple_redactions_in_one_prompt() {
        let input = "secret=supersecretvalue123 and ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
        let (out, kinds) = redact(input);
        assert!(out.contains("[REDACTED:api_key]"));
        assert!(out.contains("[REDACTED:credential]"));
        assert_eq!(kinds.len(), 2);
    }
}
