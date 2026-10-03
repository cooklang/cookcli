//! Changes to a meal plan's text made from its calendar: add a line to a
//! day's meal, remove one, move or copy one to another day or meal.
//!
//! The `.menu` text stays the source of truth. Only the lines a change is
//! about are touched; frontmatter, comments, notes and every other section
//! keep their bytes.
//!
//! The file is read the way the calendar reads it (`crate::web::plan`): a
//! section whose name holds a `YYYY-MM-DD` date is that day; in it, a line
//! that is only `Name:` (with an optional ` \`) starts a meal, which runs to
//! the next meal or section. Text after the date in the section's name, as in
//! `= 2026-10-08 Dinner`, names the meal of its lines before the first such
//! line. The meal's lines are its `- ` bullets; an empty bullet, as a new
//! plan has under each meal, is not one.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use unic_langid::LanguageIdentifier;

use crate::web::menus::{extract_meal_type, is_meal_header};
use crate::web::plan::section_date;

/// Why a change could not be made.
#[derive(Debug, PartialEq, Eq)]
pub enum EditError {
    /// The line is not where the page said, or reads differently: the file
    /// changed since the page was loaded.
    Stale,
    /// A meal name that cannot be written as a meal heading.
    BadMeal,
}

/// A plan's text, one entry a line.
pub struct PlanText {
    lines: Vec<String>,
    crlf: bool,
    /// The first line after the frontmatter.
    body: usize,
}

/// A day's section: its heading's line and the line after its last one.
struct Section {
    header: usize,
    end: usize,
    date: Option<NaiveDate>,
    /// The meal the section's name gives, as in `= 2026-10-08 Dinner`.
    meal: Option<String>,
}

/// A meal in a section: its heading's line (the section's own, for the meal
/// the section's name gives) and its bullets.
struct Meal {
    header: usize,
    name: String,
    items: Vec<usize>,
    stubs: Vec<usize>,
}

impl PlanText {
    pub fn parse(text: &str) -> Self {
        let crlf = text.contains("\r\n");
        let lines: Vec<String> = text
            .split('\n')
            .map(|line| {
                if crlf {
                    line.strip_suffix('\r').unwrap_or(line).to_string()
                } else {
                    line.to_string()
                }
            })
            .collect();
        let body = if lines.first().map(|line| line.trim_end()) == Some("---") {
            lines
                .iter()
                .skip(1)
                .position(|line| line.trim_end() == "---")
                .map_or(0, |closing| closing + 2)
        } else {
            0
        };
        Self { lines, crlf, body }
    }

    /// The section names of the plan, as `crate::web::plan::is_plan` reads
    /// them.
    pub fn section_names(&self) -> Vec<&str> {
        (self.body..self.lines.len())
            .filter(|&i| is_section_heading(&self.lines[i]))
            .map(|i| section_name(&self.lines[i]))
            .collect()
    }

    /// The meals the plan's days name, in the calendar's order
    /// (`crate::web::plan::meal_slots`): the order a new meal keeps.
    pub fn meal_order(&self) -> Vec<String> {
        let mut days: BTreeMap<NaiveDate, Vec<String>> = BTreeMap::new();
        for section in self.sections() {
            let Some(date) = section.date else { continue };
            let names = days.entry(date).or_default();
            for meal in self.meals(&section) {
                if !names.contains(&meal.name) {
                    names.push(meal.name);
                }
            }
        }
        crate::web::plan::meal_slots(days.values().map(|names| names.iter().map(String::as_str)))
    }

    pub fn render(&self) -> String {
        self.lines.join(if self.crlf { "\r\n" } else { "\n" })
    }

    /// The text of each line of `meal` on `date`, in the order the calendar
    /// shows them: without the bullet and the ` \` that joins the next line.
    pub fn items(&self, date: NaiveDate, meal: &str) -> Vec<String> {
        self.meal_lines(date, meal)
            .into_iter()
            .map(|line| item_text(&self.lines[line]))
            .collect()
    }

