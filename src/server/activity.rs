//! The activity log: a line on standard output for every change made through
//! the server, saying when, who, and what.
//!
//! ```text
//! 2026-09-27 08:41:56 alice created recipe "Soups/Pea soup.cook"
//! 2026-09-27 08:42:10 alice added "Soups/Pea soup.cook" to the shopping list
//! 2026-09-27 08:43:02 guest failed to sign in
//! ```
//!
//! Printed rather than traced: `tracing` writes to standard error and shows
//! only warnings unless `-v` is given, while this is for whoever runs the
//! server to read, in the terminal or `docker logs`, next to the startup
//! lines. The actor is the signed-in user, or `guest` when sign-in is off.
//!
//! Names and paths come from requests, so they are shown Debug-quoted: a
//! newline in a file name cannot forge a line of its own.

use crate::web::viewer::Viewer;
use camino::Utf8Path;
use std::fmt::{self, Display};

/// Prints that `viewer` did `action`.
pub fn record(viewer: &Viewer, action: impl Display) {
    let actor = if viewer.is_signed_in() {
        viewer.username()
    } else {
        GUEST
    };
    record_as(actor, action);
}

/// Prints that `actor` did `action`, for the sign-in handlers, which know
/// who someone is before their request says so.
pub fn record_as(actor: &str, action: impl Display) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{}", line(&now, actor, &action));
}

/// Who a change is attributed to when nobody is signed in.
pub const GUEST: &str = "guest";

fn line(time: &dyn Display, actor: &str, action: &dyn Display) -> String {
    format!("{time} {actor} {action}")
}

/// Text from a request, quoted and escaped.
pub fn quoted(text: &str) -> impl Display + '_ {
    Quoted(text)
}

struct Quoted<'a>(&'a str);

impl Display for Quoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// A recipe or menu file, as `recipe "Soups/Pea soup.cook"` or
/// `menu "Week.menu"`, named by its path under the recipe directory.
pub fn file<'a>(base: &Utf8Path, path: &'a Utf8Path) -> impl Display + 'a {
    File(path.strip_prefix(base).unwrap_or(path))
}

struct File<'a>(&'a Utf8Path);

impl Display for File<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = if self.0.extension() == Some("menu") {
            "menu"
        } else {
            "recipe"
        };
        write!(f, "{kind} {}", quoted(self.0.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_reads_as_a_sentence() {
        assert_eq!(
            line(&"2026-09-27 08:41:56", "alice", &"signed in"),
            "2026-09-27 08:41:56 alice signed in"
        );
    }

    #[test]
    fn files_are_named_by_kind_and_relative_path() {
        let base = Utf8Path::new("/recipes");
        assert_eq!(
            file(base, Utf8Path::new("/recipes/Soups/Pea soup.cook")).to_string(),
            "recipe \"Soups/Pea soup.cook\""
        );
        assert_eq!(
            file(base, Utf8Path::new("/recipes/Week.menu")).to_string(),
            "menu \"Week.menu\""
        );
        // Outside the base: shown whole rather than dropped.
        assert_eq!(
            file(base, Utf8Path::new("/elsewhere/A.cook")).to_string(),
            "recipe \"/elsewhere/A.cook\""
        );
    }

    #[test]
    fn request_text_cannot_break_the_line() {
        let forged = "Soup\n2026-09-27 08:00:00 admin deleted everything";
        let shown = quoted(forged).to_string();
        assert!(!shown.contains('\n'), "{shown}");
        assert_eq!(quoted("Crème brûlée").to_string(), "\"Crème brûlée\"");
    }
}
