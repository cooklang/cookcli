//! Meal plans: menus whose sections are dated.
//!
//! A menu with sections on two days or more, such as
//! `== Thursday (2026-10-01) ==` or `= 2026-10-02 Dinner` (the forms
//! cooklang-find's `list_menus_for_date` matches), shows as a calendar.
//! Nothing else marks a plan: the dates are already in the text that the CLI,
//! the shopping list and other Cooklang apps read.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use regex::Regex;
use serde::Serialize;
use unic_langid::LanguageIdentifier;

use crate::web::menus::{extract_meal_type, extract_time, is_meal_header};
use crate::web::templates::{MenuSection, MenuSectionItem};

/// The longest plan the new-plan form writes: about two months.
#[cfg(feature = "server")]
pub const MAX_PLAN_DAYS: u32 = 62;
/// The shortest: a menu needs two dated days to show as a plan.
pub const MIN_PLAN_DAYS: u32 = 2;

/// A `YYYY-MM-DD` anywhere in a section name, not inside a longer number.
static SECTION_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^\d-])(\d{4}-\d{2}-\d{2})(?:[^\d-]|$)").unwrap());

/// What the new-plan form asks for: the days a plan covers and the meals each
/// of them starts with.
#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFrame {
    pub start: NaiveDate,
    pub days: u32,
    /// Meals every day has a slot for, in order.
    pub meals: Vec<String>,
}

#[cfg(feature = "server")]
impl PlanFrame {
    fn dates(&self) -> impl Iterator<Item = NaiveDate> + '_ {
        (0..self.days).filter_map(|offset| self.start.checked_add_days(Days::new(offset.into())))
    }
}

/// A plan laid out as a calendar, one row per week.
#[derive(Debug, Clone, Serialize)]
pub struct PlanView {
    /// Column headings, in the order the rows run.
    pub weekdays: Vec<String>,
    /// Seven cells a row; `None` pads the first and last week. A week with no
    /// dated section is left out.
    pub weeks: Vec<Vec<Option<PlanDay>>>,
    /// Sections with no date.
    pub outside: Vec<MenuSection>,
    /// Set by the server when the viewer may change the plan from the page.
    pub edit: Option<PlanEdit>,
}

/// What the calendar needs to change the plan from the page.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PlanEdit {
    /// The plan's path in the collection, with `.menu`.
    pub path: String,
    /// A hash of the text the page shows, sent back with every change so one
    /// meant for an older text is refused.
    pub version: String,
    /// The plan's `servings`, which a recipe added from the page is given.
    pub servings: Option<String>,
    /// Every meal a line can be moved or copied to.
    pub meals: Vec<String>,
    /// The file's lines for each day's meal, keyed `YYYY-MM-DD|Meal`. Only
    /// meals whose lines match the card's one for one are here, so a line
    /// on the page is always the line a change reaches.
    pub items: std::collections::HashMap<String, Vec<String>>,
}

impl PlanEdit {
    /// The file's lines for `meal` on `date`, when the card may change them.
    pub fn items(&self, date: &str, meal: &str) -> Option<&Vec<String>> {
        self.items.get(&format!("{date}|{meal}"))
    }
}

