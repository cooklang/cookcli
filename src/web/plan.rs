//! Meal plans: menus that a `plan:` block in the frontmatter pins to dates.
//!
//! ```yaml
//! plan:
//!   start: 2026-10-01
//!   days: 10
//!   meals: [Breakfast, Lunch, Dinner]
//! ```
//!
//! The block only frames the plan. What is eaten each day stays in dated
//! sections (`== Thursday (2026-10-01) ==`), as in any menu, so the CLI, the
//! shopping list and other Cooklang apps read a plan like any other menu.

use chrono::{Datelike, Days, NaiveDate, Weekday};
use serde::Serialize;
use unic_langid::LanguageIdentifier;

use crate::web::menus::{extract_date, extract_meal_type, extract_time, is_meal_header};
use crate::web::templates::{MenuSection, MenuSectionItem};

/// The longest plan the planner lays out: about two months.
pub const MAX_PLAN_DAYS: u32 = 62;

/// The `plan:` block of a menu's frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFrame {
    pub start: NaiveDate,
    pub days: u32,
    /// Meals every day has a slot for, in order. May be empty.
    pub meals: Vec<String>,
}

impl PlanFrame {
    fn dates(&self) -> impl Iterator<Item = NaiveDate> + '_ {
        (0..self.days).filter_map(|offset| self.start.checked_add_days(Days::new(offset.into())))
    }
}

/// Reads the `plan:` block, or `None` when there is none or it is not one the
/// planner can lay out; the menu then shows as an ordinary menu.
pub fn plan_frame(metadata: &cooklang::Metadata) -> Option<PlanFrame> {
    let plan = metadata.get("plan")?;
    let start = NaiveDate::parse_from_str(plan.get("start")?.as_str()?, "%Y-%m-%d").ok()?;
    let days = u32::try_from(plan.get("days")?.as_u64()?).ok()?;
    if !(1..=MAX_PLAN_DAYS).contains(&days) {
        return None;
    }
    let meals = match plan.get("meals") {
        None => Vec::new(),
        Some(meals) => meals
            .as_sequence()?
            .iter()
            .filter_map(|meal| meal.as_str())
            .map(str::trim)
            .filter(|meal| !meal.is_empty())
            .map(String::from)
            .collect(),
    };
    Some(PlanFrame { start, days, meals })
}

