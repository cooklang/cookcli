//! Renders the API reference in `web::api_docs` as Markdown for `docs/api.md`.
//!
//! `docs/api.md` is a generated file, not a hand-written one — it would
//! otherwise become a third copy of the endpoint list, next to the router and
//! `api_docs.rs`, with nothing keeping it honest. `tests/api_docs_md_test.rs`
//! fails when the checked-in file no longer matches this renderer's output,
//! and regenerates it on request:
//!
//! ```text
//! UPDATE_API_DOCS=1 cargo test --test api_docs_md_test
//! ```
//!
//! The output feeds two readers: GitHub, and cooklang.org via that repo's
//! `scripts/sync-cli-docs.sh`, which strips the H1 and prepends Hugo
//! frontmatter. Both render GitHub-flavoured Markdown, so tables are fine but
//! raw HTML is avoided.

use crate::web::api_docs;
use crate::web::templates::{ApiSection, EndpointDoc};

/// The base URL written into the generated doc. The live page uses the request
/// host instead; a static file has to pick the default and say so.
const DEFAULT_BASE_URL: &str = "http://localhost:9080/api";

/// Render the whole reference, including the trailing newline.
pub fn render() -> String {
    let mut out = String::with_capacity(64 * 1024);
    let preamble = api_docs::preamble();
    let sections = api_docs::sections();

    out.push_str("# Server API\n\n");
    out.push_str(&preamble.intro);
    out.push_str("\n\n");
    out.push_str(
        "Start the server with [`cook server`](server.md); every endpoint below is served by \
         it. The same reference is available from a running server at `/api-docs`, where the \
         base URL reflects the host you reached it on.\n\n",
    );

    out.push_str("## Before you start\n\n");
    out.push_str(&format!("- **Base URL:** `{DEFAULT_BASE_URL}`\n"));
    for n in &preamble.notes {
        out.push_str(&format!("- **{}:** {}\n", n.label, n.text));
    }
    out.push('\n');

    out.push_str("## Errors\n\n");
    out.push_str(&preamble.error_intro);
    out.push_str("\n\n```json\n");
    out.push_str(&preamble.error_example);
    out.push_str("\n```\n\n");
    for c in &preamble.error_codes {
        out.push_str(&format!("- `{}` — {}\n", c.label, c.text));
    }
    out.push('\n');

    out.push_str("## Contents\n\n");
    for s in &sections {
        out.push_str(&format!("- [{}](#{})\n", s.title, anchor(&s.title)));
    }
    out.push('\n');

    for section in &sections {
        render_section(&mut out, section);
    }

    out.push_str(
        "---\n\nThis file is generated from `src/web/api_docs.rs`. Edit that, then run \
         `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test` to regenerate.\n",
    );

    out
}

fn render_section(out: &mut String, section: &ApiSection) {
    out.push_str(&format!("## {}\n\n", section.title));
    out.push_str(&section.description);
    out.push_str("\n\n");

    for endpoint in &section.endpoints {
        render_endpoint(out, endpoint);
    }
}

fn render_endpoint(out: &mut String, ep: &EndpointDoc) {
    // Backticked so route syntax survives Markdown: the `*` of `*path` would
    // otherwise be an unpaired emphasis marker, and `_` in a path an italic.
    out.push_str(&format!("### `{} {}`\n\n", ep.method, ep.path));

    out.push_str(&ep.summary);
    if let Some(feature) = &ep.feature {
        out.push_str(&format!(
            " *(requires a build with the `{feature}` feature)*"
        ));
    }
    out.push_str("\n\n");

    if !ep.description.is_empty() {
        out.push_str(&ep.description);
        out.push_str("\n\n");
    }

    if !ep.params.is_empty() {
        out.push_str("| Name | In | Type | Required | Description |\n");
        out.push_str("|------|----|------|----------|-------------|\n");
        for p in &ep.params {
            out.push_str(&format!(
                "| `{}` | {} | `{}` | {} | {} |\n",
                p.name,
                p.kind,
                p.type_name,
                if p.required { "yes" } else { "no" },
                escape_cell(&p.description),
            ));
        }
        out.push('\n');
    }

    if let Some(request) = &ep.request_example {
        out.push_str("Request body:\n\n");
        push_fence(out, request);
    }

    if let Some(response) = &ep.response_example {
        out.push_str("Response:\n\n");
        push_fence(out, response);
    }
}