impl PlanView {
    /// The file's text of line `index` of `meal` on `date`, when the page may
    /// change that line.
    pub fn line_text(&self, date: &str, meal: Option<&str>, index: &usize) -> Option<&str> {
        self.edit
            .as_ref()?
            .items(date, meal?)?
            .get(*index)
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanDay {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The day as the page shows it, e.g. `Wed 1 Oct`.
    pub label: String,
    pub meals: Vec<PlanMeal>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanMeal {
    /// `None` for lines written before the day's first meal.
    pub name: Option<String>,
    /// `HH:MM`, from a header like `Breakfast (08:30):`.
    pub time: Option<String>,
    pub lines: Vec<Vec<MenuSectionItem>>,
}

/// The day a section is for, and the meal header its name gives after the
/// date (`2026-10-02 Dinner` → `Dinner:`), if any.
pub fn section_date(name: &str) -> Option<(NaiveDate, Option<String>)> {
    let found = SECTION_DATE_RE.captures(name)?.get(1)?;
    let date = NaiveDate::parse_from_str(found.as_str(), "%Y-%m-%d").ok()?;
    // `Wednesday (2026-10-07)` leaves `)`, which names no meal.
    let rest = name[found.end()..]
        .trim_start_matches(|c: char| c.is_whitespace() || ")]-–—:,".contains(c))
        .trim_end_matches(|c: char| c.is_whitespace() || c == ':');
    let header = format!("{rest}:");
    Some((date, is_meal_header(&header).then_some(header)))
}

/// Whether sections named `names` make a plan: they are on two days or more.
#[cfg(feature = "server")]
pub fn is_plan<'a>(names: impl IntoIterator<Item = &'a str>) -> bool {
    let days: std::collections::BTreeSet<NaiveDate> = names
        .into_iter()
        .filter_map(|name| section_date(name).map(|(date, _)| date))
        .collect();
    days.len() >= MIN_PLAN_DAYS as usize
}

/// Lays `sections` out as a calendar, or `None` when fewer than two days are
/// dated: the menu then shows as an ordinary menu.
///
/// The calendar runs from the first dated day to the last. A day no section
/// is for still gets a cell in a week that has one, and every day offers the
/// meals the plan's days name, in the order they first appear.
pub fn build_plan_view(sections: &[MenuSection], lang: &LanguageIdentifier) -> Option<PlanView> {
    let mut dated: BTreeMap<NaiveDate, Vec<PlanMeal>> = BTreeMap::new();
    let mut outside = Vec::new();
    for section in sections {
        match section.name.as_deref().and_then(section_date) {
            Some((date, header)) => {
                let meals = dated.entry(date).or_default();
                match header {
                    // The meal the name gives holds the lines before the
                    // section's first meal header.
                    Some(header) => {
                        let mut lines = vec![vec![MenuSectionItem::Text(header)]];
                        lines.extend(section.lines.iter().cloned());
                        add_lines(meals, &lines);
                    }
                    None => add_lines(meals, &section.lines),
                }
            }
            None => outside.push(section.clone()),
        }
    }
    if dated.len() < MIN_PLAN_DAYS as usize {
        return None;
    }

    let slots = meal_slots(
        dated
            .values()
            .map(|meals| meals.iter().filter_map(|meal| meal.name.as_deref())),
    );
    for meals in dated.values_mut() {
        fill_slots(meals, &slots);
    }

    let locale = chrono_locale(lang);
    let first_weekday = first_weekday(lang);
    let first = *dated.keys().next()?;
    let last = *dated.keys().next_back()?;
    let mut week_starts: Vec<NaiveDate> = dated
        .keys()
        .map(|date| *date - Days::new(date.weekday().days_since(first_weekday).into()))
        .collect();
    week_starts.dedup();

    let weeks = week_starts
        .into_iter()
        .map(|week_start| {
            (0..7)
                .map(|offset| {
                    let date = week_start.checked_add_days(Days::new(offset))?;
                    if date < first || date > last {
                        return None;
                    }
                    let meals = dated.remove(&date).unwrap_or_else(|| {
                        let mut meals = Vec::new();
                        fill_slots(&mut meals, &slots);
                        meals
                    });
                    Some(PlanDay {
                        date: date.format("%Y-%m-%d").to_string(),
                        label: capitalize(&date.format_localized("%a %-d %b", locale).to_string()),
                        meals,
                    })
                })
                .collect()
        })
        .collect();

    // 2024-01-01 was a Monday: walk a week from there to name the columns.
    let monday = NaiveDate::from_ymd_opt(2024, 1, 1).expect("a valid date");
    let weekdays = (0..7)
        .map(|i| {
            let offset = u64::from(first_weekday.num_days_from_monday()) + i;
            let date = monday + Days::new(offset);
            capitalize(&date.format_localized("%a", locale).to_string())
        })
        .collect();

    Some(PlanView {
        weekdays,
        weeks,
        outside,
        edit: None,
    })
}

/// The meals the plan's days name, each once. A meal a day adds goes after
/// the meal it follows on that day, or before the one it precedes when it is
/// the day's first: days with `Breakfast, Dinner` and `Breakfast, Lunch` give
/// `Breakfast, Lunch, Dinner`, and days with `Dinner` and `Lunch, Dinner`
/// give `Lunch, Dinner`.
pub fn meal_slots<'a, D>(days: impl IntoIterator<Item = D>) -> Vec<String>
where
    D: IntoIterator<Item = &'a str>,
{
    let mut slots: Vec<String> = Vec::new();
    for meals in days {
        let meals: Vec<&str> = meals.into_iter().collect();
        let mut after: Option<usize> = None;
        for (n, name) in meals.iter().enumerate() {
            let index = match slots.iter().position(|slot| slot == name) {
                Some(index) => index,
                None => {
                    let index = match after {
                        Some(after) => after + 1,
                        None => meals[n + 1..]
                            .iter()
                            .find_map(|next| slots.iter().position(|slot| slot == next))
                            .unwrap_or(slots.len()),
                    };
                    slots.insert(index, name.to_string());
                    index
                }
            };
            after = Some(index);
        }
    }
    slots
}

/// Gives `meals` an empty meal for each slot it lacks, and puts its meals in
/// the slots' order after the lines written before any meal.
fn fill_slots(meals: &mut Vec<PlanMeal>, slots: &[String]) {
    for slot in slots {
        if !meals.iter().any(|meal| meal.name.as_ref() == Some(slot)) {
            meals.push(PlanMeal {
                name: Some(slot.clone()),
                time: None,
                lines: Vec::new(),
            });
        }
    }
    meals.sort_by_key(|meal| {
        meal.name
            .as_ref()
            .map(|name| slots.iter().position(|slot| slot == name))
    });
}

/// Appends a section's lines to the day's meals: a line that is only
/// `Name:` starts a meal (the rule the menu API follows), and a meal the day
/// already has, from another of its sections, is continued rather than
/// repeated.
fn add_lines(meals: &mut Vec<PlanMeal>, lines: &[Vec<MenuSectionItem>]) {
    let mut current: Option<usize> = None;
    for line in lines {
        if let [MenuSectionItem::Text(text)] = line.as_slice() {
            if is_meal_header(text) {
                let name = extract_meal_type(text);
                let time = extract_time(text);
                let index = match meals
                    .iter()
                    .position(|meal| meal.name.as_deref() == Some(name.as_str()))
                {
                    Some(index) => index,
                    None => {
                        meals.push(PlanMeal {
                            name: Some(name),
                            time: None,
                            lines: Vec::new(),
                        });
                        meals.len() - 1
                    }
                };
                if time.is_some() {
                    meals[index].time = time;
                }
                current = Some(index);
                continue;
            }
        }

        let line = tidy(line);
        if line.is_empty() {
            continue;
        }
        let index = *current.get_or_insert_with(|| {
            // Lines before any meal: kept first, under no heading.
            meals.insert(
                0,
                PlanMeal {
                    name: None,
                    time: None,
                    lines: Vec::new(),
                },
            );
            0
        });
        meals[index].lines.push(line);
    }
}

/// A line as a day card shows it: without its `- ` bullet or blank text at
/// either end. An empty bullet, as a new plan has under each meal, comes out
/// empty.
fn tidy(line: &[MenuSectionItem]) -> Vec<MenuSectionItem> {
    let blank = |item: &MenuSectionItem| matches!(item, MenuSectionItem::Text(text) if matches!(text.trim(), "" | "-"));
    let mut line: Vec<MenuSectionItem> = line.to_vec();
    while line.last().is_some_and(blank) {
        line.pop();
    }
    let lead = line.iter().take_while(|item| blank(item)).count();
    line.drain(..lead);
    if let Some(MenuSectionItem::Text(text)) = line.first_mut() {
        if let Some(rest) = text.trim_start().strip_prefix("- ") {
            *text = rest.to_string();
        }
    }
    line
}

/// The section heading a new plan gives `date`: `Thursday (2026-10-01)`, the
/// weekday in the page's language, as the editor toolbar writes it.
#[cfg(feature = "server")]
pub(crate) fn day_heading(date: NaiveDate, lang: &LanguageIdentifier) -> String {
    let weekday = date.format_localized("%A", chrono_locale(lang)).to_string();
    format!("{} ({})", capitalize(&weekday), date.format("%Y-%m-%d"))
}

/// What a new plan titled `title` starts with: the frontmatter, then every day
/// as a dated section with an empty bullet under each meal, ready to fill in
/// the editor. The dated sections are all that makes it a plan.
#[cfg(feature = "server")]
pub fn plan_starter(
    title: &str,
    servings: u32,
    frame: &PlanFrame,
    lang: &LanguageIdentifier,
) -> String {
    use std::fmt::Write;

    let mut out = format!("---\ntitle: {title}\nservings: {servings}\n---\n");
    for date in frame.dates() {
        let _ = write!(out, "\n== {} ==\n", day_heading(date, lang));
        for meal in &frame.meals {
            // The ` \` keeps the bullet in the meal, as in a new menu.
            let _ = write!(out, "\n{meal}: \\\n- \n");
        }
    }
    out
}

/// chrono's locale for `lang`, falling back to English.
fn chrono_locale(lang: &LanguageIdentifier) -> chrono::Locale {
    let name = match lang.region {
        Some(region) => format!("{}_{}", lang.language, region),
        None => lang.language.to_string(),
    };
    chrono::Locale::try_from(name.as_str()).unwrap_or(chrono::Locale::en_US)
}

/// Weeks start on Sunday in the United States and Japan, on Monday in the
/// other places the UI is translated for.
fn first_weekday(lang: &LanguageIdentifier) -> Weekday {
    match lang.region.as_ref().map(|region| region.as_str()) {
        Some("US") | Some("JP") => Weekday::Sun,
        _ => Weekday::Mon,
    }
}

/// Some languages write weekday names in lower case (`mercredi`); a heading
/// starts with a capital either way.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lang(tag: &str) -> LanguageIdentifier {
        tag.parse().unwrap()
    }