    /// Adds `item` (text after the bullet, e.g. `@./Risotto{}`) at the end of
    /// `meal` on `date`, making the day's section and the meal when missing.
    /// `meals` is the plan's order of meals (`meal_order`), which a new meal
    /// keeps.
    pub fn add(
        &mut self,
        date: NaiveDate,
        meal: &str,
        item: &str,
        meals: &[String],
        lang: &LanguageIdentifier,
    ) -> Result<(), EditError> {
        if !valid_meal(meal) {
            return Err(EditError::BadMeal);
        }
        let bullet = format!("- {}", item.trim());

        let sections = self.sections();
        let days: Vec<&Section> = sections.iter().filter(|s| s.date == Some(date)).collect();

        // The meal's last block on that day: add after its last bullet.
        let found = days
            .iter()
            .flat_map(|section| self.meals(section))
            .rfind(|m| m.name == meal);
        if let Some(found) = found {
            if found.items.is_empty() {
                if let Some(&stub) = found.stubs.first() {
                    // A new plan's empty bullet: fill it in.
                    let join = self.lines[stub].trim_end().ends_with('\\');
                    self.lines[stub] = if join { format!("{bullet} \\") } else { bullet };
                    return Ok(());
                }
            }
            match found.items.iter().chain(&found.stubs).copied().max() {
                Some(last) => {
                    join_next(&mut self.lines[last]);
                    self.lines.insert(last + 1, bullet);
                }
                // A meal the section's name gives, with nothing in it yet:
                // the bullet goes under the section's heading.
                None if is_section_heading(&self.lines[found.header]) => {
                    self.insert_block(found.header + 1, vec![bullet]);
                }
                None => {
                    join_next(&mut self.lines[found.header]);
                    self.lines.insert(found.header + 1, bullet);
                }
            }
            return Ok(());
        }

        let block = vec![format!("{meal}: \\"), bullet];
        match days.last() {
            Some(section) => {
                // A new meal in the day: before the first that comes after it
                // in the plan's order, else last.
                let rank = |name: &str| meals.iter().position(|m| m == name);
                let before = rank(meal).and_then(|mine| {
                    self.meals(section)
                        .into_iter()
                        .find(|m| rank(&m.name).is_some_and(|theirs| theirs > mine))
                });
                let at = match before {
                    Some(m) => m.header,
                    None => self.after_filled(section.header, section.end),
                };
                self.insert_block(at, block);
            }
            None => {
                // A new day: before the first later day, else after the last
                // earlier one, else at the end.
                let at = match sections.iter().find(|s| s.date.is_some_and(|d| d > date)) {
                    Some(later) => later.header,
                    None => match sections.iter().rev().find(|s| s.date.is_some()) {
                        Some(earlier) => self.after_filled(earlier.header, earlier.end),
                        None => self.after_filled(self.body, self.lines.len()),
                    },
                };
                let heading = crate::web::plan::day_heading(date, lang);
                let mut day = vec![format!("== {heading} =="), String::new()];
                day.extend(block);
                self.insert_block(at, day);
            }
        }
        Ok(())
    }

    /// Removes line `index` of `meal` on `date`, which must read `expected`;
    /// returns its text.
    pub fn remove(
        &mut self,
        date: NaiveDate,
        meal: &str,
        index: usize,
        expected: &str,
    ) -> Result<String, EditError> {
        let line = self.find(date, meal, index, expected)?;
        let text = item_text(&self.lines[line]);

        let owner = self
            .sections()
            .iter()
            .filter(|s| s.date == Some(date))
            .flat_map(|section| self.meals(section))
            .find(|m| m.items.contains(&line))
            .expect("the line was found in one of the day's meals");
        let bullets: Vec<usize> = owner.items.iter().chain(&owner.stubs).copied().collect();
        let was_last = bullets.iter().all(|&other| other <= line);

        if owner.items.len() == 1 && owner.stubs.is_empty() {
            // The meal's only line: leave an empty bullet, as a new plan has,
            // so the meal keeps its place.
            self.lines[line] = "- ".to_string();
            return Ok(text);
        }
        self.lines.remove(line);
        if was_last {
            // The line before now ends the meal: it joins nothing any more.
            if let Some(&previous) = bullets.iter().filter(|&&other| other < line).max() {
                unjoin(&mut self.lines[previous]);
            }
        }
        Ok(text)
    }

