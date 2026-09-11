use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
    sync::OnceLock,
};

const MAX_DIAGNOSTIC_TEXT_CHARS: usize = 1024;
const MAX_SPEECH_TEXT_CHARS: usize = 2048;
static INCLUDE_SENSITIVE_TEXT: OnceLock<bool> = OnceLock::new();

fn parse_opt_in(value: Option<String>) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

pub fn include_sensitive_text() -> bool {
    *INCLUDE_SENSITIVE_TEXT.get_or_init(|| {
        parse_opt_in(env::var("NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT").ok())
    })
}

fn sanitize_untrusted(value: &str, max_chars: usize) -> String {
    let mut output = String::with_capacity(value.len().min(max_chars));
    let mut truncated = false;

    for (index, character) in value.chars().enumerate() {
        if index >= max_chars {
            truncated = true;
            break;
        }
        if character.is_control() {
            output.push(' ');
        } else {
            output.push(character);
        }
    }

    if truncated {
        output.push('…');
    }
    output
}

fn one_line(value: &str) -> String {
    sanitize_untrusted(value, MAX_DIAGNOSTIC_TEXT_CHARS)
        .replace('\\', "\\\\")
        .replace('|', "\\|")
}

pub fn text(value: &str) -> String {
    if include_sensitive_text() {
        one_line(value)
    } else {
        format!("<redacted chars={}>", value.chars().count())
    }
}

pub fn optional_text(value: &str) -> String {
    if value.is_empty() {
        "<none>".to_string()
    } else {
        text(value)
    }
}

pub fn speech_text(value: &str) -> String {
    sanitize_untrusted(value, MAX_SPEECH_TEXT_CHARS)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn identity_key(parts: &[&str]) -> String {
    let mut hasher = DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

pub fn print_policy_marker() {
    if include_sensitive_text() {
        println!("DIAGNOSTIC_TEXT_POLICY = TEST_EVIDENCE_OPT_IN");
    } else {
        println!("DIAGNOSTIC_TEXT_POLICY = REDACTED_DEFAULT");
    }
    println!("PASSWORD_TEXT_POLICY = ALWAYS_REDACTED");
    println!(
        "UNTRUSTED_TEXT_LIMITS = speech_chars:{} diagnostic_chars:{}",
        MAX_SPEECH_TEXT_CHARS, MAX_DIAGNOSTIC_TEXT_CHARS
    );
}

#[cfg(test)]
mod tests {
    use super::{identity_key, parse_opt_in, sanitize_untrusted, speech_text};

    #[test]
    fn diagnostic_text_requires_explicit_opt_in() {
        assert!(!parse_opt_in(None));
        assert!(!parse_opt_in(Some("0".to_string())));
        assert!(!parse_opt_in(Some("false".to_string())));
        assert!(parse_opt_in(Some("1".to_string())));
        assert!(parse_opt_in(Some("TRUE".to_string())));
    }

    #[test]
    fn hostile_control_characters_are_neutralized() {
        assert_eq!(sanitize_untrusted("safe\u{0}name\nnext", 100), "safe name next");
        assert_eq!(speech_text("one\r\n\ttwo"), "one two");
    }

    #[test]
    fn hostile_text_is_bounded_by_character_count() {
        let sanitized = sanitize_untrusted(&"a".repeat(5000), 64);
        assert_eq!(sanitized.chars().count(), 65);
        assert!(sanitized.ends_with('…'));
    }

    #[test]
    fn identity_key_does_not_retain_plaintext() {
        let key = identity_key(&["private document title", "secret automation id"]);
        assert_eq!(key.len(), 16);
        assert!(!key.contains("private"));
        assert!(!key.contains("secret"));
    }
}