    #[cfg(feature = "server")]
    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn text(value: &str) -> Vec<MenuSectionItem> {
        vec![MenuSectionItem::Text(value.to_string())]
    }

    fn recipe(name: &str) -> Vec<MenuSectionItem> {
        vec![MenuSectionItem::RecipeReference {
            name: name.to_string(),
            scale: None,
            servings: None,
            menu: false,
        }]
    }

    fn section(name: &str, lines: Vec<Vec<MenuSectionItem>>) -> MenuSection {
        MenuSection {
            name: Some(name.to_string()),
            lines,
        }
    }

    fn day(date: &str) -> MenuSection {
        section(&format!("Day ({date})"), vec![])
    }

    fn dates(view: &PlanView) -> Vec<Option<&str>> {
        view.weeks
            .iter()
            .flatten()
            .map(|cell| cell.as_ref().map(|day| day.date.as_str()))
            .collect()
    }

    fn meals(day: &PlanDay) -> Vec<(Option<String>, usize)> {
        day.meals
            .iter()
            .map(|meal| (meal.name.clone(), meal.lines.len()))
            .collect()
    }

    #[test]
    fn a_section_date_can_sit_anywhere_in_its_name() {
        let date = |text: &str| NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap();
        assert_eq!(
            section_date("Wednesday (2026-10-07)"),
            Some((date("2026-10-07"), None))
        );
        assert_eq!(
            section_date("2026-10-07 Dinner"),
            Some((date("2026-10-07"), Some("Dinner:".into())))
        );
        assert_eq!(
            section_date("(2026-10-07) - Breakfast (08:30)"),
            Some((date("2026-10-07"), Some("Breakfast (08:30):".into())))
        );
        assert_eq!(section_date("2026-10-07"), Some((date("2026-10-07"), None)));
        for name in [
            "Day 1",
            "2026-02-30",
            "12026-10-07",
            "2026-10-071",
            "Week 2026-10",
        ] {
            assert_eq!(section_date(name), None, "{name}");
        }
    }

