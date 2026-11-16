//! Human-friendly rendering of quantity numbers.
//!
//! Every human-readable output goes through here -- the web UI, the terminal,
//! Markdown, LaTeX, Typst, schema.org text and the shopping lists -- so a
//! recipe shows the same numbers everywhere: `0.5 cup` reads as `1/2 cup` and
//! `1.625 cup` as `1 5/8 cup`, while a value that is not one of those
//! fractions stays a decimal rather than being rounded to the nearest one.
//! JSON, YAML and the Cooklang output keep the plain number.
//!
//! Fractions are for what is measured with cups and spoons, or counted.
//! Metric units, times and temperatures read in decimals -- `100.625 g`, not
//! `100 5/8 g` -- as in cooklang's own unit database (`[fractions]` in its
//! `units.toml`: metric, time and temperature off, imperial on).
//!
//! The shopping list page renders its quantities in the browser with a copy
//! of these rules (`formatNumber` in `templates/shopping_list.html`); change
//! both together.

use cooklang::quantity::Quantity;

/// Formats a floating-point number as a human-readable string with fractions
/// Based on the approach from cooklang-rs/bindings/src/lib.rs
pub fn format_number(value: f64) -> String {
    format_amount(value, true)
}

/// Formats a number as [`format_number`] does, but always in decimals.
pub fn format_decimal(value: f64) -> String {
    format_amount(value, false)
}

fn format_amount(value: f64, fractions: bool) -> String {
    // Round to reasonable precision to handle floating point errors
    // This handles cases like 0.89999999999 -> 0.9
    let rounded = (value * 1000000.0).round() / 1000000.0;

    // Check if it's effectively a whole number
    if (rounded.fract()).abs() < 0.0000001 {
        return format!("{rounded:.0}");
    }

    // Try to convert to a common fraction
    if fractions {
        if let Some(fraction) = decimal_to_fraction(rounded) {
            return fraction;
        }
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
                return Some(format!("{whole:.0} {fraction_str}"));
            } else {
                return Some(fraction_str.to_string());
            }
        }
    }

    None
}

/// Units read in decimals: metric, time and temperature. Compared ignoring
/// case and a trailing dot. A bare `c` or `f` is not here: `c` is also a cup.
const DECIMAL_UNITS: &[&str] = &[
    // Mass
    "mg",
    "milligram",
    "milligrams",
    "milligramme",
    "milligrammes",
    "g",
    "gr",
    "gram",
    "grams",
    "gramme",
    "grammes",
    "kg",
    "kilo",
    "kilos",
    "kilogram",
    "kilograms",
    "kilogramme",
    "kilogrammes",
    // Volume
    "ml",
    "milliliter",
    "milliliters",
    "millilitre",
    "millilitres",
    "cl",
    "centiliter",
    "centiliters",
    "centilitre",
    "centilitres",
    "dl",
    "deciliter",
    "deciliters",
    "decilitre",
    "decilitres",
    "l",
    "liter",
    "liters",
    "litre",
    "litres",
    // Time
    "s",
    "sec",
    "secs",
    "second",
    "seconds",
    "min",
    "mins",
    "minute",
    "minutes",
    "h",
    "hr",
    "hrs",
    "hour",
    "hours",
    "day",
    "days",
    // Temperature
    "°c",
    "°f",
    "celsius",
    "fahrenheit",
];

/// Whether an amount in `unit` reads in fractions: no unit, cups, spoons,
/// pounds, cloves -- anything but the [`DECIMAL_UNITS`].
pub fn unit_uses_fractions(unit: Option<&str>) -> bool {
    let Some(unit) = unit else {
        return true;
    };
    let unit = unit.trim().trim_end_matches('.').to_lowercase();
    !DECIMAL_UNITS.contains(&unit.as_str())
}

/// Formats a quantity value for display, in fractions where they apply. Use
/// [`format_quantity_value`] when the unit is at hand, so that grams and
/// litres stay in decimals.
pub fn format_quantity(value: &cooklang::Value) -> Option<String> {
    format_value(value, true)
}

/// Formats a quantity's value, without its unit, the way its unit reads:
/// `1 5/8` for cups, `100.625` for grams.
pub fn format_quantity_value(qty: &Quantity) -> Option<String> {
    format_value(qty.value(), unit_uses_fractions(qty.unit()))
}

