//! Human-friendly rendering of quantity numbers.
//!
//! Used by the web UI and menu handlers, where `0.5` should read as `1/2`
//! rather than as a float.

/// Formats a floating-point number as a human-readable string with fractions
/// Based on the approach from cooklang-rs/bindings/src/lib.rs
pub fn format_number(value: f64) -> String {
    // Round to reasonable precision to handle floating point errors
    // This handles cases like 0.89999999999 -> 0.9
    let rounded = (value * 1000000.0).round() / 1000000.0;

    // Check if it's effectively a whole number
    if (rounded.fract()).abs() < 0.0000001 {
        return format!("{rounded:.0}");
    }

    // Try to convert to a common fraction
    if let Some(fraction) = decimal_to_fraction(rounded) {
        return fraction;
    }

    // For decimals, determine appropriate precision
    // Round to at most 3 decimal places, but remove trailing zeros
    let rounded_to_3 = (rounded * 1000.0).round() / 1000.0;

    // Format with appropriate precision
    let mut result = if (rounded_to_3 * 100.0).fract().abs() < 0.001 {
        // Has at most 2 decimal places
        format!("{rounded_to_3:.2}")
    } else {
        // Needs 3 decimal places
        format!("{rounded_to_3:.3}")
    };

    // Remove trailing zeros and decimal point if not needed
    if result.contains('.') {
        result = result
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
    }

    result
}

/// Converts common decimal values to fraction strings
fn decimal_to_fraction(value: f64) -> Option<String> {
    const EPSILON: f64 = 0.0001;

    // Negatives take the decimal path. `floor` rounds away from zero, so
    // `-0.25` would otherwise give `fract == 0.75` and render as "3/4" —
    // sign and magnitude both lost, and the `whole > 0.0` guard below never
    // fires to catch it.
    if value < 0.0 {
        return None;
    }

    // Split into whole and fractional parts
    let whole = value.floor();
    let fract = value - whole;

    // Common fractions and their decimal equivalents
    let common_fractions = [
        (0.125, "1/8"),
        (0.25, "1/4"),
        (0.333333, "1/3"),
        (0.375, "3/8"),
        (0.5, "1/2"),
        (0.625, "5/8"),
        (0.666667, "2/3"),
        (0.75, "3/4"),
        (0.875, "7/8"),
    ];

    // Check if the fractional part matches any common fraction
    for &(decimal, fraction_str) in &common_fractions {
        if (fract - decimal).abs() < EPSILON {
            if whole > 0.0 {
                // For values > 1, return decimal format instead of mixed fraction
                return None;
            } else {
                return Some(fraction_str.to_string());
            }
        }
    }

    None
}

/// Formats a quantity value for display
pub fn format_quantity(value: &cooklang::Value) -> Option<String> {
    match value {
        cooklang::Value::Number(n) => Some(format_number(n.value())),
        cooklang::Value::Range { start, end } => Some(format!(
            "{} - {}",
            format_number(start.value()),
            format_number(end.value())
        )),
        cooklang::Value::Text(s) => Some(s.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_fractions() {
        // Fractions less than 1 should be displayed as fractions
        assert_eq!(format_number(0.5), "1/2");
        assert_eq!(format_number(0.25), "1/4");
        assert_eq!(format_number(0.75), "3/4");
        assert_eq!(format_number(0.333333), "1/3");
        assert_eq!(format_number(0.666667), "2/3");

        // Values greater than 1 should be displayed as decimals
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(2.25), "2.25");
        assert_eq!(format_number(1.75), "1.75");
        assert_eq!(format_number(2.333333), "2.333");
    }

    #[test]
    fn test_format_whole_numbers() {
        assert_eq!(format_number(2.0), "2");
        assert_eq!(format_number(1.9999999999), "2");
    }

    /// Negatives used to be mangled: `floor` rounds away from zero, so the
    /// fractional part of `-0.25` came out as `0.75` and rendered as "3/4",
    /// dropping the sign and changing the magnitude. They now take the
    /// decimal path.
    #[test]
    fn negatives_keep_their_sign_and_magnitude() {
        assert_eq!(format_number(-0.5), "-0.5");
        assert_eq!(format_number(-0.25), "-0.25");
        assert_eq!(format_number(-0.75), "-0.75");
        assert_eq!(format_number(-1.5), "-1.5");
        assert_eq!(format_number(-2.333333), "-2.333");
        // Whole negatives never reached the fraction path, but pin them too.
        assert_eq!(format_number(-2.0), "-2");
        assert_eq!(format_number(-1.9999999999), "-2");
    }

    /// `format_quantity` is the entry point the web UI actually calls.
    #[test]
    fn quantities_render_numbers_ranges_and_text() {
        use cooklang::Value;
        let num = |v: f64| Value::Number(v.into());
        assert_eq!(format_quantity(&num(0.5)).as_deref(), Some("1/2"));
        assert_eq!(format_quantity(&num(-0.5)).as_deref(), Some("-0.5"));
        assert_eq!(
            format_quantity(&Value::Range {
                start: 0.5.into(),
                end: 2.25.into()
            })
            .as_deref(),
            Some("1/2 - 2.25")
        );
        assert_eq!(
            format_quantity(&Value::Text("a pinch".into())).as_deref(),
            Some("a pinch")
        );
    }

    #[test]
    fn test_format_decimals() {
        assert_eq!(format_number(1.23), "1.23");
        assert_eq!(format_number(0.899), "0.899");
        assert_eq!(format_number(0.89999999999), "0.9");
        assert_eq!(format_number(0.30000000001), "0.3");
    }
}