    #[test]
    fn a_menu_needs_two_dated_days_to_be_a_plan() {
        let en = lang("en-GB");
        assert!(build_plan_view(&[], &en).is_none());
        assert!(build_plan_view(&[day("2026-12-25"), section("Day 2", vec![])], &en).is_none());
        // Two sections on the same day are still one day.
        assert!(build_plan_view(&[day("2026-12-25"), day("2026-12-25")], &en).is_none());
        assert!(build_plan_view(&[day("2026-12-25"), day("2026-12-26")], &en).is_some());
    }

    #[test]
    fn a_wednesday_start_leaves_blank_cells_before_it() {
        let view =
            build_plan_view(&[day("2026-10-16"), day("2026-10-07")], &lang("fr-FR")).unwrap();

        assert_eq!(view.weeks.len(), 2);
        assert!(view.weeks.iter().all(|week| week.len() == 7));
        let dates = dates(&view);
        assert_eq!(&dates[..3], &[None, None, Some("2026-10-07")]);
        // The days between the two sections get a cell too.
        assert_eq!(dates[5], Some("2026-10-10"));
        assert_eq!(dates[11], Some("2026-10-16"));
        assert!(dates[12..].iter().all(Option::is_none));
        assert_eq!(view.weekdays[0], "Lun.");
    }

    #[test]
    fn weeks_start_on_sunday_in_american_english() {
        let view =
            build_plan_view(&[day("2026-10-07"), day("2026-10-16")], &lang("en-US")).unwrap();

        assert_eq!(view.weekdays[0], "Sun");
        let first_week = &view.weeks[0];
        assert!(first_week[..3].iter().all(Option::is_none));
        assert_eq!(first_week[3].as_ref().unwrap().label, "Wed 7 Oct");
    }

    #[test]
    fn weeks_with_no_dated_section_are_left_out() {
        let view =
            build_plan_view(&[day("2026-10-07"), day("2027-01-01")], &lang("en-GB")).unwrap();

        assert_eq!(view.weeks.len(), 2);
        let dates = dates(&view);
        assert_eq!(dates[2], Some("2026-10-07"));
        assert_eq!(dates[6], Some("2026-10-11"));
        assert_eq!(dates[7], Some("2026-12-28"));
        assert_eq!(dates[11], Some("2027-01-01"));
    }

