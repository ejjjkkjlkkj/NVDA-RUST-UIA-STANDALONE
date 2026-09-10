pub const DEFAULT_MONITOR_SECONDS: u64 = 15;

pub fn parse_monitor_seconds(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MONITOR_SECONDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_value_uses_default() {
        assert_eq!(parse_monitor_seconds(None), DEFAULT_MONITOR_SECONDS);
    }

    #[test]
    fn invalid_value_uses_default() {
        assert_eq!(parse_monitor_seconds(Some("invalid")), DEFAULT_MONITOR_SECONDS);
    }

    #[test]
    fn valid_value_is_preserved() {
        assert_eq!(parse_monitor_seconds(Some("30")), 30);
    }

    #[test]
    fn zero_is_allowed_for_noninteractive_smoke_runs() {
        assert_eq!(parse_monitor_seconds(Some("0")), 0);
    }
}