/// Wrap an example in a fenced block, tagged `json` only when it actually is
/// JSON — the SSE stream on `/api/shopping_list/events` is not.
fn push_fence(out: &mut String, example: &str) {
    let lang = match example.trim_start().chars().next() {
        Some('{') | Some('[') => "json",
        _ => "text",
    };
    out.push_str(&format!("```{lang}\n{}\n```\n\n", example.trim_end()));
}

/// A cell's `|` would end the column early, and GFM has no way to escape one
/// inside a code span — so escape it as text and accept the stray backslash in
/// the rare cell that needs it.
fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// GitHub's heading anchor: lowercase, non-alphanumerics dropped, spaces to
/// hyphens. Hugo's Goldmark uses the same scheme, so one form serves both.
fn anchor(title: &str) -> String {
    title
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() => Some(c.to_ascii_lowercase()),
            '-' | '_' => Some(c),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_matches_github_slugging() {
        assert_eq!(anchor("Search & Stats"), "search--stats");
        assert_eq!(anchor("Shopping List"), "shopping-list");
        assert_eq!(anchor("Recipes"), "recipes");
    }

    #[test]
    fn fences_are_tagged_by_content() {
        let mut out = String::new();
        push_fence(&mut out, "{ \"a\": 1 }");
        assert!(out.starts_with("```json\n"));

        let mut out = String::new();
        push_fence(&mut out, "event: change\ndata: {}");
        assert!(out.starts_with("```text\n"));
    }

    #[test]
    fn table_cells_escape_pipes() {
        assert_eq!(escape_cell("a | b"), "a \\| b");
    }

    /// Every fence the renderer opens is closed, so no example can swallow the
    /// rest of the document.
    #[test]
    fn fences_are_balanced() {
        let md = render();
        assert_eq!(md.matches("\n```").count() % 2, 0);
    }

    /// No example may contain a fence of its own — that would terminate the
    /// block early and leave valid-looking but scrambled output.
    #[test]
    fn no_example_contains_a_fence() {
        for section in api_docs::sections() {
            for ep in section.endpoints {
                for example in [ep.request_example, ep.response_example]
                    .into_iter()
                    .flatten()
                {
                    assert!(
                        !example.contains("```"),
                        "example for {} {} contains a code fence",
                        ep.method,
                        ep.path
                    );
                }
            }
        }
    }

    /// Backticks have to pair up. The page's `inline_code` filter alternates
    /// on every backtick and Markdown closes a span at the first match, so an
    /// odd count silently mis-renders in both — as an escaped `\`` inside a
    /// code span once did on `GET /api/search`.
    #[test]
    fn backticks_pair_up_in_prose() {
        let mut prose: Vec<(String, String)> = Vec::new();
        let preamble = api_docs::preamble();
        for n in preamble.notes.iter().chain(preamble.error_codes.iter()) {
            prose.push((format!("preamble note {}", n.label), n.text.clone()));
        }
        for section in api_docs::sections() {
            prose.push((format!("section {}", section.title), section.description));
            for ep in section.endpoints {
                let at = format!("{} {}", ep.method, ep.path);
                prose.push((format!("{at} summary"), ep.summary));
                prose.push((format!("{at} description"), ep.description));
                for p in ep.params {
                    prose.push((format!("{at} param {}", p.name), p.description));
                }
            }
        }

        for (at, text) in prose {
            assert_eq!(
                text.matches('`').count() % 2,
                0,
                "unbalanced backtick in {at}: {text}"
            );
            assert!(
                !text.contains("\\`"),
                "escaped backtick in {at} — neither renderer honours it: {text}"
            );
        }
    }

    /// Every section in the contents list has a heading to land on.
    #[test]
    fn contents_links_resolve() {
        let md = render();
        for section in api_docs::sections() {
            assert!(
                md.contains(&format!("## {}\n", section.title)),
                "no heading for section {}",
                section.title
            );
            assert!(
                md.contains(&format!("](#{})", anchor(&section.title))),
                "no contents link for section {}",
                section.title
            );
        }
    }
}
