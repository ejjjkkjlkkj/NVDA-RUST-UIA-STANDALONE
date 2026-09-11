use std::{env, sync::OnceLock};

static INCLUDE_SENSITIVE_TEXT: OnceLock<bool> = OnceLock::new();

fn parse_opt_in(value: Option<String>) -> bool {
    matches!(
        value.as_deref().map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("1" | "true" | "yes")
    )
}

pub fn include_sensitive_text() -> bool {
    *INCLUDE_SENSITIVE_TEXT.get_or_init(|| {
        parse_opt_in(env::var("NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT").ok())
    })
}

fn one_line(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
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

pub fn print_policy_marker() {
    if include_sensitive_text() {
        println!("DIAGNOSTIC_TEXT_POLICY = TEST_EVIDENCE_OPT_IN");
    } else {
        println!("DIAGNOSTIC_TEXT_POLICY = REDACTED_DEFAULT");
    }
    println!("PASSWORD_TEXT_POLICY = ALWAYS_REDACTED");
}

#[cfg(test)]
mod tests {
    use super::parse_opt_in;

    #[test]
    fn diagnostic_text_requires_explicit_opt_in() {
        assert!(!parse_opt_in(None));
        assert!(!parse_opt_in(Some("0".to_string())));
        assert!(!parse_opt_in(Some("false".to_string())));
        assert!(parse_opt_in(Some("1".to_string())));
        assert!(parse_opt_in(Some("TRUE".to_string())));
    }
}
