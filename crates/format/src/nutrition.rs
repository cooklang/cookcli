//! The nutrients a recipe's `nutrition:` metadata can name.
//!
//! One table serves every reader of that metadata, so a recipe's keys mean the
//! same thing everywhere: the web page, the JSON-LD output of [`crate::schema`]
//! and the importers that write them. The canonical keys are the
//! [schema.org `NutritionInformation`](https://schema.org/NutritionInformation)
//! properties (`calories`, `fatContent`, `proteinContent`, ...), the very names
//! the JSON-LD output uses. The shorter spellings recipes were first written
//! with (`kcal`, `lipids`, `saturated-fat`, `carbohydrates`, `sugars`,
//! `fibers`, `proteins`) stay accepted as aliases. `salt` is the one key
//! schema.org has no property for, so it keeps its own name.

use serde_yaml::Value;

/// One nutrient: its canonical key, how it is measured and shown, and the
/// schema.org property it is exported as.
#[derive(Debug, PartialEq, Eq)]
pub struct Nutrient {
    /// The canonical key in a recipe's `nutrition:` mapping: a schema.org
    /// `NutritionInformation` property, or `salt`.
    pub name: &'static str,
    /// The schema.org property it is exported as: its own name, except `salt`,
    /// which is exported as `sodiumContent`, converted (see [`SALT_PER_SODIUM`]).
    pub schema: &'static str,
    /// Other spellings a recipe may use, compared without case, `-`, `_` or
    /// spaces.
    pub aliases: &'static [&'static str],
    /// The unit of a bare number.
    pub default_unit: &'static str,
    /// How many decimals the amount is shown with: 0 for energy, 1 for most,
    /// 2 for salt.
    pub decimals: usize,
    /// Whether this is the energy value.
    pub energy: bool,
    /// The nutrient this one is "of which", e.g. sugars under carbohydrates.
    pub parent: Option<&'static str>,
}

/// Salt in grams per gram of sodium. Importers that read sodium (USDA gives
/// it in mg) store salt in grams as `mg × 2.5 / 1000`; the JSON-LD output
/// goes the other way, since schema.org has `sodiumContent` and no salt.
pub const SALT_PER_SODIUM: f64 = 2.5;

