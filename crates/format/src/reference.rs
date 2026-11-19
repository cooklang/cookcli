//! One file per standard (EU, US, UK), read from `data/standards/*.yaml`.
//!
//! A standard says how a label is read under that norm: the tree of the
//! nutrients (what nests in what), the daily reference intakes and the
//! thresholds that rate a nutrient. The files carry their sources and say what
//! has been checked against the printed text. Nothing here decides which
//! standard applies; callers pick one by id.

use std::collections::BTreeMap;

use crate::nutrition::to_micrograms;
use std::sync::OnceLock;

use serde::Deserialize;

const FILES: &[&str] = &[
    include_str!("../data/standards/eu.yaml"),
    include_str!("../data/standards/us.yaml"),
    include_str!("../data/standards/uk.yaml"),
];

/// A daily amount, in `unit` (`g`, `mg` or `ug`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Amount {
    /// The quantity.
    pub amount: f64,
    /// Its unit: `g`, `mg` or `ug`.
    pub unit: String,
}

/// One node of the tree: a nutrient, or a group (`minerals`, `vitamins`) that
/// only holds others.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Node {
    /// The nutrient's canonical name, or the group's.
    pub name: String,
    #[serde(default)]
    /// A group only holds others: it has no amount.
    pub group: bool,
    /// The node it nests under, listed before it.
    pub parent: Option<String>,
    /// Where it stands in the norm: `mandatory`, `voluntary`, `derived` or `extra`.
    pub declared: Option<String>,
    /// `listed` when the label lists it next to its parent without it being a part of it
    /// (cholesterol sits with the fats, but is not a fat); a table then writes no "of which".
    pub relation: Option<String>,
}

/// Energy per gram of a substance.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EnergyFactor {
    /// Kilojoules per gram.
    pub kj: f64,
    /// Kilocalories per gram.
    pub kcal: f64,
}

/// A nutrition claim and the condition a food meets to carry it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Claim {
    /// The claim, such as `low-fat`.
    pub claim: String,
    /// The nutrient the condition is on.
    pub nutrient: String,
    /// What the figures are per: `100g`, `100ml` or `energy-share`.
    pub per: String,
    /// At most this much, for solids.
    pub max: Option<f64>,
    /// At most this much per 100 ml, for liquids.
    pub liquid_max: Option<f64>,
    /// At least this much.
    pub min: Option<f64>,
    /// Or at least this much per 100 kcal.
    pub min_per_100kcal: Option<f64>,
    /// The unit of the figures.
    pub unit: String,
    /// What else the regulation requires.
    #[serde(default)]
    pub note: String,
}

/// What counts as a vitamin or mineral that is well present in a food.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SignificantAmount {
    /// What the share is taken of: `per-100g` (a share of the reference intake supplied by 100 g)
    /// or `per-serving` (a share of the daily value in one serving).
    pub basis: String,
    /// From this percent a food is a "source of", when the standard has that step.
    pub source_percent: Option<f64>,
    /// From this percent a food is "high in".
    pub high_percent: f64,
    /// The share a beverage needs per 100 ml, when it differs.
    pub beverage_percent_per_100ml: Option<f64>,
    /// The share a single-portion package needs, when it differs.
    pub single_portion_percent: Option<f64>,
}

/// What the norm, or the health bodies it leans on, say to get less of and more of.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Direction {
    /// Where the lists come from.
    pub source: String,
    /// Nutrients to get less of: a high amount is never good news.
    pub limit: Vec<String>,
    /// Nutrients to get more of.
    pub encourage: Vec<String>,
}

/// How the standard builds its tree.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Conventions {
    /// Whether the carbohydrate figure includes the fibre (the US) or leaves it out (the EU).
    pub fibre_in_carbohydrate: bool,
    /// What the label gives for salt: `salt` or `sodium`.
    pub declares: String,
    /// Grams of salt in one gram of sodium: salt = sodium x this.
    pub salt_per_sodium: f64,
}

/// What has been checked against the printed source: `verified`, `to-verify`
/// or `partly-verified`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Status {
    /// Whether the reference intakes were checked against the printed source.
    pub intakes: String,
    /// Whether the tree was.
    pub tree: String,
    /// Whether the rating thresholds were.
    pub rating: String,
}