    #[test]
    fn sections_land_on_their_day_and_meal() {
        let sections = [
            section(
                "Friday (2026-10-02)",
                vec![
                    text("Dinner:"),
                    recipe("./Risotto"),
                    text("Breakfast (08:30):"),
                    recipe("./Pancakes"),
                ],
            ),
            section(
                "Friday again (2026-10-02)",
                vec![
                    text("Dinner:"),
                    recipe("./Salad"),
                    text("Snacks:"),
                    recipe("./Nuts"),
                ],
            ),
            section("2026-10-03 Lunch", vec![recipe("./Soup")]),
            section("Sunday (2026-10-04)", vec![recipe("./Toast")]),
            section("Day 1", vec![recipe("./Undated")]),
        ];
        let view = build_plan_view(&sections, &lang("en-GB")).unwrap();
        let days: Vec<&PlanDay> = view.weeks.iter().flatten().flatten().collect();
        assert_eq!(days.len(), 3);

        // Every day offers every meal the plan names, in the order they first
        // appear: Lunch comes last, as nothing comes before it on Saturday.
        let slots = |lines: [usize; 4]| {
            ["Dinner", "Breakfast", "Snacks", "Lunch"]
                .into_iter()
                .zip(lines)
                .map(|(name, lines)| (Some(name.to_string()), lines))
                .collect::<Vec<_>>()
        };
        assert_eq!(meals(days[0]), slots([2, 1, 1, 0]));
        assert_eq!(days[0].meals[1].time.as_deref(), Some("08:30"));
        // A section named after its meal fills that meal.
        assert_eq!(meals(days[1]), slots([0, 0, 0, 1]));
        // Lines before any meal come first, under no heading.
        let mut sunday = vec![(None, 1)];
        sunday.extend(slots([0, 0, 0, 0]));
        assert_eq!(meals(days[2]), sunday);

        let outside: Vec<_> = view
            .outside
            .iter()
            .map(|s| s.name.clone().unwrap())
            .collect();
        assert_eq!(outside, ["Day 1"]);
    }

    #[test]
    fn a_meal_a_day_adds_goes_after_the_one_it_follows() {
        assert_eq!(
            meal_slots([
                vec!["Breakfast", "Dinner"],
                vec!["Breakfast", "Lunch"],
                vec!["Supper"],
            ]),
            ["Breakfast", "Lunch", "Dinner", "Supper"]
        );
        // A day's first meal goes before the next one it has.
        assert_eq!(
            meal_slots([vec!["Dinner"], vec!["Lunch", "Dinner"]]),
            ["Lunch", "Dinner"]
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn day_headings_are_in_the_page_language() {
        assert_eq!(
            day_heading(date("2026-10-07"), &lang("fr-FR")),
            "Mercredi (2026-10-07)"
        );
        assert_eq!(
            day_heading(date("2026-10-07"), &lang("en-US")),
            "Wednesday (2026-10-07)"
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn a_new_plan_reads_back_as_the_plan_it_was_made_from() {
        let frame = PlanFrame {
            start: date("2026-10-07"),
            days: 3,
            meals: vec!["Breakfast".into(), "Dinner".into()],
        };
        let text = plan_starter("October", 2, &frame, &lang("en-US"));
        let recipe = crate::util::PARSER
            .parse(&text)
            .into_output()
            .expect("the plan parses");

        assert_eq!(
            recipe.metadata.get("servings").and_then(|v| v.as_u64()),
            Some(2)
        );
        assert!(recipe.metadata.get("plan").is_none());
        assert!(
            text.contains("== Thursday (2026-10-08) ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- \n")
        );

        // The menu page reads the same days and meals back from the sections.
        let sections: Vec<MenuSection> = recipe
            .sections
            .iter()
            .map(|section| section_with_name(section.name.as_deref()))
            .collect();
        let view = build_plan_view(&sections, &lang("en-US")).unwrap();
        let days: Vec<&PlanDay> = view.weeks.iter().flatten().flatten().collect();
        let dates: Vec<&str> = days.iter().map(|day| day.date.as_str()).collect();
        assert_eq!(dates, ["2026-10-07", "2026-10-08", "2026-10-09"]);
    }

    #[cfg(feature = "server")]
    fn section_with_name(name: Option<&str>) -> MenuSection {
        MenuSection {
            name: name.map(String::from),
            lines: Vec::new(),
        }
    }
}
