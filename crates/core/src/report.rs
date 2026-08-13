//! Rendering a recipe through a Jinja2 template.
//!
//! [`render`] is a thin, non-fatal layer over the `cooklang-reports` crate: it
//! resolves the recipe and the aisle/pantry configuration the way the rest of
//! this crate does, hands the result to `cooklang-reports`, and turns any
//! failure into a [`CoreError`] instead of ending the process.
//!
//! # What a template can see
//!
//! The variables come from `cooklang-reports`, not from here. As of 0.5.1 they
//! are `scale`, `sections`, `ingredients`, `cookware`, `metadata`, `datastore`,
//! `base_path`, `aisle_content` and `pantry_content`, plus the functions and
//! filters that crate registers — `db`, `get_ingredient_list`, `aisled`,
//! `excluding_pantry`, `from_pantry`, the `number_*` family and the string
//! filters. There is **no** `recipe` variable: `{{ recipe.title }}` is
//! `{{ metadata.title }}`, and `recipe.ingredients` is `ingredients`.
//!
//! # Two things `cooklang-reports` does that this crate cannot stop
//!
//! - **It parses the recipe itself**, with `CooklangParser::canonical` (every
//!   extension on, no unit converter) rather than the configuration [`PARSER`]
//!   uses. So a recipe may render here and fail [`parse_recipe`], or the
//!   reverse, and the quantities a template sees are not necessarily the ones
//!   [`recipe::read`] would produce.
//! - **It writes warnings straight to stderr.** Recipe warnings, aisle and
//!   pantry warnings, and a datastore key it could not find are all
//!   `eprintln!`ed from inside the render call. Nothing reaches this crate, so
//!   [`Outcome::diagnostics`] on the way back is always empty — do not read it
//!   as "the recipe was clean".
//!
//! [`PARSER`]: crate::PARSER
//! [`parse_recipe`]: crate::parse_recipe
//! [`recipe::read`]: crate::recipe::read
//! [`Outcome::diagnostics`]: crate::Outcome::diagnostics