/// A threshold per 100 g: at or under `low` is low, over `high` is high.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Threshold {
    /// At or under this is low.
    pub low: f64,
    /// Over this is high.
    pub high: f64,
    /// The unit of both.
    pub unit: String,
}

/// How a nutrient is rated under the standard.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Rating {
    /// No rating yet (the EU has claims, not traffic lights).
    None {
        #[serde(default)]
        /// What is still missing.
        note: String,
    },
    /// Thresholds per 100 g, as the UK traffic lights.
    TrafficLight {
        /// What the thresholds are per, `per-100g`.
        basis: String,
        /// The thresholds, by nutrient.
        nutrients: BTreeMap<String, Threshold>,
        /// Thresholds not yet confirmed by a source that was read.
        #[serde(default)]
        unverified: Vec<String>,
    },
    /// A nutrient is coloured by the nutrition claims it earns (`Standard::claims`), as in the EU.
    Claims {},
    /// A share of the daily value per serving: low up to `low_percent`, high from `high_percent`.
    PercentDv {
        /// Up to this share of the daily value is low.
        low_percent: f64,
        /// From this share is high.
        high_percent: f64,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct Raw {
    id: String,
    extends: Option<String>,
    name: String,
    source: String,
    url: String,
    status: Status,
    energy_kcal: Option<f64>,
    conventions: Option<Conventions>,
    tree: Option<Vec<Node>>,
    intakes: Option<BTreeMap<String, Amount>>,
    energy_factors: Option<BTreeMap<String, EnergyFactor>>,
    claims: Option<Vec<Claim>>,
    cards: Option<Vec<String>>,
    energy_sources: Option<Vec<String>>,
    direction: Option<Direction>,
    significant_amount: Option<SignificantAmount>,
    rating: Rating,
}

/// A standard, with what it extends already filled in.
#[derive(Debug, Clone)]
pub struct Standard {
    /// Its id: `eu`, `us`, `uk`.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The text it comes from.
    pub source: String,
    /// Where to read it.
    pub url: String,
    /// What has been checked.
    pub status: Status,
    /// The daily energy the percentages of energy refer to.
    pub energy_kcal: f64,
    /// How it builds its tree.
    pub conventions: Conventions,
    /// The nutrients and how they nest, parents first.
    pub tree: Vec<Node>,
    /// The daily reference intakes, by nutrient.
    pub intakes: BTreeMap<String, Amount>,
    /// Energy per gram, where the norm gives it (empty when it does not).
    pub energy_factors: BTreeMap<String, EnergyFactor>,
    /// The nutrients singled out as small cards, in order.
    pub cards: Vec<String>,
    /// The nutrition claims and what a food needs to earn them (empty when the norm has none).
    pub claims: Vec<Claim>,
    /// The substances the energy shares of the donut are made of.
    pub energy_sources: Vec<String>,
    /// What to get less of and more of.
    pub direction: Direction,
    /// What counts as well present.
    pub significant_amount: SignificantAmount,
    /// How a nutrient is rated.
    pub rating: Rating,
}

impl Standard {
    /// The daily amount for a nutrient, by canonical name.
    pub fn intake(&self, nutrient: &str) -> Option<&Amount> {
        self.intakes.get(nutrient)
    }

    /// `value` (in the nutrient's own unit) as a percentage of the daily amount,
    /// or `None` when the standard has no amount for it or the units differ.
    pub fn percent(&self, nutrient: &str, value: f64, unit: &str) -> Option<f64> {
        let reference = self.intake(nutrient)?;
        (reference.unit == unit).then(|| value / reference.amount * 100.0)
    }

    /// The same amount of a nutrient, and of its twin when the norm counts it the other way:
    /// salt and sodium are one quantity written two ways, linked by `salt_per_sodium`.
    fn forms(&self, canon: &str, value: f64, unit: &str) -> Vec<(String, f64, String)> {
        let mut out = vec![(canon.to_string(), value, unit.to_string())];
        let factor = self.conventions.salt_per_sodium;
        if let Some(grams) = to_micrograms(value, unit).map(|ug| ug / 1e6) {
            if canon == "salt" {
                out.push(("sodiumContent".into(), grams / factor * 1000.0, "mg".into()));
            }
            if canon == "sodiumContent" {
                out.push(("salt".into(), grams * factor, "g".into()));
            }
        }
        out
    }

    /// `value` in `from` expressed in `to`, when both are masses.
    fn convert(value: f64, from: &str, to: &str) -> Option<f64> {
        Some(to_micrograms(value, from)? / to_micrograms(1.0, to)?)
    }

    /// Whether the amount per 100 g earns one of the standard's claims: a low amount of a
    /// nutrient with a "low" claim, or enough of one with a "source of" claim.
    fn earns_a_claim(&self, canon: &str, per_100g: f64, unit: &str) -> bool {
        self.forms(canon, per_100g, unit).iter().any(|(c, v, u)| {
            self.claims.iter().any(|claim| {
                if claim.nutrient != *c || claim.per != "100g" {
                    return false;
                }
                let Some(v) = Self::convert(*v, u, &claim.unit) else {
                    return false;
                };
                (claim.claim.starts_with("low-") && claim.max.is_some_and(|max| v <= max))
                    || (claim.claim.starts_with("source-of")
                        && claim.min.is_some_and(|min| v >= min))
            })
        })
    }

    /// How a nutrient reads under this standard: `low`, `mid` and `high` for what it rates on
    /// a scale, `good` for what earns a claim or is to get more of, `neutral` for the rest.
    /// `per_100g` is `None` when the recipe has no weight: what the standard rates per 100 g
    /// is then neutral. `unit` is the unit of both amounts.
    pub fn rate(
        &self,
        canon: &str,
        per_100g: Option<f64>,
        per_serving: f64,
        unit: &str,
    ) -> &'static str {
        match &self.rating {
            Rating::TrafficLight { nutrients, .. } => {
                let Some(per_100g) = per_100g else {
                    return "neutral";
                };
                for (c, v, u) in self.forms(canon, per_100g, unit) {
                    if let Some(t) = nutrients.get(&c) {
                        let Some(v) = Self::convert(v, &u, &t.unit) else {
                            continue;
                        };
                        return if v <= t.low {
                            "low"
                        } else if v <= t.high {
                            "mid"
                        } else {
                            "high"
                        };
                    }
                }
                if self.earns_a_claim(canon, per_100g, unit) {
                    "good"
                } else {
                    "neutral"
                }
            }
            Rating::Claims {} => match per_100g {
                Some(per_100g) if self.earns_a_claim(canon, per_100g, unit) => "good",
                _ => "neutral",
            },
            Rating::PercentDv {
                low_percent,
                high_percent,
            } => {
                for (c, v, u) in self.forms(canon, per_serving, unit) {
                    let Some(share) = self.share_of(&c, v, &u) else {
                        continue;
                    };
                    let percent = share * 100.0;
                    if self.direction.limit.contains(&c) {
                        return if percent <= *low_percent {
                            "low"
                        } else if percent >= *high_percent {
                            "high"
                        } else {
                            "mid"
                        };
                    }
                    if self.direction.encourage.contains(&c) {
                        return if percent >= *high_percent {
                            "good"
                        } else {
                            "neutral"
                        };
                    }
                    return "neutral";
                }
                "neutral"
            }
            Rating::None { .. } => "neutral",
        }
    }

    /// `value` of a nutrient as a fraction of its daily reference amount (1.0 is all of it).
    pub fn share_of(&self, nutrient: &str, value: f64, unit: &str) -> Option<f64> {
        let reference = self.intake(nutrient)?;
        Some(to_micrograms(value, unit)? / to_micrograms(reference.amount, &reference.unit)?)
    }

    /// Whether a nutrient is a vitamin or a mineral that can be "well present": under the
    /// minerals or vitamins group of the tree, and not one the standard says to limit.
    pub fn is_micronutrient(&self, canon: &str) -> bool {
        if self.direction.limit.iter().any(|n| n == canon) || self.node(canon).is_none() {
            return false;
        }
        let mut name = canon;
        while let Some(parent) = self.parent_of(name) {
            name = parent;
        }
        matches!(name, "minerals" | "vitamins")
    }

    /// How present a vitamin or mineral is by the standard's rule: `high` or `source`, or `None`.
    /// The share is of the reference intake supplied by 100 g, or by one serving, as the
    /// rule says; without a weight a per-100 g rule gives nothing.
    pub fn presence(
        &self,
        canon: &str,
        per_100g: Option<f64>,
        per_serving: f64,
        unit: &str,
    ) -> Option<&'static str> {
        if !self.is_micronutrient(canon) {
            return None;
        }
        let rule = &self.significant_amount;
        let amount = if rule.basis == "per-serving" {
            Some(per_serving)
        } else {
            per_100g
        }?;
        let percent = self.share_of(canon, amount, unit)? * 100.0;
        if percent >= rule.high_percent {
            Some("high")
        } else if rule.source_percent.is_some_and(|source| percent >= source) {
            Some("source")
        } else {
            None
        }
    }

    /// The node of a nutrient in the tree.
    pub fn node(&self, name: &str) -> Option<&Node> {
        self.tree.iter().find(|n| n.name == name)
    }

    /// The parent of a nutrient in this standard's tree.
    pub fn parent_of(&self, name: &str) -> Option<&str> {
        self.node(name)?.parent.as_deref()
    }

    /// How many levels below the top a node sits (0 for a top-level one).
    pub fn depth(&self, name: &str) -> usize {
        let mut depth = 0;
        let mut current = self.parent_of(name);
        // A loop in the file is ruled out by a test; the bound only keeps a bad file from hanging.
        while let Some(parent) = current.filter(|_| depth < 16) {
            depth += 1;
            current = self.parent_of(parent);
        }
        depth
    }
}