/// A plan laid out as a calendar, one row per week.
#[derive(Debug, Clone, Serialize)]
pub struct PlanView {
    /// Column headings, in the order the rows run.
    pub weekdays: Vec<String>,
    /// Seven cells a row; `None` pads the first and last week.
    pub weeks: Vec<Vec<Option<PlanDay>>>,
    /// Sections with no date, or a date the plan does not cover.
    pub outside: Vec<MenuSection>,
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

/// Sorts the menu's `sections` into the days of `frame`.
pub fn build_plan_view(
    frame: &PlanFrame,
    sections: &[MenuSection],
    lang: &LanguageIdentifier,
) -> PlanView {
    let locale = chrono_locale(lang);
    let first_weekday = first_weekday(lang);

    let mut days: Vec<(NaiveDate, Vec<PlanMeal>)> = frame
        .dates()
        .map(|date| {
            let meals = frame
                .meals
                .iter()
                .map(|name| PlanMeal {
                    name: Some(name.clone()),
                    time: None,
                    lines: Vec::new(),
                })
                .collect();
            (date, meals)
        })
        .collect();

    let mut outside = Vec::new();
    for section in sections {
        let date = section
            .name
            .as_deref()
            .and_then(extract_date)
            .and_then(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok());
        match date.and_then(|date| days.iter_mut().find(|(day, _)| *day == date)) {
            Some((_, meals)) => add_lines(meals, &section.lines),
            None => outside.push(section.clone()),
        }
    }

    let lead = days.first().map_or(0, |(date, _)| {
        date.weekday().days_since(first_weekday) as usize
    });
    let mut cells: Vec<Option<PlanDay>> = std::iter::repeat_with(|| None).take(lead).collect();
    cells.extend(days.into_iter().map(|(date, meals)| {
        Some(PlanDay {
            date: date.format("%Y-%m-%d").to_string(),
            label: capitalize(&date.format_localized("%a %-d %b", locale).to_string()),
            meals,
        })
    }));
    while !cells.len().is_multiple_of(7) {
        cells.push(None);
    }
    let weeks = cells.chunks(7).map(<[_]>::to_vec).collect();

    // 2024-01-01 was a Monday: walk a week from there to name the columns.
    let monday = NaiveDate::from_ymd_opt(2024, 1, 1).expect("a valid date");
    let weekdays = (0..7)
        .map(|i| {
            let offset = u64::from(first_weekday.num_days_from_monday()) + i;
            let date = monday + Days::new(offset);
            capitalize(&date.format_localized("%a", locale).to_string())
        })
        .collect();

    PlanView {
        weekdays,
        weeks,
        outside,
    }
}

/// Appends a section's lines to the day's meals: a line that is only
/// `Name:` starts a meal (the rule the menu API follows), and a meal the day
/// already has, such as one of the plan's, is continued rather than repeated.
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
fn day_heading(date: NaiveDate, lang: &LanguageIdentifier) -> String {
    let weekday = date.format_localized("%A", chrono_locale(lang)).to_string();
    format!("{} ({})", capitalize(&weekday), date.format("%Y-%m-%d"))
}

/// What a new plan titled `title` starts with: the frontmatter, then every day
/// with an empty bullet under each meal, ready to fill in the editor.
#[cfg(feature = "server")]
pub fn plan_starter(
    title: &str,
    servings: u32,
    frame: &PlanFrame,
    lang: &LanguageIdentifier,
) -> String {
    use std::fmt::Write;

    // JSON strings are valid YAML double-quoted scalars.
    let meals = frame
        .meals
        .iter()
        .map(|meal| serde_json::to_string(meal).expect("a string serializes"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!(
        "---\ntitle: {title}\nservings: {servings}\nplan:\n  start: {}\n  days: {}\n  meals: [{meals}]\n---\n",
        frame.start.format("%Y-%m-%d"),
        frame.days,
    );
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

    fn metadata(yaml: &str) -> cooklang::Metadata {
        let text = format!("---\n{yaml}\n---\n\n== Day ==\n\n- @eggs{{}}\n");
        crate::util::PARSER
            .parse(&text)
            .into_output()
            .expect("the menu parses")
            .metadata
    }

    fn lang(tag: &str) -> LanguageIdentifier {
        tag.parse().unwrap()
    }

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
        }]
    }

    fn section(name: &str, lines: Vec<Vec<MenuSectionItem>>) -> MenuSection {
        MenuSection {
            name: Some(name.to_string()),
            lines,
        }
    }

    fn frame(start: &str, days: u32, meals: &[&str]) -> PlanFrame {
        PlanFrame {
            start: date(start),
            days,
            meals: meals.iter().map(|meal| meal.to_string()).collect(),
        }
    }

    #[test]
    fn reads_the_plan_block() {
        let frame = plan_frame(&metadata(
            "plan:\n  start: 2026-10-01\n  days: 10\n  meals: [Breakfast, \"Dinner\"]",
        ));
        assert_eq!(
            frame,
            Some(PlanFrame {
                start: date("2026-10-01"),
                days: 10,
                meals: vec!["Breakfast".into(), "Dinner".into()],
            })
        );
    }

    #[test]
    fn meals_are_optional() {
        let frame = plan_frame(&metadata("plan:\n  start: 2026-10-01\n  days: 7")).unwrap();
        assert!(frame.meals.is_empty());
    }