use crate::{
    find::{entry_error, get_recipe},
    ConfigSource, Context, CoreError, Outcome, RecipeSource,
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_reports::{config::Config, render_template_with_config};

/// A report to render.
#[derive(Debug, Clone)]
pub struct RenderRequest {
    /// The recipe to render.
    pub source: RecipeSource,
    /// The template text itself, not a path to it.
    ///
    /// Reading a `.jinja` off disk is the caller's job, which is what lets a
    /// caller holding one in a buffer pass it straight in. Nothing is lost by
    /// taking the text: `cooklang-reports` registers this as the environment's
    /// only template and configures no loader, so `{% include %}` and
    /// `{% extends %}` have nothing to resolve against either way.
    pub template: String,
    /// Scaling factor. Pass `1.0` to leave quantities alone.
    ///
    /// Reaches the template as the `scale` variable *and* scales the recipe, so
    /// a template printing `{{ scale }}` alongside its quantities stays
    /// consistent. As in [`ReadRequest::scale`](crate::recipe::ReadRequest),
    /// CookCLI's `name:factor` spelling is a command-line convention: callers
    /// split it themselves with
    /// [`split_name_and_scale`](crate::recipe::split_name_and_scale).
    pub scale: f64,
    /// Directory of the YAML datastore the `db()` template function reads.
    ///
    /// `None` leaves `db()` unusable; a template that calls it then fails with
    /// [`CoreError::Render`]. A path that does not exist is *not* an error
    /// here — `cooklang-reports` reports a key it cannot find by warning on
    /// stderr and substituting an empty string.
    pub datastore: Option<Utf8PathBuf>,
    /// Directory that recipe references inside the template resolve against,
    /// and that a [`RecipeSource::Path`] is looked up under.
    ///
    /// Defaults to [`Context::base_path`]. It also reaches the template as the
    /// `base_path` variable. Unlike the CLI, nothing here makes it absolute: a
    /// relative path is interpreted against the *process* working directory.
    pub base_path: Option<Utf8PathBuf>,
}

/// Render `req`'s recipe through `req`'s template.
///
/// Aisle and pantry configuration come from `ctx`, and both
/// [`ConfigSource`] kinds work: a [`ConfigSource::Path`] is handed to
/// `cooklang-reports` to read, and a [`ConfigSource::Inline`] is injected
/// directly as the `aisle_content` / `pantry_content` template variables that
/// `aisled()`, `excluding_pantry()` and `from_pantry()` look up. The two are
/// equivalent to a template, with one difference worth knowing: a
/// [`ConfigSource::Path`] naming a file that cannot be read is *not* an error —
/// `cooklang-reports` warns on stderr and carries on as though no aisle or
/// pantry had been given.
///
/// # Errors
///
/// - [`CoreError::InvalidScale`] if the scale is not finite. Checked before
///   anything is read.
/// - [`CoreError::RecipeNotFound`] if a [`RecipeSource::Path`] matches nothing.
/// - [`CoreError::Io`] if such a path matches a file that cannot be read.
/// - [`CoreError::Render`] if the template is broken, if rendering it fails, or
///   if `cooklang-reports` could not parse the recipe. Its `rendered` field
///   carries that crate's own formatted report, which the CLI prints verbatim.
pub fn render(ctx: &Context, req: RenderRequest) -> Result<Outcome<String>, CoreError> {
    if !req.scale.is_finite() {
        return Err(CoreError::InvalidScale { scale: req.scale });
    }

    let base_path = req
        .base_path
        .unwrap_or_else(|| ctx.base_path().to_path_buf());

    let recipe = recipe_text(&base_path, req.source)?;

    let mut builder = Config::builder();
    builder.scale(req.scale);
    builder.base_path(base_path.as_std_path());
    if let Some(datastore) = &req.datastore {
        builder.datastore_path(datastore.as_std_path());
    }
    if let Some(aisle) = ctx.aisle().path() {
        builder.aisle_path(aisle.as_std_path());
    }
    if let Some(pantry) = ctx.pantry().path() {
        builder.pantry_path(pantry.as_std_path());
    }

    let mut config = builder.build();
    // Inline configuration cannot go through the builder, which only takes
    // paths. `with_context` overlays the template context and wins on conflict,
    // and these are the very names `cooklang-reports` fills in from a path and
    // that its aisle and pantry functions look up — so an inline source is not
    // a second-class one, and nothing is written to a temporary file to fake it.
    if let ConfigSource::Inline(text) = ctx.aisle() {
        config = config.with_context("aisle_content", text.clone());
    }
    if let ConfigSource::Inline(text) = ctx.pantry() {
        config = config.with_context("pantry_content", text.clone());
    }

    tracing::trace!(
        "rendering a report against {base_path} at scale {}",
        req.scale
    );

    let report =
        render_template_with_config(&recipe, &req.template, &config).map_err(render_error)?;

    // Always empty: see the module docs on where `cooklang-reports` puts its
    // warnings. `Outcome` is returned anyway so that this command reads like
    // every other one, and so that diagnostics can start arriving without a
    // breaking change.
    Ok(Outcome::new(report))
}

/// The recipe text to render, read from disk only when asked for a path.
fn recipe_text(base_path: &Utf8Path, source: RecipeSource) -> Result<String, CoreError> {
    match source {
        RecipeSource::Content { text, .. } => Ok(text),
        RecipeSource::Path(lookup) => {
            let entry = get_recipe(base_path, lookup.as_str())?;
            let path = entry.path().cloned().unwrap_or(lookup);
            entry.content().map_err(|source| CoreError::Io {
                path,
                source: entry_error(source),
            })
        }
    }
}

/// Turn a `cooklang-reports` failure into a [`CoreError::Render`].
///
/// `format_with_source` is multi-line — source location, error chain and hints
/// — so it goes in `rendered` and a one-line summary goes in `message`, keeping
/// `Display` to a single line like every other variant.
fn render_error(error: cooklang_reports::Error) -> CoreError {
    let rendered = error.format_with_source();
    let message = match &error {
        // minijinja's own `Display`, e.g. "syntax error: unexpected end of
        // input (in base:1)". `first_line` guards the day it stops being one.
        cooklang_reports::Error::TemplateError(e) => first_line(&e.to_string()),
        // The `SourceReport` this carries renders as a multi-line report, which
        // is already in `rendered`; summarising it here would only repeat the
        // first diagnostic out of context.
        cooklang_reports::Error::RecipeParseError(_) => {
            "the recipe could not be parsed".to_string()
        }
    };
    CoreError::Render { message, rendered }
}

/// The first line of `text`, with trailing whitespace removed.
fn first_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfigSource, Context, RecipeSource};

    const PANCAKES: &str = "---\ntitle: Pancakes\n---\n\n\
        Mix @eggs{3%large} with @milk{250%ml} and @flour{125%g}.\n";

    fn ctx() -> Context {
        Context::new(Utf8PathBuf::from("."))
    }

    fn request(template: &str) -> RenderRequest {
        RenderRequest {
            source: RecipeSource::Content {
                text: PANCAKES.to_string(),
                name: "pancakes".to_string(),
            },
            template: template.to_string(),
            scale: 1.0,
            datastore: None,
            base_path: None,
        }
    }

    fn utf8(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    fn write(path: &Utf8Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn renders_a_template_against_the_recipe() {
        let outcome = render(
            &ctx(),
            request("{% for i in ingredients %}{{ i.name }};{% endfor %}"),
        )
        .expect("renders");
        assert_eq!(outcome.value, "eggs;milk;flour;");
        assert!(outcome.diagnostics.is_empty());
    }

    #[test]
    fn metadata_is_available_to_the_template() {
        let outcome = render(&ctx(), request("{{ metadata.title }}")).expect("renders");
        assert_eq!(outcome.value, "Pancakes");
    }

    /// Both halves of the scale contract: the number reaches the template, and
    /// the quantities it prints have actually been scaled by it. Asserting only
    /// the former would pass with the scaling dropped.
    #[test]
    fn scale_reaches_the_template_and_the_quantities() {
        let template = "{{ scale }}|{{ ingredients[1].name }}={{ ingredients[1].quantity }}";

        let one = render(&ctx(), request(template)).expect("renders");
        assert_eq!(one.value, "1.0|milk=250 ml");

        let mut req = request(template);
        req.scale = 3.0;
        let three = render(&ctx(), req).expect("renders");
        assert_eq!(three.value, "3.0|milk=750 ml");
    }

    #[test]
    fn a_non_finite_scale_is_rejected_before_anything_is_read() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut req = request("{{ scale }}");
            req.scale = bad;
            // A source that would fail loudly if it were ever looked at, so
            // this cannot pass by rendering something.
            req.source = RecipeSource::Path(Utf8PathBuf::from("no/such/recipe.cook"));
            match render(&ctx(), req) {
                Err(CoreError::InvalidScale { scale }) => {
                    assert_eq!(scale.is_nan(), bad.is_nan());
                }
                other => panic!("expected InvalidScale for {bad}, got {other:?}"),
            }
        }
    }

    /// The whole point of the extraction: a broken template must come back as a
    /// value, not end the process. The CLI is what exits.
    #[test]
    fn a_broken_template_is_a_render_error() {
        // Missing `%}` on the endfor.
        let req = request("{% for i in ingredients %}{{ i.name }}{% endfor");
        match render(&ctx(), req) {
            Err(CoreError::Render { message, rendered }) => {
                assert!(
                    !message.contains('\n'),
                    "the summary must stay one line: {message:?}"
                );
                assert!(
                    message.to_lowercase().contains("syntax"),
                    "expected the engine's own summary, got {message:?}"
                );
                // What the CLI prints instead of core's one-liner: it must say
                // more than the summary does.
                assert!(
                    rendered.len() > message.len(),
                    "rendered should carry the long report, got {rendered:?}"
                );
                assert!(
                    rendered.contains("endfor"),
                    "rendered should quote the template, got {rendered:?}"
                );
            }
            other => panic!("expected CoreError::Render, got {other:?}"),
        }
    }

    /// A template that parses but fails while rendering — a different minijinja
    /// error kind, down the same channel.
    #[test]
    fn a_failing_expression_is_a_render_error_too() {
        // `db()` with no datastore configured.
        match render(&ctx(), request("{{ db('eggs.price') }}")) {
            Err(CoreError::Render { message, .. }) => {
                assert!(!message.is_empty(), "expected a summary");
                assert!(!message.contains('\n'), "one line: {message:?}");
            }
            other => panic!("expected CoreError::Render, got {other:?}"),
        }
    }

    /// `cooklang-reports` parses the recipe itself, so this arrives as a render
    /// failure rather than `CoreError::Parse`. Pinned because it is surprising.
    #[test]
    fn an_unparseable_recipe_is_also_a_render_error() {
        let mut req = request("{{ metadata.title }}");
        req.source = RecipeSource::Content {
            // An ingredient with a quantity and no name.
            text: "Add @{1%tsp} to the pot.\n".to_string(),
            name: "broken".to_string(),
        };
        match render(&ctx(), req) {
            Err(CoreError::Render { message, rendered }) => {
                assert_eq!(message, "the recipe could not be parsed");
                assert!(
                    !rendered.is_empty(),
                    "the parse report must survive into `rendered`"
                );
            }
            other => panic!("expected CoreError::Render, got {other:?}"),
        }
    }

    #[test]
    fn a_path_source_is_resolved_by_name_under_the_base_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        write(&base.join("pancakes.cook"), PANCAKES);

        // A bare name with no extension: only a `cooklang-find` lookup resolves
        // this, which is the difference from opening the path as given.
        let mut req = request("{{ metadata.title }}");
        req.source = RecipeSource::Path(Utf8PathBuf::from("pancakes"));
        let outcome = render(&Context::new(base), req).expect("renders");
        assert_eq!(outcome.value, "Pancakes");
    }

    #[test]
    fn a_missing_path_source_is_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut req = request("{{ metadata.title }}");
        req.source = RecipeSource::Path(Utf8PathBuf::from("absent.cook"));
        match render(&Context::new(utf8(&dir)), req) {
            Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "absent.cook"),
            other => panic!("expected RecipeNotFound, got {other:?}"),
        }
    }

    /// `Content` must never touch the filesystem, even when a file of the same
    /// name is sitting there with different text.
    #[test]
    fn content_is_rendered_as_given_and_never_read_from_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        write(
            &base.join("pancakes.cook"),
            "---\ntitle: On Disk\n---\n\nBoil @water{1%l}.\n",
        );

        let outcome =
            render(&Context::new(base), request("{{ metadata.title }}")).expect("renders");
        assert_eq!(
            outcome.value, "Pancakes",
            "the buffer must win over the file of the same name"
        );
    }

    #[test]
    fn the_base_path_defaults_to_the_context_and_is_overridable() {
        let ctx = Context::new(Utf8PathBuf::from("/from/context"));
        let outcome = render(&ctx, request("{{ base_path }}")).expect("renders");
        assert_eq!(outcome.value, "/from/context");

        let mut req = request("{{ base_path }}");
        req.base_path = Some(Utf8PathBuf::from("/from/request"));
        let outcome = render(&ctx, req).expect("renders");
        assert_eq!(outcome.value, "/from/request");
    }

    /// The request's base path must steer the recipe lookup too, not only the
    /// variable the template sees.
    #[test]
    fn the_request_base_path_steers_the_recipe_lookup() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        write(&base.join("elsewhere").join("pancakes.cook"), PANCAKES);

        let mut req = request("{{ metadata.title }}");
        req.source = RecipeSource::Path(Utf8PathBuf::from("pancakes"));
        req.base_path = Some(base.join("elsewhere"));

        // The context points somewhere with no recipes in it at all.
        let ctx = Context::new(base.join("nothing-here"));
        let outcome = render(&ctx, req).expect("renders");
        assert_eq!(outcome.value, "Pancakes");
    }

    const AISLE: &str = "[dairy]\nmilk\neggs\n\n[grains]\nflour\n";
    const PANTRY: &str = "[baking]\nflour = \"2%kg\"\n";

    /// Both the raw variable and the function that consumes it, for each
    /// source kind — a `ConfigSource::Inline` reaching only the variable would
    /// leave `aisled()` quietly returning nothing.
    #[test]
    fn an_aisle_reaches_the_template_from_a_path_and_from_inline_text() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = utf8(&dir).join("aisle.conf");
        write(&path, AISLE);

        let template = "{{ aisle_content | length }}|\
            {% for aisle, items in aisled(ingredients) | items %}{{ aisle }},{% endfor %}";

        for source in [
            ConfigSource::Path(path.clone()),
            ConfigSource::Inline(AISLE.to_string()),
        ] {
            let ctx = Context::new(Utf8PathBuf::from(".")).with_aisle(source.clone());
            let outcome = render(&ctx, request(template)).expect("renders");
            assert_eq!(
                outcome.value,
                format!("{}|dairy,grains,", AISLE.len()),
                "aisle not honoured for {source:?}"
            );
        }
    }

    #[test]
    fn a_pantry_reaches_the_template_from_a_path_and_from_inline_text() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = utf8(&dir).join("pantry.conf");
        write(&path, PANTRY);

        let template = "{{ pantry_content | length }}|\
            {% for i in excluding_pantry(ingredients) %}{{ i.name }},{% endfor %}";

        for source in [
            ConfigSource::Path(path.clone()),
            ConfigSource::Inline(PANTRY.to_string()),
        ] {
            let ctx = Context::new(Utf8PathBuf::from(".")).with_pantry(source.clone());
            let outcome = render(&ctx, request(template)).expect("renders");
            assert_eq!(
                outcome.value,
                format!("{}|eggs,milk,", PANTRY.len()),
                "pantry not honoured for {source:?}"
            );
        }
    }

    /// Without a configuration the functions must still work, returning
    /// everything unfiltered rather than failing.
    #[test]
    fn no_aisle_or_pantry_leaves_the_template_functions_usable() {
        let outcome = render(
            &ctx(),
            request("{% for i in excluding_pantry(ingredients) %}{{ i.name }},{% endfor %}"),
        )
        .expect("renders");
        assert_eq!(outcome.value, "eggs,milk,flour,");
    }

    #[test]
    fn the_datastore_path_is_what_the_db_function_reads() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = utf8(&dir).join("db");
        write(
            &store.join("eggs").join("shopping.yml"),
            "price_per_unit: 0.25\n",
        );

        let mut req = request("{{ db('eggs.shopping.price_per_unit') }}");
        req.datastore = Some(store);
        let outcome = render(&ctx(), req).expect("renders");
        assert_eq!(outcome.value, "0.25");
    }
}