fn resolve(raw: Raw, parent: Option<&Raw>) -> Standard {
    Standard {
        id: raw.id,
        name: raw.name,
        source: raw.source,
        url: raw.url,
        status: raw.status,
        energy_kcal: raw
            .energy_kcal
            .or(parent.and_then(|p| p.energy_kcal))
            .expect("a standard has an energy value, its own or inherited"),
        conventions: raw
            .conventions
            .or(parent.and_then(|p| p.conventions.clone()))
            .expect("a standard has conventions, its own or inherited"),
        tree: raw
            .tree
            .or(parent.and_then(|p| p.tree.clone()))
            .expect("a standard has a tree, its own or inherited"),
        intakes: raw
            .intakes
            .or(parent.and_then(|p| p.intakes.clone()))
            .expect("a standard has intakes, its own or inherited"),
        energy_factors: raw
            .energy_factors
            .or(parent.and_then(|p| p.energy_factors.clone()))
            .unwrap_or_default(),
        claims: raw
            .claims
            .or(parent.and_then(|p| p.claims.clone()))
            .unwrap_or_default(),
        cards: raw
            .cards
            .or(parent.and_then(|p| p.cards.clone()))
            .unwrap_or_default(),
        energy_sources: raw
            .energy_sources
            .or(parent.and_then(|p| p.energy_sources.clone()))
            .expect("a standard has energy sources, its own or inherited"),
        direction: raw
            .direction
            .or(parent.and_then(|p| p.direction.clone()))
            .expect("a standard has a direction, its own or inherited"),
        significant_amount: raw
            .significant_amount
            .or(parent.and_then(|p| p.significant_amount.clone()))
            .expect("a standard has a significant amount, its own or inherited"),
        rating: raw.rating,
    }
}

