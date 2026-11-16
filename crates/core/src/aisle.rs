//! Checks on `aisle.conf` that its parser does not make.
//!
//! The aisle file's only comment is `//`: anything else on a line is an aisle
//! or an ingredient. That is easy to miss — `pantry.conf` beside it is TOML,
//! where comments start with `#`, and `.shopping-list` and recipes use `--` —
//! so a comment written the other way quietly becomes an ingredient, or the
//! end of a name a recipe then no longer matches.

use crate::diagnostic::{Diagnostic, Location, Span};
use camino::Utf8Path;

/// A line of an aisle file that looks like a comment but is read as an
/// ingredient.
///
/// `#[non_exhaustive]` because this is an output type consumers read rather
/// than construct.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentLookalike {
    /// The line's number, counted from 1.
    pub line: usize,
    /// What the parser reads on that line: the line trimmed, without any `//`
    /// comment it ends with.
    pub text: String,
    /// Where [`text`](CommentLookalike::text) is in the file.
    pub span: Span,
}

impl CommentLookalike {
    /// This line as a warning, located in `file` when the text came from one.
    pub fn diagnostic(&self, file: Option<&Utf8Path>) -> Diagnostic {
        let mut diagnostic = Diagnostic::warning(format!(
            "line {} is read as an ingredient: {} (comments in aisle.conf start with //)",
            self.line, self.text
        ));
        diagnostic.location = Some(Location {
            file: file.map(ToOwned::to_owned),
            span: Some(self.span),
        });
        diagnostic
    }
}

/// The lines of an aisle file that look like a `#` or `--` comment but are
/// read as an ingredient, in file order.
///
/// A line is one when one of its names starts with `#` or `--`, or has a word
/// that does: `# dairy`, `-- fruit first`, `tomato  # cherry ones are fine`.
/// `[aisle]` lines are left alone — the parser takes the name between the
/// brackets as it is — and so is anything after `//`, which is a comment.
pub fn comment_lookalikes(text: &str) -> Vec<CommentLookalike> {
    let mut found = Vec::new();
    let mut offset = 0;
    for (index, raw) in text.split_inclusive('\n').enumerate() {
        let start = offset;
        offset += raw.len();

        // As `cooklang::aisle::parse` reads a line.
        let code = match raw.find("//") {
            Some(at) => &raw[..at],
            None => raw,
        };
        let trimmed = code.trim_ascii();
        if trimmed.is_empty() || (trimmed.starts_with('[') && trimmed.ends_with(']')) {
            continue;
        }
        if !trimmed.split('|').any(looks_like_comment) {
            continue;
        }
        let lead = code.len() - code.trim_ascii_start().len();
        found.push(CommentLookalike {
            line: index + 1,
            text: trimmed.to_string(),
            span: Span::from(start + lead..start + lead + trimmed.len()),
        });
    }
    found
}

fn looks_like_comment(name: &str) -> bool {
    name.split_whitespace()
        .any(|word| word.starts_with('#') || word.starts_with("--"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<(usize, &str)> {
        comment_lookalikes(text)
            .into_iter()
            .map(|found| {
                let span = found.span.start..found.span.end;
                assert_eq!(
                    &text[span.clone()],
                    found.text,
                    "span of line {}",
                    found.line
                );
                (found.line, &text[span])
            })
            .collect()
    }

    #[test]
    fn finds_hash_and_dash_comments() {
        let text = "# My aisles\n[produce]\n  # fruit first\napples\n-- dash\n--dash\n#tight\n";
        assert_eq!(
            lines(text),
            [
                (1, "# My aisles"),
                (3, "# fruit first"),
                (5, "-- dash"),
                (6, "--dash"),
                (7, "#tight"),
            ]
        );
    }

    #[test]
    fn finds_a_comment_at_the_end_of_a_line() {
        let text =
            "[produce]\ntomatoes | tomato  # cherry ones are fine\nbasil|#fresh\n[dairy] # cold\n";
        assert_eq!(
            lines(text),
            [
                (2, "tomatoes | tomato  # cherry ones are fine"),
                (3, "basil|#fresh"),
                (4, "[dairy] # cold"),
            ]
        );
    }

    #[test]
    fn leaves_real_comments_aisles_and_names_alone() {
        let text = "// # not read\n[# aisle]\nsalt // -- not read either\ncrème fraîche\nhalf-and-half\nbird's-eye chilli\n\n";
        assert_eq!(lines(text), []);
    }

    #[test]
    fn counts_lines_and_bytes_across_crlf_and_a_missing_final_newline() {
        let text = "[produce]\r\nnaïve\r\n# é\r\n# last";
        assert_eq!(lines(text), [(3, "# é"), (4, "# last")]);
    }

    #[test]
    fn the_warning_names_the_line_and_where_comments_start() {
        let found = &comment_lookalikes("[a]\n# b\n")[0];
        let diagnostic = found.diagnostic(Some(Utf8Path::new("config/aisle.conf")));
        assert_eq!(
            diagnostic.message,
            "line 2 is read as an ingredient: # b (comments in aisle.conf start with //)"
        );
        let location = diagnostic.location.unwrap();
        assert_eq!(
            location.file.as_deref(),
            Some(Utf8Path::new("config/aisle.conf"))
        );
        assert_eq!(location.span, Some(Span::from(4..7)));
    }
}