    /// The line of the file that is line `index` of `meal` on `date`, if it
    /// reads `expected`.
    fn find(
        &self,
        date: NaiveDate,
        meal: &str,
        index: usize,
        expected: &str,
    ) -> Result<usize, EditError> {
        let line = *self
            .meal_lines(date, meal)
            .get(index)
            .ok_or(EditError::Stale)?;
        if item_text(&self.lines[line]) != expected.trim() {
            return Err(EditError::Stale);
        }
        Ok(line)
    }

    /// Checks that line `index` of `meal` on `date` reads `expected`, and
    /// returns its text.
    pub fn read(
        &self,
        date: NaiveDate,
        meal: &str,
        index: usize,
        expected: &str,
    ) -> Result<String, EditError> {
        let line = self.find(date, meal, index, expected)?;
        Ok(item_text(&self.lines[line]))
    }

    fn meal_lines(&self, date: NaiveDate, meal: &str) -> Vec<usize> {
        self.sections()
            .iter()
            .filter(|s| s.date == Some(date))
            .flat_map(|section| self.meals(section))
            .filter(|m| m.name == meal)
            .flat_map(|m| m.items)
            .collect()
    }

    fn sections(&self) -> Vec<Section> {
        let headers: Vec<usize> = (self.body..self.lines.len())
            .filter(|&i| is_section_heading(&self.lines[i]))
            .collect();
        headers
            .iter()
            .enumerate()
            .map(|(n, &header)| Section {
                header,
                end: headers.get(n + 1).copied().unwrap_or(self.lines.len()),
                date: None,
                meal: None,
            })
            .map(|mut section| {
                if let Some((date, meal)) = section_date(section_name(&self.lines[section.header]))
                {
                    section.date = Some(date);
                    section.meal = meal.map(|header| extract_meal_type(&header));
                }
                section
            })
            .collect()
    }

    fn meals(&self, section: &Section) -> Vec<Meal> {
        let mut headers: Vec<(usize, String)> = (section.header + 1..section.end)
            .filter_map(|i| meal_heading(&self.lines[i]).map(|name| (i, name)))
            .collect();
        if let Some(name) = &section.meal {
            headers.insert(0, (section.header, name.clone()));
        }
        headers
            .iter()
            .enumerate()
            .map(|(n, (header, name))| {
                let end = headers.get(n + 1).map_or(section.end, |(next, _)| *next);
                let bullets = (header + 1..end).filter(|&i| is_bullet(&self.lines[i]));
                let (items, stubs) = bullets.partition(|&i| !item_text(&self.lines[i]).is_empty());
                Meal {
                    header: *header,
                    name: name.clone(),
                    items,
                    stubs,
                }
            })
            .collect()
    }

    /// Where a block goes after everything in `from..to`: the line after the
    /// last one with something on it, or `from` when there is none.
    fn after_filled(&self, from: usize, to: usize) -> usize {
        (from..to)
            .rev()
            .find(|&i| !self.lines[i].trim().is_empty())
            .map_or(from, |i| i + 1)
    }

    /// Inserts `block` at line `at`, with a blank line on either side unless
    /// one is already there.
    fn insert_block(&mut self, at: usize, block: Vec<String>) {
        let mut lines = Vec::with_capacity(block.len() + 2);
        if at > 0 && !self.lines[at - 1].trim().is_empty() {
            lines.push(String::new());
        }
        lines.extend(block);
        match self.lines.get(at) {
            Some(next) if !next.trim().is_empty() => lines.push(String::new()),
            // The end of a file without a final newline: add one.
            None => lines.push(String::new()),
            _ => {}
        }
        self.lines.splice(at..at, lines);
    }
}

/// Whether `meal` can be written as a meal heading and read back as the same
/// name.
pub fn valid_meal(meal: &str) -> bool {
    let meal = meal.trim();
    !meal.is_empty()
        && meal.chars().count() <= 60
        && !meal.starts_with(['-', '>', '='])
        && !meal.contains([
            '\n', '\r', ':', '\\', '@', '#', '~', '{', '}', '[', ']', '(', ')',
        ])
}

fn is_section_heading(line: &str) -> bool {
    line.trim_start().starts_with('=')
}

fn section_name(line: &str) -> &str {
    line.trim().trim_matches('=').trim()
}

fn is_bullet(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with('-') && !line.starts_with("--")
}