fn format_value(value: &cooklang::Value, fractions: bool) -> Option<String> {
    match value {
        cooklang::Value::Number(n) => Some(format_amount(n.value(), fractions)),
        cooklang::Value::Range { start, end } => Some(format!(
            "{} - {}",
            format_amount(start.value(), fractions),
            format_amount(end.value(), fractions)
        )),
        cooklang::Value::Text(s) => Some(s.clone()),
    }
}

/// Formats a quantity and its unit for display: `"1 5/8 cup"`, `"100.625 g"`,
/// or `"3"` for one without a unit.
pub fn format_quantity_with_unit(qty: &Quantity) -> String {
    let value = format_quantity_value(qty).unwrap_or_default();
    match qty.unit() {
        Some(unit) => format!("{value} {unit}"),
        None => value,
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

        // Above 1 they keep the whole part: a mixed number
        assert_eq!(format_number(1.5), "1 1/2");
        assert_eq!(format_number(2.25), "2 1/4");
        assert_eq!(format_number(1.75), "1 3/4");
        assert_eq!(format_number(2.333333), "2 1/3");
        assert_eq!(format_number(1.625), "1 5/8");
    }

    /// The bug in #432: a value that is not one of the fractions must not be
    /// shown as the nearest one. It stays a decimal.
    #[test]
    fn values_between_fractions_stay_decimals() {
        assert_eq!(format_number(1.6), "1.6");
        assert_eq!(format_number(0.3), "0.3");
        assert_eq!(format_number(0.33), "0.33");
        // 5/8 scaled by 1.5 is 15/16: not in the list.
        assert_eq!(format_number(0.9375), "0.938");
        assert_eq!(format_number(2.4375), "2.438");
    }

    #[test]
    fn quantities_with_units() {
        let parsed = crate::test_support::parse_recipe(
            "Mix @milk{1 5/8%cup}, @eggs{3} and @salt{a pinch}.\n",
            "milk",
            1.0,
        )
        .expect("parses");
        let shown: Vec<String> = parsed
            .value
            .ingredients
            .iter()
            .map(|i| format_quantity_with_unit(i.quantity.as_ref().unwrap()))
            .collect();
        assert_eq!(shown, ["1 5/8 cup", "3", "a pinch"]);
    }

    /// Grams, litres, times and temperatures read in decimals, as cooklang's
    /// unit database has it; cups, spoons, counts and unknown units keep the
    /// fractions.
    #[test]
    fn metric_time_and_temperature_stay_decimals() {
        let parsed = crate::test_support::parse_recipe(
            "Add @flour{100.625%g}, @butter{0.5%kg}, @milk{0.25%l}, @oil{1.5%Tbsp}, \
             @garlic{2.5%cloves}, @eggs{1.5}, @sugar{0.75%cup} and \
             @water{80.5%°C}, rest for ~{1.5%hours}.\n",
            "flour",
            1.0,
        )
        .expect("parses");
        let recipe = &parsed.value;
        let shown: Vec<String> = recipe
            .ingredients
            .iter()
            .filter_map(|i| i.quantity.as_ref().map(format_quantity_with_unit))
            .chain(
                recipe
                    .timers
                    .iter()
                    .filter_map(|t| t.quantity.as_ref().map(format_quantity_with_unit)),
            )
            .collect();
        assert_eq!(
            shown,
            [
                "100.625 g",
                "0.5 kg",
                "0.25 l",
                "1 1/2 Tbsp",
                "2 1/2 cloves",
                "1 1/2",
                "3/4 cup",
                "80.5 °C",
                "1.5 hours",
            ]
        );
    }

    #[test]
    fn which_units_use_fractions() {
        for unit in [
            None,
            Some("cup"),
            Some("c"),
            Some("tsp"),
            Some("lb"),
            Some("pinch"),
        ] {
            assert!(unit_uses_fractions(unit), "{unit:?}");
        }
        for unit in ["g", "Kg", "ml", "L", "cl", "min", "hours", "°C", "gr."] {
            assert!(!unit_uses_fractions(Some(unit)), "{unit}");
        }
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
            Some("1/2 - 2 1/4")
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
