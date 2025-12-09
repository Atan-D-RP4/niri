//! Shared parsing utilities for size and position changes.
//!
//! This module provides common parsing logic for size/position change strings
//! like "+10%", "-5%", "50%", "+100", "-50", "800".
//!
//! Used by both `action_proxy.rs` (for runtime IPC actions) and
//! `config_converter.rs` (for configuration parsing).

use niri_ipc::SizeChange;

/// Parse a size change string.
///
/// Supported formats:
/// - `"+10%"` - Adjust proportion by +10%
/// - `"-5%"` - Adjust proportion by -5%
/// - `"50%"` - Set proportion to 50%
/// - `"+100"` - Adjust fixed size by +100 pixels
/// - `"-50"` - Adjust fixed size by -50 pixels
/// - `"800"` - Set fixed size to 800 pixels
///
/// Returns `None` if the string cannot be parsed.
pub fn parse_size_change(s: &str) -> Option<SizeChange> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let is_relative = s.starts_with('+') || s.starts_with('-');
    let is_proportion = s.ends_with('%');

    let num_str = s
        .trim_start_matches('+')
        .trim_start_matches('-')
        .trim_end_matches('%');

    if is_proportion {
        let value: f64 = num_str.parse().ok()?;
        let proportion = value / 100.0;
        if is_relative {
            if s.starts_with('-') {
                Some(SizeChange::AdjustProportion(-proportion))
            } else {
                Some(SizeChange::AdjustProportion(proportion))
            }
        } else {
            Some(SizeChange::SetProportion(proportion))
        }
    } else {
        let value: i32 = num_str.parse().ok()?;
        if is_relative {
            if s.starts_with('-') {
                Some(SizeChange::AdjustFixed(-value))
            } else {
                Some(SizeChange::AdjustFixed(value))
            }
        } else {
            Some(SizeChange::SetFixed(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_change_percentage_adjust() {
        assert_eq!(
            parse_size_change("+10%"),
            Some(SizeChange::AdjustProportion(0.1))
        );
        assert_eq!(
            parse_size_change("-5%"),
            Some(SizeChange::AdjustProportion(-0.05))
        );
    }

    #[test]
    fn test_parse_size_change_percentage_set() {
        assert_eq!(
            parse_size_change("50%"),
            Some(SizeChange::SetProportion(0.5))
        );
        assert_eq!(
            parse_size_change("100%"),
            Some(SizeChange::SetProportion(1.0))
        );
    }

    #[test]
    fn test_parse_size_change_fixed_adjust() {
        assert_eq!(parse_size_change("+100"), Some(SizeChange::AdjustFixed(100)));
        assert_eq!(parse_size_change("-50"), Some(SizeChange::AdjustFixed(-50)));
    }

    #[test]
    fn test_parse_size_change_fixed_set() {
        assert_eq!(parse_size_change("800"), Some(SizeChange::SetFixed(800)));
        assert_eq!(parse_size_change("1920"), Some(SizeChange::SetFixed(1920)));
    }

    #[test]
    fn test_parse_size_change_with_whitespace() {
        assert_eq!(
            parse_size_change("  +10%  "),
            Some(SizeChange::AdjustProportion(0.1))
        );
        assert_eq!(
            parse_size_change(" 800 "),
            Some(SizeChange::SetFixed(800))
        );
    }

    #[test]
    fn test_parse_size_change_invalid() {
        assert_eq!(parse_size_change(""), None);
        assert_eq!(parse_size_change("abc"), None);
        assert_eq!(parse_size_change("10px"), None);
        assert_eq!(parse_size_change("%"), None);
        assert_eq!(parse_size_change("+"), None);
    }
}
