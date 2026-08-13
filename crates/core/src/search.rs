//! Full-text recipe search.
//!
//! [`search`] walks the `.cook` and `.menu` files under a root, scores each
//! against a query, and returns the matches best first.
//!
//! # What counts as a match
//!
//! Scoring is `cooklang-find`'s, and it is worth knowing before building a UI
//! on top of it, because it is **not** the AND over the query's terms that
//! CookCLI's own help text has long claimed:
//!
//! - The file name is matched against the **whole query**, spaces included.
//!   An exact stem match scores highest, a substring match next.
//! - The contents are matched against **each term separately**, and every
//!   occurrence adds a little. A file containing any one term scores above
//!   zero, so a multi-term query is closer to a union than an intersection —
//!   adding a term can return *more* recipes, not fewer.
//!
//! Anything scoring above zero is a hit, so a query of common words matches
//! most of a collection.

use crate::{find, Context, CoreError, Outcome};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

/// A search to run.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// The query, as a single string.
    ///
    /// Whitespace splits it into terms for the content match, while the whole
    /// string is what the file name match looks for — so the spacing is part of
    /// the query rather than a separator this crate is free to normalise.
    ///
    /// Callers holding a list of words, such as a command line's arguments,
    /// join them with a space. That loses nothing: `["olive", "oil"]` and
    /// `["olive oil"]` are the same query, and no scoring rule below could tell
    /// them apart if this field kept them separate.
    pub query: String,
    /// Directory to search. Defaults to the context base path.
    ///
    /// Every hit's [`SearchHit::relative_path`] is expressed against this, so
    /// it doubles as the root that results are reported relative to.
    pub base_dir: Option<Utf8PathBuf>,
}

/// One matching recipe.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct SearchHit {
    /// Where the recipe sits under the search root.
    ///
    /// This is [`SearchHit::path`] with the root stripped off, and is what
    /// `cook search` prints. A path that does not begin with the root is kept
    /// whole rather than mangled; see [`path`](SearchHit::path) for when that
    /// happens.
    pub relative_path: Utf8PathBuf,
    /// The path the search found the recipe at, ready to open.
    ///
    /// Absolute when the search root is absolute, which is the case worth
    /// designing for. When the root is relative the path is too, and is
    /// interpreted against the *process* working directory — for an in-process
    /// editor integration that is the editor's, not the project's. This mirrors
    /// the caveat on [`Context::base_path`] and is the reason a relative root
    /// can leave `relative_path` and `path` equal.
    pub path: Utf8PathBuf,
    /// The recipe's title, falling back to its file stem when it has none.
    ///
    /// Read from front matter that the search has already parsed, so it costs
    /// nothing to carry. `None` only if the entry has neither, which a file
    /// found on disk cannot manage.
    pub name: Option<String>,
}

/// Search the recipes under `req`'s root, best match first.
///
/// The root is [`SearchRequest::base_dir`], or [`Context::base_path`] when that
/// is unset. Nothing else on the context is consulted. A root that does not
/// exist is not an error: there is simply nothing under it to match.
///
/// See the [module documentation](self) for what scoring counts as a match —
/// it is not an AND over the query's terms.
///
/// # Errors
///
/// - [`CoreError::Search`] if the root cannot be searched at all, which in
///   practice means its name contains glob syntax.
/// - [`CoreError::Io`] if a file under the root turned up in the walk but could
///   not be read, or its front matter could not be understood. One such file
///   fails the whole search rather than being skipped — that is
///   `cooklang-find`'s behaviour and this crate does not paper over it.
pub fn search(ctx: &Context, req: SearchRequest) -> Result<Outcome<Vec<SearchHit>>, CoreError> {
    let base_dir = req
        .base_dir
        .unwrap_or_else(|| ctx.base_path().to_path_buf());

    tracing::trace!("searching {base_dir} for {:?}", req.query);

    let entries =
        cooklang_find::search(&base_dir, &req.query).map_err(|e| search_error(e, &base_dir))?;

    let hits = entries
        .iter()
        .filter_map(|entry| {
            // A search only ever yields file-backed entries, so this skips
            // nothing today. It is a `filter_map` rather than an unwrap because
            // a `RecipeEntry` need not have a path, and inventing one for an
            // entry that lacks it would be worse than leaving it out.
            let path = entry.path()?;
            Some(SearchHit {
                relative_path: relative_to(&base_dir, path),
                path: path.clone(),
                name: entry.name().clone(),
            })
        })
        .collect();

    Ok(Outcome::new(hits))
}