/// The meal a line starts, when it is only `Name:` (with an optional ` \`).
fn meal_heading(line: &str) -> Option<String> {
    let text = line.trim();
    if text.starts_with(['-', '>', '=', '@']) {
        return None;
    }
    let text = text.strip_suffix('\\').unwrap_or(text).trim_end();
    is_meal_header(text).then(|| extract_meal_type(text))
}

/// A bullet's text: without the bullet and the ` \` that joins the next line.
fn item_text(line: &str) -> String {
    let text = line.trim();
    let text = text.strip_prefix('-').unwrap_or(text);
    let text = text.trim();
    text.strip_suffix('\\').unwrap_or(text).trim().to_string()
}

/// Ends `line` with ` \`, so the next line stays in the same meal.
fn join_next(line: &mut String) {
    if !line.trim_end().ends_with('\\') {
        let kept = line.trim_end().len();
        line.truncate(kept);
        line.push_str(" \\");
    }
}

/// Drops the ` \` that ends `line`.
fn unjoin(line: &mut String) {
    let trimmed = line.trim_end();
    if let Some(rest) = trimmed.strip_suffix('\\') {
        *line = rest.trim_end().to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn en() -> LanguageIdentifier {
        "en-US".parse().unwrap()
    }

    fn meals(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn add(text: &str, day: &str, meal: &str, item: &str) -> String {
        let mut plan = PlanText::parse(text);
        plan.add(
            date(day),
            meal,
            item,
            &meals(&["Breakfast", "Lunch", "Dinner"]),
            &en(),
        )
        .unwrap();
        plan.render()
    }

    const PLAN: &str = "---\ntitle: Week\n---\n\n\
== Wednesday (2026-10-07) ==\n\n\
Breakfast (08:30): \\\n- @./Pancakes{} \\\n- @coffee{1%cup}\n\n\
-- leftovers tomorrow\n\n\
Dinner: \\\n- @./Risotto{2%servings}\n\n\
== Friday (2026-10-09) ==\n\n\
Dinner: \\\n- \n";

    #[test]
    fn a_new_plans_empty_bullet_is_filled_in() {
        let frame = crate::web::plan::PlanFrame {
            start: date("2026-10-07"),
            days: 2,
            meals: meals(&["Breakfast", "Dinner"]),
        };
        let text = crate::web::plan::plan_starter("Week", 2, &frame, &en());
        let mut plan = PlanText::parse(&text);

        plan.add(
            date("2026-10-08"),
            "Dinner",
            "@./Risotto{2%servings}",
            &frame.meals,
            &en(),
        )
        .unwrap();

        assert_eq!(
            plan.render(),
            text.replace(
                "== Thursday (2026-10-08) ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- \n",
                "== Thursday (2026-10-08) ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- @./Risotto{2%servings}\n"
            )
        );
        assert_eq!(
            plan.items(date("2026-10-08"), "Dinner"),
            ["@./Risotto{2%servings}"]
        );
        assert!(plan.items(date("2026-10-08"), "Breakfast").is_empty());
    }

    #[test]
    fn a_line_is_added_after_the_meals_last_one() {
        assert_eq!(
            add(PLAN, "2026-10-07", "Breakfast", "@./Toast{}"),
            PLAN.replace("- @coffee{1%cup}\n", "- @coffee{1%cup} \\\n- @./Toast{}\n")
        );
    }

    #[test]
    fn items_read_the_meal_as_the_calendar_shows_it() {
        let plan = PlanText::parse(PLAN);
        assert_eq!(
            plan.items(date("2026-10-07"), "Breakfast"),
            ["@./Pancakes{}", "@coffee{1%cup}"]
        );
        assert_eq!(
            plan.items(date("2026-10-07"), "Dinner"),
            ["@./Risotto{2%servings}"]
        );
        assert!(plan.items(date("2026-10-08"), "Dinner").is_empty());
    }

    #[test]
    fn a_missing_meal_goes_where_the_plan_orders_it() {
        assert_eq!(
            add(PLAN, "2026-10-07", "Lunch", "@./Salad{}"),
            PLAN.replace(
                "Dinner: \\\n- @./Risotto",
                "Lunch: \\\n- @./Salad{}\n\nDinner: \\\n- @./Risotto"
            )
        );
        // A meal the plan does not list goes last.
        assert_eq!(
            add(PLAN, "2026-10-07", "Snacks", "@nuts{}"),
            PLAN.replace(
                "- @./Risotto{2%servings}\n\n== Friday",
                "- @./Risotto{2%servings}\n\nSnacks: \\\n- @nuts{}\n\n== Friday"
            )
        );
    }

    #[test]
    fn a_missing_day_goes_between_the_days_around_it() {
        assert_eq!(
            add(PLAN, "2026-10-08", "Dinner", "@./Soup{}"),
            PLAN.replace(
                "== Friday",
                "== Thursday (2026-10-08) ==\n\nDinner: \\\n- @./Soup{}\n\n== Friday"
            )
        );
        // After the last day, at the end.
        assert_eq!(
            add(PLAN, "2026-10-10", "Lunch", "@./Soup{}"),
            format!("{PLAN}\n== Saturday (2026-10-10) ==\n\nLunch: \\\n- @./Soup{{}}\n")
        );
        // A plan with no days yet.
        assert_eq!(
            add("---\nplan: {}\n---\n", "2026-10-10", "Lunch", "@./Soup{}"),
            "---\nplan: {}\n---\n\n== Saturday (2026-10-10) ==\n\nLunch: \\\n- @./Soup{}\n"
        );
    }

    #[test]
    fn a_day_before_undated_sections_stays_with_the_days() {
        let text = "== Monday (2026-10-05) ==\n\nDinner: \\\n- @./Soup{}\n\n== Leftovers ==\n\n- @./Beans{}\n";
        assert_eq!(
            add(text, "2026-10-06", "Dinner", "@./Stew{}"),
            text.replace(
                "== Leftovers",
                "== Tuesday (2026-10-06) ==\n\nDinner: \\\n- @./Stew{}\n\n== Leftovers"
            )
        );
    }

    #[test]
    fn removing_the_last_line_unjoins_the_one_before() {
        let mut plan = PlanText::parse(PLAN);
        let removed = plan
            .remove(date("2026-10-07"), "Breakfast", 1, "@coffee{1%cup}")
            .unwrap();
        assert_eq!(removed, "@coffee{1%cup}");
        assert_eq!(
            plan.render(),
            PLAN.replace(
                "- @./Pancakes{} \\\n- @coffee{1%cup}\n",
                "- @./Pancakes{}\n"
            )
        );
    }

    #[test]
    fn removing_a_line_in_the_middle_keeps_the_join() {
        let mut plan = PlanText::parse(PLAN);
        plan.remove(date("2026-10-07"), "Breakfast", 0, "@./Pancakes{}")
            .unwrap();
        assert_eq!(plan.render(), PLAN.replace("- @./Pancakes{} \\\n", ""));
    }

    #[test]
    fn removing_a_meals_only_line_leaves_an_empty_bullet() {
        let mut plan = PlanText::parse(PLAN);
        plan.remove(date("2026-10-07"), "Dinner", 0, "@./Risotto{2%servings}")
            .unwrap();
        assert_eq!(
            plan.render(),
            PLAN.replace("- @./Risotto{2%servings}\n", "- \n")
        );
        assert!(plan.items(date("2026-10-07"), "Dinner").is_empty());
        // And an add fills it again.
        plan.add(date("2026-10-07"), "Dinner", "@./Soup{}", &[], &en())
            .unwrap();
        assert_eq!(
            plan.render(),
            PLAN.replace("@./Risotto{2%servings}", "@./Soup{}")
        );
    }

    #[test]
    fn a_line_that_moved_or_changed_is_refused() {
        let mut plan = PlanText::parse(PLAN);
        assert_eq!(
            plan.remove(date("2026-10-07"), "Breakfast", 0, "@coffee{1%cup}"),
            Err(EditError::Stale)
        );
        assert_eq!(
            plan.remove(date("2026-10-07"), "Breakfast", 2, "@coffee{1%cup}"),
            Err(EditError::Stale)
        );
        assert_eq!(
            plan.remove(date("2026-10-08"), "Breakfast", 0, "@coffee{1%cup}"),
            Err(EditError::Stale)
        );
        assert_eq!(plan.render(), PLAN);
    }

    #[test]
    fn a_meal_name_that_would_not_read_back_is_refused() {
        let mut plan = PlanText::parse(PLAN);
        for meal in [
            "",
            "Tea: time",
            "Dinner\n== Evil ==",
            "- Dinner",
            "Late (22:00)",
        ] {
            assert_eq!(
                plan.add(date("2026-10-07"), meal, "@x{}", &[], &en()),
                Err(EditError::BadMeal),
                "{meal:?}"
            );
        }
        assert_eq!(plan.render(), PLAN);
    }

    #[test]
    fn windows_line_endings_are_kept() {
        let text = PLAN.replace('\n', "\r\n");
        let out = add(&text, "2026-10-07", "Dinner", "@./Salad{}");
        assert_eq!(
            out,
            text.replace(
                "- @./Risotto{2%servings}\r\n",
                "- @./Risotto{2%servings} \\\r\n- @./Salad{}\r\n"
            )
        );
    }

    #[test]
    fn two_sections_of_one_day_read_as_one() {
        let text = "== Mon (2026-10-05) ==\n\nDinner: \\\n- @./A{}\n\n== Mon again (2026-10-05) ==\n\nDinner: \\\n- @./B{}\n";
        let mut plan = PlanText::parse(text);
        assert_eq!(
            plan.items(date("2026-10-05"), "Dinner"),
            ["@./A{}", "@./B{}"]
        );
        plan.remove(date("2026-10-05"), "Dinner", 1, "@./B{}")
            .unwrap();
        assert_eq!(plan.render(), text.replace("- @./B{}", "- "));
        // An add goes to the day's last block of the meal.
        plan.add(date("2026-10-05"), "Dinner", "@./C{}", &[], &en())
            .unwrap();
        assert_eq!(plan.render(), text.replace("- @./B{}", "- @./C{}"));
    }

    #[test]
    fn a_comment_between_lines_is_left_alone() {
        let text = "== Mon (2026-10-05) ==\n\nDinner: \\\n- @./A{} \\\n-- a note\n- @./B{}\n";
        let mut plan = PlanText::parse(text);
        assert_eq!(
            plan.items(date("2026-10-05"), "Dinner"),
            ["@./A{}", "@./B{}"]
        );
        plan.remove(date("2026-10-05"), "Dinner", 1, "@./B{}")
            .unwrap();
        assert_eq!(
            plan.render(),
            "== Mon (2026-10-05) ==\n\nDinner: \\\n- @./A{}\n-- a note\n"
        );
    }

    #[test]
    fn a_section_named_after_its_meal_is_that_meal() {
        let text = "= 2026-10-05 Dinner\n\n- @./A{}\n\n= 2026-10-06 Lunch\n\n= 2026-10-07\n";
        let mut plan = PlanText::parse(text);
        assert_eq!(plan.items(date("2026-10-05"), "Dinner"), ["@./A{}"]);

        plan.add(date("2026-10-05"), "Dinner", "@./B{}", &[], &en())
            .unwrap();
        assert_eq!(
            plan.render(),
            text.replace("- @./A{}\n", "- @./A{} \\\n- @./B{}\n")
        );

        // An empty one gets its first line under the heading.
        let mut plan = PlanText::parse(text);
        plan.add(date("2026-10-06"), "Lunch", "@./C{}", &[], &en())
            .unwrap();
        assert_eq!(
            plan.render(),
            text.replace(
                "= 2026-10-06 Lunch\n\n",
                "= 2026-10-06 Lunch\n\n- @./C{}\n\n"
            )
        );
        assert_eq!(plan.items(date("2026-10-06"), "Lunch"), ["@./C{}"]);
    }

    #[test]
    fn the_meal_order_is_the_calendars() {
        let plan = PlanText::parse(
            "== Mon (2026-10-05) ==\n\nDinner: \\\n- \n\n\
             = 2026-10-06 Lunch\n\nDinner: \\\n- \n\n\
             == Snacks ==\n\nSnacks: \\\n- \n",
        );
        // The undated section names no meal of the plan.
        assert_eq!(plan.meal_order(), ["Lunch", "Dinner"]);
        assert_eq!(
            plan.section_names(),
            ["Mon (2026-10-05)", "2026-10-06 Lunch", "Snacks"]
        );
    }
}
