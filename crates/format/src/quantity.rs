//! Deterministic ordering for [`GroupedQuantity`].
//!
//! Everything in this crate that renders or serialises a grouped quantity goes
//! through [`ordered_components`] or [`grouped_quantity_fmt`], so the human
//! table, Markdown, JSON, YAML, LaTeX, Typst, schema.org and library outputs
//! all agree on one order.

use cooklang::quantity::{GroupedQuantity, Quantity};

/// The components of `grouped`, ordered by unit name, the unitless component
/// first.
///
/// # The rule
///
/// **Components are ordered by unit name, with the unitless component first —
/// it counts as an empty unit name — and components sharing a unit keep the
/// order they were added.**
///
/// # Why this exists
///
/// A [`GroupedQuantity`] holds the quantities that could not be added into one
/// another: `1 cup` of flour plus `100 g` of flour stays two components,
/// because no conversion between them exists. It keeps the ones whose unit the
/// converter does not know in a [`HashMap`](std::collections::HashMap) keyed by
/// unit name, and Rust randomises `HashMap` iteration order per process, so
/// [`GroupedQuantity::iter`] — and the `Display` impl built on it — yield those
/// components in a different order on every run:
///
/// ```text
/// flour 1 cup, 100 g
/// flour 100 g, 1 cup
/// ```
///
/// This became visible when `cooklang`'s `bundled_units` feature was switched
/// off so quantities keep the units they were authored in: without the unit
/// database no unit is "known", so every component lands in that map and any
/// ingredient measured two ways prints in random order.
///
/// The order the units were *written* in cannot be recovered here — the map has
/// already lost it — so ordering by unit name is chosen instead: it is
/// implementable from the data at hand, identical on every run and platform
/// (byte-wise comparison of the unit text, no locale involved), and short
/// enough to explain in one sentence.
pub fn ordered_components(grouped: &GroupedQuantity) -> Vec<&Quantity> {
    let mut components: Vec<&Quantity> = grouped.iter().collect();
    // `sort_by` is stable, so components sharing a unit — which `cooklang`
    // keeps apart only when it could not add them, such as a text value — stay
    // in the order they were added.
    components.sort_by(|a, b| unit_key(a).cmp(unit_key(b)));
    components
}

/// Render `grouped` the way its own `Display` impl does — the components joined
/// with `", "` — but in [`ordered_components`] order.
pub fn grouped_quantity_fmt(grouped: &GroupedQuantity) -> String {
    ordered_components(grouped)
        .into_iter()
        .map(|q| q.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A component's sort key: its unit, or `""` when it has none, which is what
/// puts the unitless component first.
fn unit_key(qty: &Quantity) -> &str {
    qty.unit().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::PARSER;

    /// Build a group by adding each quantity in turn, the way the shopping
    /// list and the recipe formatters do.
    fn group(quantities: &[Quantity]) -> GroupedQuantity {
        let mut grouped = GroupedQuantity::empty();
        for qty in quantities {
            grouped.add(qty, PARSER.converter());
        }
        grouped
    }

    fn qty(value: f64, unit: Option<&str>) -> Quantity {
        Quantity::new(
            cooklang::quantity::Value::Number(value.into()),
            unit.map(|u| u.to_string()),
        )
    }

    /// The bug this module exists for: two units that cannot be added must
    /// render in the same order every time, whichever order they arrived in.
    ///
    /// A single run could pass by luck, so both input orders are checked and
    /// the exact rendered string is asserted rather than "one of two orders".
    #[test]
    fn inconvertible_units_render_in_unit_name_order() {
        let cup_first = group(&[qty(1.0, Some("cup")), qty(100.0, Some("g"))]);
        let gram_first = group(&[qty(100.0, Some("g")), qty(1.0, Some("cup"))]);

        assert_eq!(grouped_quantity_fmt(&cup_first), "1 cup, 100 g");
        assert_eq!(grouped_quantity_fmt(&gram_first), "1 cup, 100 g");
    }

    /// Repeat the group often enough that a `HashMap` iterating in insertion
    /// order by chance cannot hide an unsorted implementation.
    ///
    /// The units here are ones no converter knows, so every component lands in
    /// the randomised map whether or not `cooklang`'s unit database is
    /// compiled in.
    #[test]
    fn the_order_does_not_vary_between_groups() {
        let rendered: Vec<String> = (0..64)
            .map(|_| {
                grouped_quantity_fmt(&group(&[
                    qty(1.0, Some("sprig")),
                    qty(2.0, Some("clove")),
                    qty(3.0, Some("knob")),
                    qty(4.0, Some("glug")),
                ]))
            })
            .collect();

        assert!(
            rendered
                .iter()
                .all(|r| r == "2 clove, 4 glug, 3 knob, 1 sprig"),
            "every group must render identically: {rendered:?}"
        );
    }

    #[test]
    fn the_unitless_component_comes_first() {
        let grouped = group(&[qty(2.0, Some("g")), qty(3.0, None), qty(1.0, Some("cup"))]);
        assert_eq!(grouped_quantity_fmt(&grouped), "3, 1 cup, 2 g");
    }

    #[test]
    fn an_empty_group_renders_as_nothing() {
        assert_eq!(grouped_quantity_fmt(&GroupedQuantity::empty()), "");
        assert!(ordered_components(&GroupedQuantity::empty()).is_empty());
    }
}
