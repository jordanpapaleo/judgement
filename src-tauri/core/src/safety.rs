//! Unsafe-content rejection (§8). One check, run on every text field of every
//! write path. A match rejects the whole write; nothing is redacted or stored.

use std::sync::LazyLock;

use regex::Regex;

use crate::error::{Error, Result};

struct Pattern {
    class: &'static str,
    regex: Regex,
}

static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let p = |class, re: &str| Pattern {
        class,
        regex: Regex::new(re).expect("valid pattern"),
    };
    vec![
        p("a private key", r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY"),
        p("an AWS access key", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
        p(
            "a GitHub token",
            r"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{22,})",
        ),
        p("a Slack token", r"\bxox[abposr]-[A-Za-z0-9-]{10,}"),
        p("an API key", r"\bsk-(?:[a-z]+-)?[A-Za-z0-9_-]{20,}"),
        p("a 1Password reference", r"\bop://[^\s/]+/[^\s/]+/[^\s]+"),
    ]
});

/// `password = hunter2`-style assignments. The value is checked separately so
/// prose like "token: rotate them weekly" isn't rejected.
static ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b(?:password|passwd|pwd|secret|token|api[_-]?key|access[_-]?key)\b["']?\s*[:=]\s*["']?([^\s"',;]+)"#,
    )
    .expect("valid pattern")
});

static LONG_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9+/=_\-]{32,}").expect("valid pattern"));

/// Shannon entropy threshold in bits per character. Hex digests top out at 4.0
/// and English words sit well below; random base64/base62 secrets exceed it.
const ENTROPY_THRESHOLD: f64 = 4.2;

/// Returns the pattern class the text matches, if any.
pub fn scan(text: &str) -> Option<&'static str> {
    if let Some(p) = PATTERNS.iter().find(|p| p.regex.is_match(text)) {
        return Some(p.class);
    }
    if ASSIGNMENT
        .captures_iter(text)
        .any(|c| looks_like_secret_value(&c[1]))
    {
        return Some("a credential assignment");
    }
    if LONG_TOKEN
        .find_iter(text)
        .any(|m| token_is_high_entropy(m.as_str()))
    {
        return Some("a long high-entropy string");
    }
    None
}

/// Path-like tokens (several `/`) are judged per segment, so a repo path
/// through a random temp directory isn't mistaken for one long secret.
fn token_is_high_entropy(token: &str) -> bool {
    if token.matches('/').count() >= 3 {
        token
            .split('/')
            .filter(|s| s.len() >= 32)
            .any(is_high_entropy)
    } else {
        is_high_entropy(token)
    }
}

/// Rejects `text` with an error naming `field` and the matched pattern class.
pub fn check(field: &str, text: &str) -> Result<()> {
    match scan(text) {
        Some(class) => Err(Error::Unsafe {
            field: field.to_string(),
            class,
        }),
        None => Ok(()),
    }
}

fn looks_like_secret_value(value: &str) -> bool {
    value.len() >= 8
        && value
            .chars()
            .any(|c| c.is_ascii_digit() || c.is_ascii_uppercase() || !c.is_alphanumeric())
        && !value.starts_with('<')
        && !value.starts_with("${")
}

fn is_high_entropy(token: &str) -> bool {
    let has_letter = token.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    has_letter && has_digit && shannon_entropy(token) > ENTROPY_THRESHOLD
}

fn shannon_entropy(text: &str) -> f64 {
    let mut counts = [0usize; 256];
    for b in text.bytes() {
        counts[b as usize] += 1;
    }
    let len = text.len() as f64;
    counts
        .iter()
        .filter(|&&n| n > 0)
        .map(|&n| {
            let p = n as f64 / len;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_secret_shapes() {
        let fixtures = [
            ("-----BEGIN RSA PRIVATE KEY-----", "a private key"),
            ("-----BEGIN OPENSSH PRIVATE KEY-----", "a private key"),
            ("key AKIAIOSFODNN7EXAMPLE here", "an AWS access key"),
            ("ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789", "a GitHub token"),
            (
                "github_pat_11ABCDEFG0123456789_abcdefghijklmnop",
                "a GitHub token",
            ),
            ("xoxb-1234567890-abcdefghij", "a Slack token"),
            ("sk-proj-abcdefghijklmnopqrstuvwx", "an API key"),
            ("sk-ant-api03-abcdefghijklmnopqrstuvwx", "an API key"),
            ("op://Private/GitHub/token", "a 1Password reference"),
            ("password = hunter2Hunter", "a credential assignment"),
            ("API_KEY: \"Zx81kq0pLm\"", "a credential assignment"),
            (
                "blob 9fK2mQ7xL0pR4tV8wZ1cB6nH3jD5sG2aE7uY0iO",
                "a long high-entropy string",
            ),
        ];
        for (text, class) in fixtures {
            assert_eq!(scan(text), Some(class), "{text}");
        }
    }

    #[test]
    fn accepts_ordinary_prose() {
        let fixtures = [
            "The user cut five tests that re-asserted a child's output.",
            "Rotate the token: rotate them weekly.",
            "Pass the password via the keychain, never inline.",
            "token = <redacted>",
            "Commit 3f2a9c1b8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a fixed it",
            "Item 01JPMXYZ1234567890ABCDEFGH was retrieved.",
            "op:// references must never appear in docs",
            "use sk- prefixed names sparingly",
            "a_very_long_snake_case_identifier_name_for_config",
            "/private/var/folders/x7/q1w2e3r4t5y6u7i8o9p0/T/.tmpAb3Xy9/.git",
        ];
        for text in fixtures {
            assert_eq!(scan(text), None, "{text}");
        }
    }

    #[test]
    fn check_names_the_field() {
        let err = check("evidence", "AKIAIOSFODNN7EXAMPLE").unwrap_err();
        assert!(err
            .to_string()
            .starts_with("evidence looks like it contains an AWS access key"));
    }
}