/// Every standard, parsed once.
pub fn standards() -> &'static BTreeMap<String, Standard> {
    static PARSED: OnceLock<BTreeMap<String, Standard>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let raws: Vec<Raw> = FILES
            .iter()
            .map(|f| {
                serde_yaml::from_str(f)
                    .expect("a file of data/standards is valid, a test checks it")
            })
            .collect();
        let by_id: BTreeMap<&str, &Raw> = raws.iter().map(|r| (r.id.as_str(), r)).collect();
        raws.iter()
            .map(|r| {
                let parent = r
                    .extends
                    .as_deref()
                    .map(|id| *by_id.get(id).expect("`extends` names a standard"));
                (r.id.clone(), resolve(r.clone(), parent))
            })
            .collect()
    })
}

/// A standard by id (`eu`, `us`, `uk`).
pub fn standard(id: &str) -> Option<&'static Standard> {
    standards().get(id)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn the_three_standards_load() {
        for id in ["eu", "us", "uk"] {
            assert!(standard(id).is_some(), "{id}");
        }
        assert!(standard("nowhere").is_none());
    }

    #[test]
    fn a_standard_that_extends_another_gets_its_tree_and_intakes() {
        let (uk, eu) = (standard("uk").unwrap(), standard("eu").unwrap());
        assert_eq!(uk.tree, eu.tree);
        assert_eq!(uk.intakes, eu.intakes);
        assert!(matches!(uk.rating, Rating::TrafficLight { .. }));
    }

    #[test]
    fn every_amount_is_positive_and_in_a_known_unit() {
        for (id, s) in standards() {
            assert!(s.energy_kcal > 0.0, "{id}: energy");
            assert!(
                !s.source.is_empty() && s.url.starts_with("https://"),
                "{id}: source"
            );
            for (name, a) in &s.intakes {
                assert!(a.amount > 0.0, "{id}.{name}: amount");
                assert!(
                    ["g", "mg", "ug"].contains(&a.unit.as_str()),
                    "{id}.{name}: unit {}",
                    a.unit
                );
            }
        }
    }

    #[test]
    fn the_trees_are_well_formed() {
        for (id, s) in standards() {
            let mut seen = BTreeSet::new();
            for node in &s.tree {
                assert!(
                    seen.insert(node.name.as_str()),
                    "{id}: {} appears twice",
                    node.name
                );
                if let Some(parent) = &node.parent {
                    assert!(
                        seen.contains(parent.as_str()),
                        "{id}: {parent} must be listed before {}",
                        node.name
                    );
                }
                assert!(
                    s.depth(&node.name) <= 4,
                    "{id}: {} sits too deep",
                    node.name
                );
            }
            for name in s.intakes.keys() {
                assert!(
                    s.node(name).is_some_and(|n| !n.group),
                    "{id}: the intake of {name} has no nutrient in the tree"
                );
            }
        }
    }

    #[test]
    fn a_relation_is_listed_or_absent_and_only_on_a_nutrient() {
        for (id, s) in standards() {
            for node in &s.tree {
                if let Some(relation) = &node.relation {
                    assert_eq!(relation, "listed", "{id}.{}", node.name);
                    assert!(!node.group && node.parent.is_some(), "{id}.{}", node.name);
                }
            }
        }
        assert_eq!(
            standard("us")
                .unwrap()
                .node("cholesterolContent")
                .unwrap()
                .relation
                .as_deref(),
            Some("listed")
        );
    }

    #[test]
    fn every_nutrient_of_a_standard_is_in_its_tree_once_with_the_others_it_shares() {
        // the trees differ in where things sit, not in what they hold: a nutrient in the EU tree
        // is in the US tree too, bar those that only one label knows
        let (eu, us) = (standard("eu").unwrap(), standard("us").unwrap());
        let only_eu = ["fluoride", "polyols"];
        for node in &eu.tree {
            if only_eu.contains(&node.name.as_str()) {
                continue;
            }
            assert!(
                us.node(&node.name).is_some(),
                "{} is in the EU tree but not the US one",
                node.name
            );
        }
    }

    #[test]
    fn the_fibre_convention_matches_the_tree() {
        for (id, s) in standards() {
            let under_carbohydrate = s.parent_of("fiberContent") == Some("carbohydrateContent");
            assert_eq!(
                under_carbohydrate, s.conventions.fibre_in_carbohydrate,
                "{id}"
            );
        }
        assert!(!standard("eu").unwrap().conventions.fibre_in_carbohydrate);
        assert!(standard("us").unwrap().conventions.fibre_in_carbohydrate);
    }

    #[test]
    fn the_label_gives_salt_or_sodium_and_the_tree_has_that_one() {
        for (id, s) in standards() {
            let key = if s.conventions.declares == "salt" {
                "salt"
            } else {
                "sodiumContent"
            };
            assert!(
                matches!(s.conventions.declares.as_str(), "salt" | "sodium"),
                "{id}"
            );
            assert!(s.node(key).is_some(), "{id}: no {key} in the tree");
        }
    }

    #[test]
    fn the_nutrients_of_the_table_that_have_a_reference_use_its_unit() {
        // A standard may leave a nutrient out, but where it has one the unit must
        // be the table's, or a percentage would compare grams with milligrams.
        for (id, s) in standards() {
            for n in crate::nutrition::NUTRIENTS {
                if let Some(reference) = s.intake(n.name) {
                    // `ug` and `µg` are the same unit, written two ways
                    let magnitude = |unit: &str| crate::nutrition::to_micrograms(1.0, unit);
                    assert_eq!(
                        magnitude(&reference.unit),
                        magnitude(n.default_unit),
                        "{id}.{}",
                        n.name
                    );
                }
            }
        }
    }

    #[test]
    fn eu_values_match_annex_xiii() {
        let eu = standard("eu").unwrap();
        assert_eq!(eu.intake("fatContent").unwrap().amount, 70.0);
        assert_eq!(eu.intake("saturatedFatContent").unwrap().amount, 20.0);
        assert_eq!(eu.intake("carbohydrateContent").unwrap().amount, 260.0);
        assert_eq!(eu.intake("sugarContent").unwrap().amount, 90.0);
        assert_eq!(eu.intake("proteinContent").unwrap().amount, 50.0);
        assert_eq!(eu.intake("salt").unwrap().amount, 6.0);
    }

    #[test]
    fn percent_needs_matching_units() {
        let eu = standard("eu").unwrap();
        assert_eq!(eu.percent("fatContent", 7.0, "g"), Some(10.0));
        assert_eq!(eu.percent("fatContent", 7000.0, "mg"), None);
        assert_eq!(eu.percent("unknown", 1.0, "g"), None);
    }

    #[test]
    fn depth_follows_the_tree_of_the_standard() {
        let (eu, us) = (standard("eu").unwrap(), standard("us").unwrap());
        assert_eq!(eu.depth("fatContent"), 1);
        assert_eq!(eu.depth("monounsaturatedFatContent"), 3);
        // fibre sits next to carbohydrate in the EU, under it in the US
        assert_eq!(eu.depth("fiberContent"), eu.depth("carbohydrateContent"));
        assert_eq!(
            us.depth("fiberContent"),
            us.depth("carbohydrateContent") + 1
        );
    }

    #[test]
    fn the_ratings_are_ordered_and_say_what_is_unconfirmed() {
        for (id, s) in standards() {
            match &s.rating {
                Rating::TrafficLight {
                    nutrients,
                    unverified,
                    ..
                } => {
                    for (name, t) in nutrients {
                        assert!(t.low < t.high, "{id}.{name}");
                    }
                    for name in unverified {
                        assert!(
                            nutrients.contains_key(name),
                            "{id}: {name} is flagged but has no threshold"
                        );
                    }
                }
                Rating::PercentDv {
                    low_percent,
                    high_percent,
                } => assert!(low_percent < high_percent, "{id}"),
                Rating::Claims {} | Rating::None { .. } => {}
            }
        }
        let Rating::TrafficLight {
            nutrients,
            unverified,
            ..
        } = &standard("uk").unwrap().rating
        else {
            panic!()
        };
        assert_eq!(nutrients["saturatedFatContent"].high, 5.0);
        assert_eq!(unverified, &["salt".to_string()]);
    }

    #[test]
    fn the_eu_energy_factors_are_those_of_annex_xiv() {
        let f = &standard("eu").unwrap().energy_factors;
        assert_eq!((f["fat"].kj, f["fat"].kcal), (37.0, 9.0));
        assert_eq!((f["carbohydrate"].kj, f["carbohydrate"].kcal), (17.0, 4.0));
        assert_eq!((f["polyols"].kj, f["polyols"].kcal), (10.0, 2.4));
        assert_eq!((f["fibre"].kj, f["fibre"].kcal), (8.0, 2.0));
        assert_eq!(f["erythritol"].kcal, 0.0);
        // the UK reuses them, the US page gives none
        assert_eq!(standard("uk").unwrap().energy_factors, *f);
        assert!(standard("us").unwrap().energy_factors.is_empty());
        // kJ and kcal agree to within what a rounded factor allows
        for (name, factor) in f {
            assert!((factor.kj - factor.kcal * 4.184).abs() < 1.0, "{name}");
        }
    }

    #[test]
    fn the_eu_claims_are_those_of_the_annex_of_1924_2006() {
        let claims = &standard("eu").unwrap().claims;
        assert!(matches!(standard("eu").unwrap().rating, Rating::Claims {}));
        let find = |name: &str| {
            claims
                .iter()
                .find(|c| c.claim == name)
                .unwrap_or_else(|| panic!("{name}"))
        };
        assert_eq!(
            (find("low-fat").max, find("low-fat").liquid_max),
            (Some(3.0), Some(1.5))
        );
        assert_eq!(find("low-saturated-fat").max, Some(1.5));
        assert_eq!(find("low-sugars").max, Some(5.0));
        assert_eq!(find("low-sodium").max, Some(120.0));
        assert_eq!(find("source-of-fibre").min, Some(3.0));
        assert_eq!(find("high-fibre").min, Some(6.0));
        assert_eq!(find("source-of-protein").min, Some(12.0));
        assert_eq!(find("high-protein").min, Some(20.0));
        let significant = &standard("eu").unwrap().significant_amount;
        assert_eq!(significant.basis, "per-100g");
        assert_eq!(significant.source_percent, Some(15.0));
        assert_eq!(significant.beverage_percent_per_100ml, Some(7.5));
        // "high in" needs twice the "source of" amount (Annex of 1924/2006)
        assert_eq!(significant.high_percent, 2.0 * 15.0);
        // a "high" claim asks for more than the matching "source of" one
        assert!(find("high-fibre").min > find("source-of-fibre").min);
        assert!(find("high-protein").min > find("source-of-protein").min);
        for c in claims {
            assert!(
                c.max.is_some() || c.liquid_max.is_some() || c.min.is_some(),
                "{}",
                c.claim
            );
        }
    }

    #[test]
    fn the_uk_low_salt_matches_the_eu_low_sodium_claim() {
        // 0,12 g of sodium is 0,30 g of salt (salt = sodium x 2,5)
        let claims = &standard("eu").unwrap().claims;
        let sodium_mg = claims
            .iter()
            .find(|c| c.claim == "low-sodium")
            .unwrap()
            .max
            .unwrap();
        let Rating::TrafficLight { nutrients, .. } = &standard("uk").unwrap().rating else {
            panic!()
        };
        assert!((sodium_mg * 2.5 / 1000.0 - nutrients["salt"].low).abs() < 1e-9);
    }

    #[test]
    fn the_rules_that_a_display_needs_come_from_the_file() {
        for (id, s) in standards() {
            // every nutrient the direction or the energy sources name is in the tree
            for name in s
                .direction
                .limit
                .iter()
                .chain(&s.direction.encourage)
                .chain(&s.energy_sources)
            {
                assert!(
                    s.node(name).is_some_and(|n| !n.group),
                    "{id}: {name} is not in the tree"
                );
            }
            // a nutrient is not both to limit and to encourage
            for name in &s.direction.limit {
                assert!(!s.direction.encourage.contains(name), "{id}: {name}");
            }
            assert!(s.conventions.salt_per_sodium > 0.0, "{id}");
            assert!(
                ["per-100g", "per-serving"].contains(&s.significant_amount.basis.as_str()),
                "{id}"
            );
            if let Some(source) = s.significant_amount.source_percent {
                assert!(source < s.significant_amount.high_percent, "{id}");
            }
            assert_eq!(
                s.energy_sources.len(),
                3,
                "{id}: fat, carbohydrate, protein"
            );
        }
        let (eu, us) = (standard("eu").unwrap(), standard("us").unwrap());
        // the US has no "source of" step, and judges per serving
        assert_eq!(us.significant_amount.basis, "per-serving");
        assert_eq!(us.significant_amount.source_percent, None);
        assert_eq!(us.significant_amount.high_percent, 20.0);
        assert!(us.direction.limit.contains(&"sodiumContent".to_string()));
        assert!(us.direction.encourage.contains(&"fiberContent".to_string()));
        // a sodium to limit is the salt to limit, in either unit
        assert!(
            eu.direction.limit.contains(&"salt".to_string())
                && eu.direction.limit.contains(&"sodiumContent".to_string())
        );
        assert_eq!(eu.conventions.salt_per_sodium, 2.5);
        // the UK reuses what the EU says, the claims included: they are law there too
        assert_eq!(standard("uk").unwrap().direction, eu.direction);
        assert_eq!(standard("uk").unwrap().claims, eu.claims);
        assert!(us.claims.is_empty());
        assert_eq!(
            standard("uk").unwrap().significant_amount,
            eu.significant_amount
        );
    }

    // The bowl of the mockup: 360 g a serving, 6,2 g of saturated fat, 7,5 g of sugars, 14,5 g of
    // fibre, 1,4 g of salt (560 mg of sodium).
    #[test]
    fn the_same_food_is_rated_by_the_rules_of_each_standard() {
        let (eu, us, uk) = (
            standard("eu").unwrap(),
            standard("us").unwrap(),
            standard("uk").unwrap(),
        );
        let per_100g = |per_serving: f64| Some(per_serving / 360.0 * 100.0);
        // UK traffic lights per 100 g: 1,7 g of saturates is amber, 2,1 g of sugars green
        assert_eq!(
            uk.rate("saturatedFatContent", per_100g(6.2), 6.2, "g"),
            "mid"
        );
        assert_eq!(uk.rate("sugarContent", per_100g(7.5), 7.5, "g"), "low");
        assert_eq!(uk.rate("salt", per_100g(1.4), 1.4, "g"), "mid");
        // the EU colours only what earns a claim: low sugars and a source of fibre
        assert_eq!(
            eu.rate("saturatedFatContent", per_100g(6.2), 6.2, "g"),
            "neutral"
        );
        assert_eq!(eu.rate("sugarContent", per_100g(7.5), 7.5, "g"), "good");
        assert_eq!(eu.rate("fiberContent", per_100g(14.5), 14.5, "g"), "good");
        assert_eq!(eu.rate("salt", per_100g(1.4), 1.4, "g"), "neutral");
        // the US judges the share of the daily value of a serving, and gives no value to sugars
        assert_eq!(
            us.rate("saturatedFatContent", per_100g(6.2), 6.2, "g"),
            "high"
        );
        assert_eq!(
            us.rate("sodiumContent", per_100g(560.0), 560.0, "mg"),
            "high"
        );
        assert_eq!(us.rate("salt", per_100g(1.4), 1.4, "g"), "high");
        assert_eq!(us.rate("sugarContent", per_100g(7.5), 7.5, "g"), "neutral");
        assert_eq!(us.rate("fiberContent", per_100g(14.5), 14.5, "g"), "good");
        // no weight: what is rated per 100 g stays neutral, what is per serving does not
        assert_eq!(uk.rate("saturatedFatContent", None, 6.2, "g"), "neutral");
        assert_eq!(us.rate("saturatedFatContent", None, 6.2, "g"), "high");
    }

    #[test]
    fn a_vitamin_or_mineral_is_well_present_by_the_rule_of_the_standard() {
        let (eu, us) = (standard("eu").unwrap(), standard("us").unwrap());
        // vitamin D: 12,5 µg a serving of 360 g, which is 3,5 µg per 100 g
        let per_100g = Some(12.5 / 360.0 * 100.0);
        assert_eq!(eu.presence("vitaminD", per_100g, 12.5, "µg"), Some("high"));
        // vitamin E: 1,9 mg per 100 g is 16 % of 12 mg, a source but not high
        assert_eq!(
            eu.presence("vitaminE", Some(1.9), 6.8, "mg"),
            Some("source")
        );
        assert_eq!(eu.presence("calcium", Some(39.0), 142.0, "mg"), None);
        // the US counts a serving, and has no "source of" step
        assert_eq!(us.presence("vitaminD", per_100g, 12.5, "µg"), Some("high"));
        assert_eq!(us.presence("calcium", Some(39.0), 142.0, "mg"), None);
        // what is to limit is never well present, and a macro is not a micronutrient
        assert_eq!(us.presence("sodiumContent", Some(156.0), 560.0, "mg"), None);
        assert!(!eu.is_micronutrient("fatContent"));
        assert!(eu.is_micronutrient("magnesium"));
        // no weight, no per-100 g rule
        assert_eq!(eu.presence("vitaminD", None, 12.5, "µg"), None);
    }

    #[test]
    fn the_cards_are_nutrients_of_the_tree() {
        for (id, s) in standards() {
            assert!(!s.cards.is_empty(), "{id}");
            for name in &s.cards {
                assert!(s.node(name).is_some_and(|n| !n.group), "{id}: {name}");
            }
        }
        // the EU label gives salt, the US one sodium: the card follows
        assert!(standard("eu").unwrap().cards.contains(&"salt".to_string()));
        assert!(standard("us")
            .unwrap()
            .cards
            .contains(&"sodiumContent".to_string()));
    }

    #[test]
    fn what_is_not_verified_is_said() {
        for (id, s) in standards() {
            for (part, value) in [
                ("intakes", &s.status.intakes),
                ("tree", &s.status.tree),
                ("rating", &s.status.rating),
            ] {
                assert!(
                    ["verified", "to-verify", "partly-verified"].contains(&value.as_str()),
                    "{id}.{part}: {value}"
                );
            }
        }
    }
}