/// Every nutrient a recipe can give, in the order a nutrition table lists
/// them.
pub const NUTRIENTS: &[Nutrient] = &[
    Nutrient {
        name: "calories",
        schema: "calories",
        aliases: &["kcal", "cal", "energy"],
        default_unit: "kcal",
        decimals: 0,
        energy: true,
        parent: None,
    },
    Nutrient {
        name: "fatContent",
        schema: "fatContent",
        aliases: &["lipids", "fat", "fats", "lipid"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "saturatedFatContent",
        schema: "saturatedFatContent",
        aliases: &["saturated-fat"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("fatContent"),
    },
    Nutrient {
        name: "unsaturatedFatContent",
        schema: "unsaturatedFatContent",
        aliases: &["unsaturated-fat"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("fatContent"),
    },
    Nutrient {
        name: "monounsaturatedFatContent",
        schema: "",
        aliases: &["monounsaturated-fat", "mono-unsaturated-fat"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("fatContent"),
    },
    Nutrient {
        name: "polyunsaturatedFatContent",
        schema: "",
        aliases: &["polyunsaturated-fat", "poly-unsaturated-fat"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("fatContent"),
    },
    Nutrient {
        name: "transFatContent",
        schema: "transFatContent",
        aliases: &["trans-fat"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("fatContent"),
    },
    Nutrient {
        name: "carbohydrateContent",
        schema: "carbohydrateContent",
        aliases: &["carbohydrates", "carbohydrate", "carbs"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "sugarContent",
        schema: "sugarContent",
        aliases: &["sugars", "sugar"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("carbohydrateContent"),
    },
    Nutrient {
        name: "starchContent",
        schema: "",
        aliases: &["starch"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("carbohydrateContent"),
    },
    Nutrient {
        name: "fiberContent",
        schema: "fiberContent",
        aliases: &["fibers", "fiber", "fibre", "fibres"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "proteinContent",
        schema: "proteinContent",
        aliases: &["proteins", "protein"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "salt",
        schema: "sodiumContent",
        aliases: &[],
        default_unit: "g",
        decimals: 2,
        energy: false,
        parent: None,
    },
];

/// Lowercases `name` and drops `-`, `_` and spaces, so `saturated_fat`,
/// `Saturated-Fat` and `saturatedFat` compare equal.
fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

impl Nutrient {
    /// Whether a recipe's key `field` names this nutrient, by its canonical
    /// key or any alias.
    pub fn matches(&self, field: &str) -> bool {
        let field = normalize(field);
        normalize(self.name) == field || self.aliases.iter().any(|a| normalize(a) == field)
    }

    /// The position in [`NUTRIENTS`], which is the order tables list them in.
    pub fn index(&self) -> usize {
        NUTRIENTS
            .iter()
            .position(|n| n.name == self.name)
            .unwrap_or(usize::MAX)
    }

    /// `value` rounded to this nutrient's decimals, with a `.` separator and
    /// no unit: `4.2`, `111`, `0.05`.
    pub fn format_amount(&self, value: f64) -> String {
        format!("{value:.*}", self.decimals)
    }
}

/// The nutrient a recipe's key `field` names, if it is one of [`NUTRIENTS`].
pub fn find(field: &str) -> Option<&'static Nutrient> {
    NUTRIENTS.iter().find(|n| n.matches(field))
}

/// A numeric YAML value or text such as `"4.2"` / `"45.3%g"`, as an amount and
/// the unit written after `%` (empty when none). A `,` decimal separator is
/// accepted.
pub fn amount_and_unit(value: &Value) -> Option<(f64, String)> {
    match value {
        Value::Number(n) => Some((n.as_f64()?, String::new())),
        Value::String(s) => {
            let (amount, unit) = match s.split_once('%') {
                Some((a, u)) => (a.trim(), u.trim()),
                None => (s.trim(), ""),
            };
            Some((amount.replace(',', ".").parse().ok()?, unit.to_string()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_ld_names_and_short_aliases_find_the_same_nutrient() {
        for (a, b) in [
            ("proteinContent", "proteins"),
            ("fatContent", "Lipids"),
            ("saturatedFatContent", "saturated_fat"),
            ("saturatedFatContent", "saturated-fat"),
            ("calories", "kcal"),
            ("fiberContent", "fibres"),
            ("carbohydrateContent", "carbs"),
            ("sugarContent", "sugars"),
            ("monounsaturatedFatContent", "mono-unsaturated_fat"),
            ("transFatContent", "trans-fat"),
            ("starchContent", "Starch"),
        ] {
            assert_eq!(find(a), find(b), "{a} / {b}");
            assert!(find(a).is_some(), "{a}");
        }
    }

    #[test]
    fn the_canonical_name_is_the_json_ld_property() {
        for n in NUTRIENTS
            .iter()
            .filter(|n| n.name != "salt" && !n.schema.is_empty())
        {
            assert_eq!(n.name, n.schema);
        }
        // Not in schema.org: shown, never exported.
        for name in [
            "monounsaturatedFatContent",
            "polyunsaturatedFatContent",
            "starchContent",
        ] {
            assert_eq!(find(name).unwrap().schema, "");
        }
        assert_eq!(find("salt").unwrap().schema, "sodiumContent");
    }

    #[test]
    fn unknown_names_are_not_nutrients() {
        assert!(find("allergens").is_none());
        assert!(find("sodium").is_none());
    }

    #[test]
    fn amounts_follow_the_rounding_of_each_nutrient() {
        assert_eq!(find("kcal").unwrap().format_amount(110.6), "111");
        assert_eq!(find("lipids").unwrap().format_amount(12.14), "12.1");
        assert_eq!(find("fatContent").unwrap().format_amount(2.0), "2.0");
        assert_eq!(find("salt").unwrap().format_amount(0.05), "0.05");
    }

    #[test]
    fn parents_name_a_nutrient_of_the_table() {
        for n in NUTRIENTS {
            if let Some(p) = n.parent {
                assert!(NUTRIENTS.iter().any(|x| x.name == p), "{}", n.name);
            }
        }
    }

    #[test]
    fn amount_and_unit_reads_numbers_and_text() {
        let y = |s: &str| serde_yaml::from_str::<Value>(s).unwrap();
        assert_eq!(amount_and_unit(&y("4.2")), Some((4.2, String::new())));
        assert_eq!(amount_and_unit(&y("\"45,3%g\"")), Some((45.3, "g".into())));
        assert_eq!(amount_and_unit(&y("[1]")), None);
    }
}
