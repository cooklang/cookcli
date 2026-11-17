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
    /// The nutrient this one is "of which" in the older, norm-less tree. The page no
    /// longer reads it: the tree comes from the standard in use, which differs
    /// between norms (the fibre sits next to the carbohydrates in the EU, inside
    /// them in the US).
    pub parent: Option<&'static str>,
}

/// An amount in micrograms, from its unit: `g`, `mg`, `µg` (also `ug`, `mcg`, `μg`).
/// `None` for any other unit (kcal, kJ, a percentage...).
pub fn to_micrograms(value: f64, unit: &str) -> Option<f64> {
    let factor = match unit.trim().to_lowercase().as_str() {
        "g" => 1e6,
        "mg" => 1e3,
        "µg" | "ug" | "mcg" | "μg" => 1.0,
        _ => return None,
    };
    Some(value * factor)
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
        parent: Some("unsaturatedFatContent"),
    },
    Nutrient {
        name: "polyunsaturatedFatContent",
        schema: "",
        aliases: &["polyunsaturated-fat", "poly-unsaturated-fat"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: Some("unsaturatedFatContent"),
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
    // Nutrients below carry no `parent`: how they nest is the business of the
    // standard in use (see `crate::reference`), which differs between norms.
    Nutrient {
        name: "cholesterolContent",
        schema: "cholesterolContent",
        aliases: &["cholesterol"],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "sodiumContent",
        schema: "sodiumContent",
        aliases: &["sodium"],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "addedSugars",
        schema: "",
        aliases: &["added-sugars", "added-sugar"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "polyols",
        schema: "",
        aliases: &["polyol"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "omega3",
        schema: "",
        aliases: &["omega-3"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "omega6",
        schema: "",
        aliases: &["omega-6"],
        default_unit: "g",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "calcium",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "phosphorus",
        schema: "",
        aliases: &["phosphorous"],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "magnesium",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "potassium",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "chloride",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "iron",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "zinc",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "copper",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "manganese",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "fluoride",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "selenium",
        schema: "",
        aliases: &[],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "chromium",
        schema: "",
        aliases: &[],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "molybdenum",
        schema: "",
        aliases: &[],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "iodine",
        schema: "",
        aliases: &["iodide"],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminA",
        schema: "",
        aliases: &["vitamin-a"],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminD",
        schema: "",
        aliases: &["vitamin-d"],
        default_unit: "µg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminE",
        schema: "",
        aliases: &["vitamin-e"],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminK",
        schema: "",
        aliases: &["vitamin-k"],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminC",
        schema: "",
        aliases: &["vitamin-c"],
        default_unit: "mg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "thiamin",
        schema: "",
        aliases: &["thiamine", "vitamin-b1"],
        default_unit: "mg",
        decimals: 2,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "riboflavin",
        schema: "",
        aliases: &["vitamin-b2"],
        default_unit: "mg",
        decimals: 2,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "niacin",
        schema: "",
        aliases: &["vitamin-b3"],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "pantothenicAcid",
        schema: "",
        aliases: &["vitamin-b5", "pantothenic-acid"],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminB6",
        schema: "",
        aliases: &["vitamin-b6"],
        default_unit: "mg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "biotin",
        schema: "",
        aliases: &["vitamin-b8", "vitamin-b7"],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "folate",
        schema: "",
        aliases: &["folic-acid", "vitamin-b9"],
        default_unit: "µg",
        decimals: 0,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "vitaminB12",
        schema: "",
        aliases: &["vitamin-b12"],
        default_unit: "µg",
        decimals: 1,
        energy: false,
        parent: None,
    },
    Nutrient {
        name: "choline",
        schema: "",
        aliases: &[],
        default_unit: "mg",
        decimals: 0,
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

    /// How many levels this nutrient sits below a top-level one: 0 for fat,
    /// 1 for saturated fat, 2 for monounsaturated fat.
    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut name = self.parent;
        // The bound only guards against a loop in the table, which a test rules out.
        while let Some(parent) = name.filter(|_| depth < 8) {
            depth += 1;
            name = NUTRIENTS
                .iter()
                .find(|n| n.name == parent)
                .and_then(|n| n.parent);
        }
        depth
    }

    /// The top-level nutrient this one belongs to: `fatContent` for every kind of
    /// fat, `carbohydrateContent` for sugars and starch, itself when it has no parent.
    pub fn root(&self) -> &'static str {
        let mut name = self.name;
        for _ in 0..8 {
            match NUTRIENTS
                .iter()
                .find(|n| n.name == name)
                .and_then(|n| n.parent)
            {
                Some(parent) => name = parent,
                None => break,
            }
        }
        name
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
        assert!(find("sugar-free").is_none());
    }

    #[test]
    fn amounts_follow_the_rounding_of_each_nutrient() {
        assert_eq!(find("kcal").unwrap().format_amount(110.6), "111");
        assert_eq!(find("lipids").unwrap().format_amount(12.14), "12.1");
        assert_eq!(find("fatContent").unwrap().format_amount(2.0), "2.0");
        assert_eq!(find("salt").unwrap().format_amount(0.05), "0.05");
    }

    #[test]
    fn the_nutrients_form_a_tree() {
        for (index, n) in NUTRIENTS.iter().enumerate() {
            let Some(parent) = n.parent else { continue };
            let parent_index = NUTRIENTS
                .iter()
                .position(|x| x.name == parent)
                .unwrap_or_else(|| panic!("{} names an unknown parent {parent}", n.name));
            assert!(
                parent_index < index,
                "{parent} must be listed before {}",
                n.name
            );
            assert!(n.depth() <= 3, "{} sits too deep", n.name);
        }
    }

    #[test]
    fn depth_and_root_follow_the_tree() {
        assert_eq!(find("fat").unwrap().depth(), 0);
        assert_eq!(find("saturated-fat").unwrap().depth(), 1);
        assert_eq!(find("monounsaturatedFatContent").unwrap().depth(), 2);
        assert_eq!(
            find("monounsaturatedFatContent").unwrap().root(),
            "fatContent"
        );
        assert_eq!(find("starch").unwrap().root(), "carbohydrateContent");
        assert_eq!(find("fibers").unwrap().root(), "fiberContent");
        assert_eq!(find("proteins").unwrap().root(), "proteinContent");
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

    #[test]
    fn the_nutrients_of_every_standard_are_known_to_the_table() {
        // every nutrient a standard's tree names can be read from a recipe
        for (id, standard) in crate::reference::standards() {
            for node in standard.tree.iter().filter(|n| !n.group) {
                assert!(
                    find(&node.name).is_some(),
                    "{id}: {} is not in the table",
                    node.name
                );
            }
        }
    }

    #[test]
    fn units_convert_to_micrograms() {
        assert_eq!(to_micrograms(1.5, "mg"), Some(1500.0));
        assert_eq!(to_micrograms(2.0, "g"), Some(2e6));
        assert_eq!(to_micrograms(12.0, "µg"), Some(12.0));
        assert_eq!(to_micrograms(12.0, "ug"), Some(12.0));
        assert_eq!(to_micrograms(120.0, "kcal"), None);
    }

    #[test]
    fn salt_per_sodium_is_the_one_of_every_standard() {
        for (id, standard) in crate::reference::standards() {
            assert_eq!(
                standard.conventions.salt_per_sodium, SALT_PER_SODIUM,
                "{id}"
            );
        }
    }
}