/// Express `path` relative to `base_dir`, leaving it whole when it does not
/// start with it.
///
/// The fallback is load-bearing rather than defensive, because the root is
/// matched as written rather than resolved. A root of `./recipes` has the walk
/// produce `recipes/soup.cook`, which does not begin with `./recipes`, so the
/// hit keeps the root in it. Pass a root without a `./` — or an absolute one —
/// to get the stripping the name promises.
fn relative_to(base_dir: &Utf8Path, path: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(base_dir).unwrap_or(path).to_owned()
}

/// Map a search failure onto the error that describes what actually happened.
///
/// Only a bad glob pattern means the root itself was unsearchable; the rest
/// mean a file under it was found and could not be read or understood.
fn search_error(error: cooklang_find::search::SearchError, base_dir: &Utf8Path) -> CoreError {
    use cooklang_find::search::SearchError;
    match error {
        SearchError::PatternError(source) => CoreError::Search {
            base_dir: base_dir.to_owned(),
            message: source.to_string(),
        },
        // Carries the file it failed on, which is more use than the root.
        SearchError::GlobError(source) => CoreError::Io {
            path: Utf8Path::from_path(source.path())
                .map(Utf8Path::to_owned)
                .unwrap_or_else(|| base_dir.to_owned()),
            source: source.into_error(),
        },
        // These two lose the file on the way out of `cooklang-find`, so the
        // root is the most specific thing left to name.
        SearchError::IoError(source) => CoreError::Io {
            path: base_dir.to_owned(),
            source,
        },
        SearchError::RecipeEntryError(source) => CoreError::Io {
            path: base_dir.to_owned(),
            source: find::entry_error(source),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture whose recipes overlap deliberately: `chicken` and `rice`
    /// appear in one recipe each, so a two-term query tells AND and OR apart.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        std::fs::create_dir(base.join("Breakfast")).unwrap();
        write(
            &base.join("Breakfast").join("pancakes.cook"),
            "---\ntitle: Fluffy Pancakes\n---\n\nMix @flour{2%cups} with @milk{1%cup}.\n",
        );
        write(
            &base.join("curry.cook"),
            "Fry @chicken{1} in @oil{1%tbsp}.\n",
        );
        write(
            &base.join("pilaf.cook"),
            "Boil @rice{200%g} in @water{1%l}.\n",
        );
        dir
    }

    fn write(path: &Utf8Path, text: &str) {
        std::fs::write(path, text).unwrap();
    }

    fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    /// Run a search rooted at the context base path.
    fn run(base_dir: &Utf8Path, query: &str) -> Vec<SearchHit> {
        search(
            &Context::new(base_dir.to_owned()),
            SearchRequest {
                query: query.to_string(),
                base_dir: None,
            },
        )
        .expect("search succeeds")
        .into_value()
    }

    /// The hits' paths, left as paths rather than stringified.
    ///
    /// That is what makes the assertions below portable: camino compares a
    /// path component by component, so the `Breakfast\pancakes.cook` the walk
    /// produces on Windows equals the `Breakfast/pancakes.cook` written here,
    /// while comparing the two as strings would not. Nothing else is
    /// loosened — a hit under a different directory, or with a different file
    /// name, still fails.
    fn relative_paths(hits: &[SearchHit]) -> Vec<Utf8PathBuf> {
        hits.iter().map(|h| h.relative_path.clone()).collect()
    }

    #[test]
    fn finds_a_recipe_whose_content_matches_a_term() {
        let dir = fixture();
        let hits = run(&base(&dir), "flour");
        assert_eq!(relative_paths(&hits), ["Breakfast/pancakes.cook"]);
    }

    #[test]
    fn finds_a_recipe_by_its_file_name() {
        let dir = fixture();
        // "pilaf" appears nowhere in any recipe's text.
        let hits = run(&base(&dir), "pilaf");
        assert_eq!(relative_paths(&hits), ["pilaf.cook"]);
    }

    #[test]
    fn a_query_that_matches_nothing_returns_no_hits() {
        let dir = fixture();
        assert!(run(&base(&dir), "kohlrabi").is_empty());
    }

    /// Pins the semantics that CookCLI's help text has long described as AND.
    ///
    /// It is not AND. `cooklang-find` scores a file for *any* term it contains
    /// and keeps everything that scores above zero, so a two-term query is a
    /// union: `curry.cook` has no rice in it and `pilaf.cook` has no chicken,
    /// and "chicken rice" returns both. Changing that is `cooklang-find`'s
    /// call, not this crate's; this test is here so the change is noticed.
    #[test]
    fn multiple_terms_are_ored_not_anded() {
        let dir = fixture();
        let mut hits = relative_paths(&run(&base(&dir), "chicken rice"));
        hits.sort();
        assert_eq!(hits, ["curry.cook", "pilaf.cook"]);
    }

    /// The corollary of the above: an AND-style query does not narrow.
    #[test]
    fn adding_a_term_can_widen_the_result_set() {
        let dir = fixture();
        let one = run(&base(&dir), "chicken");
        let two = run(&base(&dir), "chicken rice");
        assert_eq!(relative_paths(&one), ["curry.cook"]);
        assert!(
            two.len() > one.len(),
            "a second term must not be an AND filter: {:?}",
            relative_paths(&two)
        );
    }

    #[test]
    fn hits_are_relative_to_the_search_root() {
        let dir = fixture();
        let hits = run(&base(&dir), "flour");
        let hit = hits.first().expect("one hit");
        assert_eq!(hit.relative_path, "Breakfast/pancakes.cook");
        assert!(
            hit.relative_path.is_relative(),
            "relative_path must not be absolute: {}",
            hit.relative_path
        );
    }

    #[test]
    fn hits_carry_the_path_the_search_found_them_at() {
        let dir = fixture();
        let base = base(&dir);
        let hits = run(&base, "flour");
        let hit = hits.first().expect("one hit");
        assert_eq!(hit.path, base.join("Breakfast").join("pancakes.cook"));
        assert!(hit.path.is_file(), "path must be openable: {}", hit.path);
    }

    #[test]
    fn a_hit_is_named_by_its_title_when_it_has_one() {
        let dir = fixture();
        let hits = run(&base(&dir), "flour");
        assert_eq!(
            hits.first().expect("one hit").name.as_deref(),
            Some("Fluffy Pancakes")
        );
    }

    #[test]
    fn a_hit_with_no_title_is_named_by_its_file_stem() {
        let dir = fixture();
        let hits = run(&base(&dir), "pilaf");
        assert_eq!(
            hits.first().expect("one hit").name.as_deref(),
            Some("pilaf")
        );
    }

    #[test]
    fn base_dir_overrides_the_context_base_path() {
        let searched = fixture();
        let ignored = tempfile::TempDir::new().unwrap();
        write(&base(&ignored).join("decoy.cook"), "Mix @flour{1%cup}.\n");

        let hits = search(
            &Context::new(base(&ignored)),
            SearchRequest {
                query: "flour".to_string(),
                base_dir: Some(base(&searched)),
            },
        )
        .expect("search succeeds")
        .into_value();

        assert_eq!(relative_paths(&hits), ["Breakfast/pancakes.cook"]);
    }

    #[test]
    fn without_a_base_dir_the_context_base_path_is_searched() {
        let searched = fixture();
        let hits = run(&base(&searched), "flour");
        assert_eq!(relative_paths(&hits), ["Breakfast/pancakes.cook"]);
    }

    /// A filename match outranks a content-only match, so the caller can print
    /// the list as it comes and have the likeliest recipe first.
    #[test]
    fn hits_come_back_best_first() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = base(&dir);
        write(&base.join("aaa-mentions-pilaf.cook"), "Serve with pilaf.\n");
        write(&base.join("pilaf.cook"), "Boil @rice{200%g}.\n");

        // Alphabetically the mention sorts first, so ordering by score is the
        // only thing that can put `pilaf.cook` in front.
        assert_eq!(
            relative_paths(&run(&base, "pilaf")),
            ["pilaf.cook", "aaa-mentions-pilaf.cook"]
        );
    }

    /// Searching somewhere that does not exist is empty, not an error.
    #[test]
    fn a_missing_search_root_yields_no_hits() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(run(&base(&dir).join("nope"), "flour").is_empty());
    }

    /// `relative_to` is exercised directly for the case that cannot be reached
    /// from a test without changing the process working directory: a search
    /// root spelled relatively.
    ///
    /// The walk resolves `./recipes/**/*.cook` to paths like
    /// `recipes/soup.cook`, dropping the `./` that `strip_prefix` would then
    /// need to find. The root therefore survives into the result. This is
    /// long-standing `cook search --base-dir ./recipes` behaviour, pinned here
    /// rather than endorsed.
    #[test]
    fn a_path_that_does_not_start_with_the_root_is_left_alone() {
        assert_eq!(
            relative_to(
                Utf8Path::new("./recipes"),
                Utf8Path::new("recipes/soup.cook")
            ),
            "recipes/soup.cook"
        );
        assert_eq!(
            relative_to(
                Utf8Path::new("/recipes"),
                Utf8Path::new("/elsewhere/soup.cook")
            ),
            "/elsewhere/soup.cook"
        );
    }

    #[test]
    fn a_path_under_the_root_is_stripped_to_the_remainder() {
        assert_eq!(
            relative_to(
                Utf8Path::new("/recipes"),
                Utf8Path::new("/recipes/Breakfast/pancakes.cook")
            ),
            "Breakfast/pancakes.cook"
        );
    }

    /// A search root whose name contains glob syntax cannot be turned into a
    /// pattern. It is a real directory that a user can really have, so it gets
    /// an error naming it rather than a silent empty result.
    #[test]
    fn a_search_root_that_is_not_a_valid_glob_pattern_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = base(&dir).join("re[ci");
        std::fs::create_dir(&root).unwrap();
        write(&root.join("soup.cook"), "Boil @water{1%l}.\n");

        match search(
            &Context::new(root.clone()),
            SearchRequest {
                query: "water".to_string(),
                base_dir: None,
            },
        ) {
            Err(CoreError::Search { base_dir, message }) => {
                assert_eq!(base_dir, root);
                assert!(
                    message.contains("attern"),
                    "the cause must survive: {message}"
                );
            }
            other => panic!(
                "expected CoreError::Search, got {:?}",
                other.map(|o| o.value)
            ),
        }
    }

    /// The remaining failures mean a file under the root was unusable, not that
    /// the root was. They are pinned through `search_error` directly, because
    /// provoking them needs a file that breaks between the walk finding it and
    /// the walk reading it.
    ///
    /// `SearchError::GlobError` is absent only because `glob` exposes no way to
    /// construct one; its arm is the one that names the failing file rather
    /// than the root.
    #[test]
    fn a_file_that_cannot_be_read_is_an_io_error_not_a_search_error() {
        use cooklang_find::search::SearchError;
        let root = Utf8Path::new("/recipes");

        let unreadable = search_error(
            SearchError::IoError(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            )),
            root,
        );
        match unreadable {
            CoreError::Io { path, source } => {
                assert_eq!(path, root);
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
            }
            other => panic!("expected CoreError::Io, got {other:?}"),
        }

        let unusable = search_error(
            SearchError::RecipeEntryError(cooklang_find::RecipeEntryError::MetadataError(
                "bad front matter".to_string(),
            )),
            root,
        );
        match unusable {
            CoreError::Io { path, source } => {
                assert_eq!(path, root);
                assert!(
                    source.to_string().contains("bad front matter"),
                    "the cause must survive: {source}"
                );
            }
            other => panic!("expected CoreError::Io, got {other:?}"),
        }
    }
}