    #[test]
    fn a_plan_block_the_planner_cannot_lay_out_is_ignored() {
        for yaml in [
            "servings: 2",
            "plan: yes",
            "plan:\n  start: 2026-02-30\n  days: 7",
            "plan:\n  start: next week\n  days: 7",
            "plan:\n  start: 2026-10-01\n  days: 0",
            "plan:\n  start: 2026-10-01\n  days: 63",
            "plan:\n  start: 2026-10-01\n  days: -1",
            "plan:\n  start: 2026-10-01",
            "plan:\n  start: 2026-10-01\n  days: 7\n  meals: Dinner",
        ] {
            assert_eq!(plan_frame(&metadata(yaml)), None, "{yaml}");
        }
    }

    #[test]
    fn a_wednesday_start_leaves_blank_cells_before_it() {
        let view = build_plan_view(&frame("2026-10-07", 10, &[]), &[], &lang("fr-FR"));

        assert_eq!(view.weeks.len(), 2);
        assert!(view.weeks.iter().all(|week| week.len() == 7));
        let dates: Vec<Option<&str>> = view
            .weeks
            .iter()
            .flatten()
            .map(|cell| cell.as_ref().map(|day| day.date.as_str()))
            .collect();
        assert_eq!(&dates[..3], &[None, None, Some("2026-10-07")]);
        assert_eq!(dates[11], Some("2026-10-16"));
        assert!(dates[12..].iter().all(Option::is_none));
        assert_eq!(view.weekdays[0], "Lun.");
    }

    #[test]
    fn weeks_start_on_sunday_in_american_english() {
        let view = build_plan_view(&frame("2026-10-07", 10, &[]), &[], &lang("en-US"));

        assert_eq!(view.weekdays[0], "Sun");
        let first_week = &view.weeks[0];
        assert!(first_week[..3].iter().all(Option::is_none));
        assert_eq!(first_week[3].as_ref().unwrap().label, "Wed 7 Oct");
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
            section("Saturday (2026-10-03)", vec![recipe("./Toast")]),
            section("Day 1", vec![recipe("./Undated")]),
            section("Much later (2027-01-01)", vec![recipe("./Late")]),
        ];
        let view = build_plan_view(
            &frame("2026-10-01", 3, &["Breakfast", "Dinner"]),
            &sections,
            &lang("en-GB"),
        );
        let days: Vec<&PlanDay> = view.weeks.iter().flatten().flatten().collect();

        let meals = |day: &PlanDay| -> Vec<(Option<String>, usize)> {
            day.meals
                .iter()
                .map(|meal| (meal.name.clone(), meal.lines.len()))
                .collect()
        };
        // The plan's meals come first, even empty, then the ones the text adds.
        assert_eq!(
            meals(days[0]),
            [(Some("Breakfast".into()), 0), (Some("Dinner".into()), 0)]
        );
        assert_eq!(
            meals(days[1]),
            [
                (Some("Breakfast".into()), 1),
                (Some("Dinner".into()), 2),
                (Some("Snacks".into()), 1),
            ]
        );
        assert_eq!(days[1].meals[0].time.as_deref(), Some("08:30"));
        // Lines before any meal come first, under no heading.
        assert_eq!(meals(days[2])[0], (None, 1));

        let outside: Vec<_> = view
            .outside
            .iter()
            .map(|s| s.name.clone().unwrap())
            .collect();
        assert_eq!(outside, ["Day 1", "Much later (2027-01-01)"]);
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
        let frame = frame("2026-10-07", 3, &["Breakfast", "Dinner"]);
        let text = plan_starter("October", 2, &frame, &lang("en-US"));
        let recipe = crate::util::PARSER
            .parse(&text)
            .into_output()
            .expect("the plan parses");

        assert_eq!(plan_frame(&recipe.metadata), Some(frame));
        assert_eq!(
            recipe.metadata.get("servings").and_then(|v| v.as_u64()),
            Some(2)
        );
        let dates: Vec<Option<String>> = recipe
            .sections
            .iter()
            .map(|section| section.name.as_deref().and_then(extract_date))
            .collect();
        assert_eq!(
            dates,
            [
                Some("2026-10-07".into()),
                Some("2026-10-08".into()),
                Some("2026-10-09".into()),
            ]
        );
        assert!(
            text.contains("== Thursday (2026-10-08) ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- \n")
        );
    }
}
