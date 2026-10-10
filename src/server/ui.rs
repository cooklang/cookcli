use crate::server::handlers::common::{
    is_recipe_request, is_request_path, recipe_file, RecipeFile,
};
use crate::server::{activity, AppState};
use crate::web::language::FeatureFlags;
use crate::web::templates::*;
use crate::web::viewer::Viewer;
use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub(crate) fn error_page(
    lang: LanguageIdentifier,
    prefix: &str,
    msg: impl std::fmt::Display,
    features: FeatureFlags,
    viewer: Viewer,
) -> axum::response::Response {
    let template = ErrorTemplate {
        active: String::new(),
        error_message: msg.to_string(),
        tr: Tr::new(lang),
        prefix: prefix.to_string(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    };
    template.into_response()
}

pub fn ui() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(recipes_page))
        .route("/directory/{*path}", get(recipes_directory))
        .route("/random", get(random_recipe_page))
        .route("/random/{*path}", get(random_recipe_directory))
        .route("/recipe/{*path}", get(recipe_page))
        .route("/edit/{*path}", get(edit_page))
        .route("/new", get(new_page).post(create_recipe))
        .route("/shopping-list", get(shopping_list_page))
        .route("/pantry", get(pantry_page))
        .route("/aisles", get(aisles_page))
        .route("/preferences", get(preferences_page))
        .route("/api-docs", get(api_docs_page))
        .route("/atom.xml", get(atom_feed))
        .route("/rss.xml", get(rss_feed))
        .route("/feed.json", get(json_feed))
        .route(
            "/login",
            get(super::auth::handlers::login_page).post(super::auth::handlers::login),
        )
        .route("/logout", post(super::auth::handlers::logout))
}

async fn recipes_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    recipes_handler(state, None, lang, features, viewer).await
}

async fn recipes_directory(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    recipes_handler(state, Some(path), lang, features, viewer).await
}

async fn recipes_handler(
    state: Arc<AppState>,
    path: Option<String>,
    lang: LanguageIdentifier,
    features: FeatureFlags,
    viewer: Viewer,
) -> axum::response::Response {
    let input = crate::web::builders::RecipesBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        sub_path: path.as_deref(),
        lang: lang.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer: viewer.clone(),
        exclude: None,
    };
    match crate::web::builders::build_recipes_template(input) {
        Ok(template) => template.into_response(),
        Err(e) => {
            tracing::error!("Failed to build recipes template: {:?}", e);
            error_page(lang, &state.url_prefix, &e, features, viewer)
        }
    }
}

async fn random_recipe_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    random_recipe_handler(&state, None, lang, features, viewer)
}

async fn random_recipe_directory(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    random_recipe_handler(&state, Some(&path), lang, features, viewer)
}

/// Redirect to a `.cook` recipe picked at random from a folder and everything
/// below it, so "what should I cook?" can be narrowed to `Mains/` or `Desserts/`.
fn random_recipe_handler(
    state: &AppState,
    path: Option<&str>,
    lang: LanguageIdentifier,
    features: FeatureFlags,
    viewer: Viewer,
) -> axum::response::Response {
    let prefix = &state.url_prefix;
    let dir = match path {
        Some(p) if !crate::util::is_safe_relative_path(p) => {
            let page = error_page(lang, prefix, format!("Invalid path: {p}"), features, viewer);
            return (StatusCode::BAD_REQUEST, page).into_response();
        }
        Some(p) => state.base_path.join(p),
        None => state.base_path.clone(),
    };
    let tree = match cooklang_find::build_tree(&dir) {
        Ok(tree) => tree,
        Err(e) => {
            let page = error_page(
                lang,
                prefix,
                format!("Failed to list recipes: {e}"),
                features,
                viewer,
            );
            return (StatusCode::NOT_FOUND, page).into_response();
        }
    };
    let recipes = crate::web::builders::cook_recipe_paths(&tree);
    if recipes.is_empty() {
        let page = error_page(lang, prefix, "No recipes in this folder", features, viewer);
        return (StatusCode::NOT_FOUND, page).into_response();
    }

    let picked = recipes[fastrand::usize(..recipes.len())];
    // `build_tree` joins what it finds onto `dir`, which starts with the base.
    let Ok(relative) = picked.strip_prefix(&state.base_path) else {
        tracing::error!("Random recipe {picked} is outside {}", state.base_path);
        let page = error_page(lang, prefix, "Failed to pick a recipe", features, viewer);
        return (StatusCode::INTERNAL_SERVER_ERROR, page).into_response();
    };
    let url_path = relative
        .with_extension("")
        .components()
        .map(|c| c.as_str())
        .collect::<Vec<_>>()
        .join("/");
    // Encoded, since `Redirect` refuses a non-ASCII `Location`.
    axum::response::Redirect::to(&format!(
        "{prefix}/recipe/{}",
        crate::util::encode_url_path(&url_path)
    ))
    .into_response()
}

#[derive(Deserialize)]
struct RecipeQuery {
    scale: Option<f64>,
    servings: Option<f64>,
}

async fn recipe_page(
    Path(path): Path<String>,
    Query(query): Query<RecipeQuery>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    // The same rules as `GET /api/recipes/{path}`: nothing hidden, and only a
    // recipe or a menu. Checked here rather than in the builder, since a static
    // build hands it paths from the recipe tree, not from a visitor.
    if !is_recipe_request(&path) {
        let error = if is_request_path(&path) {
            format!("Recipe not found: {path}")
        } else {
            format!("Invalid path: {path}")
        };
        tracing::error!("{error}");
        return error_page(lang, &state.url_prefix, error, features, viewer);
    }

    let scale = query.scale.unwrap_or(1.0);

    let aisle_file = state.aisle_file();
    let input = crate::web::builders::RecipeBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        recipe_path: &path,
        aisle_path: aisle_file.as_ref(),
        scale,
        servings: query.servings,
        lang: lang.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer: viewer.clone(),
    };

    match crate::web::builders::build_recipe_template(input) {
        Ok(crate::web::builders::RecipeBuildOutput::Recipe(template)) => template.into_response(),
        Ok(crate::web::builders::RecipeBuildOutput::Menu(mut template)) => {
            // A plan's calendar can change the plan for those who may.
            if viewer.can_edit_recipes() {
                let servings = template
                    .servings
                    .as_ref()
                    .map(|servings| servings.base.to_string());
                if let Some(plan) = template.plan.as_mut() {
                    super::handlers::plans::annotate(plan, &state.base_path, &path, servings).await;
                }
            }
            template.into_response()
        }
        Err(e) => {
            tracing::error!("Failed to build recipe template: {:?}", e);
            error_page(lang, &state.url_prefix, &e, features, viewer)
        }
    }
}

async fn edit_page(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    tracing::info!("Edit page requested for path: {}", path);

    // The same rules as the save and delete endpoints behind this page: inside
    // the recipe directory, nothing hidden, and only a `.cook` or `.menu` file.
    // Opening anything else here would show a file the page cannot save.
    if !is_request_path(&path) {
        tracing::error!("Invalid path: {path}");
        return error_page(
            lang,
            &state.url_prefix,
            format!("Invalid path: {path}"),
            features,
            viewer,
        );
    }

    let file_path = match recipe_file(&state.base_path, &path) {
        Ok(RecipeFile::Existing(file_path)) => file_path,
        _ => {
            tracing::error!("Recipe not found: {path}");
            return error_page(
                lang,
                &state.url_prefix,
                format!("Recipe not found: {path}"),
                features,
                viewer,
            );
        }
    };

    // The listing links recipes and menus without their extension, and the
    // Edit button keeps the path it was given, so `/edit/Week` can be a menu.
    // Send the editor to the file's full name: the toolbar mode below, and the
    // save, delete and picture calls the page makes, then all name the file.
    let requested_extension = camino::Utf8Path::new(&path).extension();
    if !matches!(requested_extension, Some("cook" | "menu")) {
        if let Some(extension) = file_path.extension() {
            // Encoded, since `Redirect` refuses a non-ASCII `Location`.
            return axum::response::Redirect::to(&format!(
                "{}/edit/{}",
                state.url_prefix,
                crate::util::encode_url_path(&format!("{path}.{extension}"))
            ))
            .into_response();
        }
    }

    // Read raw content
    let content = match tokio::fs::read_to_string(file_path).await {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to read recipe file: {e}");
            return error_page(
                lang,
                &state.url_prefix,
                format!("Failed to read recipe file: {e}"),
                features,
                viewer,
            );
        }
    };

    // Get recipe name from path
    let recipe_name = path
        .split('/')
        .next_back()
        .unwrap_or(&path)
        .replace(".cook", "")
        .replace(".menu", "");

    let is_menu = path.ends_with(".menu");

    let template = crate::web::templates::EditTemplate {
        active: "recipes".to_string(),
        recipe_name,
        is_menu,
        recipe_path: path,
        content,
        base_path: state.base_path.to_string(),
        max_image_bytes: super::title_image::MAX_UPLOAD_BYTES,
        max_image_edge: super::title_image::MAX_EDGE,
        tr: crate::web::templates::Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    };

    template.into_response()
}

/// What the new-file form creates: a `.cook` recipe, a `.menu` menu, or a
/// meal plan (a `.menu` whose frontmatter pins it to dates).
#[derive(Clone, Copy, PartialEq, Eq)]
enum NewKind {
    Recipe,
    Menu,
    Plan,
}

impl NewKind {
    /// No kind and `recipe` are a recipe, `menu` is a menu; anything else is
    /// not a kind of file this form makes.
    fn parse(kind: Option<&str>) -> Option<Self> {
        match kind {
            None | Some("recipe") => Some(Self::Recipe),
            Some("menu") => Some(Self::Menu),
            Some("plan") => Some(Self::Plan),
            Some(_) => None,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Recipe => "cook",
            Self::Menu | Self::Plan => "menu",
        }
    }

    /// The word error messages use for the file.
    fn noun(self) -> &'static str {
        match self {
            Self::Recipe => "recipe",
            Self::Menu => "menu",
            Self::Plan => "plan",
        }
    }

    /// The `kind` parameter that brings the form back for this kind of file.
    fn query(self) -> &'static str {
        match self {
            Self::Recipe => "",
            Self::Menu => "&kind=menu",
            Self::Plan => "&kind=plan",
        }
    }

    /// What a new file starts with, titled `name`. A plan starts from its
    /// frame instead: see [`crate::web::plan::plan_starter`].
    fn starter(self, name: &str) -> String {
        match self {
            Self::Recipe => format!("---\ntitle: {name}\n---\n\n"),
            // One day and one meal. The ` \` keeps the bullet below it in the
            // same meal: the menu page only groups lines that end with one.
            Self::Menu | Self::Plan => format!(
                "---\ntitle: {name}\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- \n"
            ),
        }
    }
}

/// The meals a new plan offers, as the form field and the translation of the
/// name written into the file (the editor toolbar's).
const PLAN_MEALS: [(&str, &str); 4] = [
    ("breakfast", "editor-toolbar-meal-breakfast"),
    ("lunch", "editor-toolbar-meal-lunch"),
    ("dinner", "editor-toolbar-meal-dinner"),
    ("snacks", "editor-toolbar-meal-snacks"),
];

/// A new plan's frame, as the form posts it and as an error sends it back.
/// Numbers stay text: a blank number field posts an empty string.
#[derive(Deserialize, Default)]
struct PlanFields {
    start: Option<String>,
    days: Option<String>,
    servings: Option<String>,
    breakfast: Option<String>,
    lunch: Option<String>,
    dinner: Option<String>,
    snacks: Option<String>,
}

/// A plan the form asked for, checked.
struct NewPlan {
    frame: crate::web::plan::PlanFrame,
    servings: u32,
}

impl PlanFields {
    fn has_meal(&self, field: &str) -> bool {
        match field {
            "breakfast" => self.breakfast.is_some(),
            "lunch" => self.lunch.is_some(),
            "dinner" => self.dinner.is_some(),
            "snacks" => self.snacks.is_some(),
            _ => false,
        }
    }

    /// The plan these fields ask for, meals named in `lang`, or what is wrong
    /// with them.
    fn plan(&self, lang: &LanguageIdentifier) -> Result<NewPlan, String> {
        use crate::web::plan::{PlanFrame, MAX_PLAN_DAYS, MIN_PLAN_DAYS};

        let start = self
            .start
            .as_deref()
            .and_then(|start| chrono::NaiveDate::parse_from_str(start.trim(), "%Y-%m-%d").ok())
            .ok_or("Pick the day the plan starts")?;
        let days = self
            .days
            .as_deref()
            .and_then(|days| days.trim().parse::<u32>().ok())
            .filter(|days| (MIN_PLAN_DAYS..=MAX_PLAN_DAYS).contains(days))
            .ok_or(format!(
                "A plan lasts from {MIN_PLAN_DAYS} to {MAX_PLAN_DAYS} days"
            ))?;
        let servings = match self.servings.as_deref().map(str::trim) {
            None | Some("") => 2,
            Some(servings) => servings
                .parse::<u32>()
                .ok()
                .filter(|&servings| servings > 0)
                .ok_or("Servings must be a whole number above zero")?,
        };
        let tr = Tr::new(lang.clone());
        let meals: Vec<String> = PLAN_MEALS
            .iter()
            .filter(|(field, _)| self.has_meal(field))
            .map(|(_, key)| tr.t(key))
            .collect();
        if meals.is_empty() {
            return Err("Pick at least one meal".to_string());
        }
        Ok(NewPlan {
            frame: PlanFrame { start, days, meals },
            servings,
        })
    }

    /// The query string that brings these fields back to the form.
    fn query(&self) -> String {
        let mut query = String::new();
        for (name, value) in [
            ("start", &self.start),
            ("days", &self.days),
            ("servings", &self.servings),
        ] {
            if let Some(value) = value {
                query.push_str(&format!("&{name}={}", urlencoding::encode(value)));
            }
        }
        for (field, _) in PLAN_MEALS {
            if self.has_meal(field) {
                query.push_str(&format!("&{field}=on"));
            }
        }
        query
    }
}

#[derive(Deserialize, Default)]
struct NewPageQuery {
    error: Option<String>,
    filename: Option<String>,
    kind: Option<String>,
    #[serde(flatten)]
    plan: PlanFields,
}

async fn new_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
    Query(query): Query<NewPageQuery>,
) -> impl IntoResponse {
    let kind = NewKind::parse(query.kind.as_deref());
    let tr = Tr::new(lang);
    // A fresh form (not one an error sent back) offers three meals a day.
    let fresh = query.plan.start.is_none() && query.plan.days.is_none();
    let plan = crate::web::templates::NewPlanForm {
        start: query.plan.start.clone().unwrap_or_else(|| {
            chrono::Local::now()
                .date_naive()
                .format("%Y-%m-%d")
                .to_string()
        }),
        days: query.plan.days.clone().unwrap_or_else(|| "7".to_string()),
        min_days: crate::web::plan::MIN_PLAN_DAYS,
        max_days: crate::web::plan::MAX_PLAN_DAYS,
        servings: query
            .plan
            .servings
            .clone()
            .unwrap_or_else(|| "2".to_string()),
        meals: PLAN_MEALS
            .iter()
            .map(|(field, key)| crate::web::templates::NewPlanMeal {
                field,
                label: tr.t(key),
                checked: if fresh {
                    *field != "snacks"
                } else {
                    query.plan.has_meal(field)
                },
            })
            .collect(),
    };
    crate::web::templates::NewTemplate {
        active: "recipes".to_string(),
        tr,
        error: query.error,
        filename: query.filename,
        is_menu: matches!(kind, Some(NewKind::Menu | NewKind::Plan)),
        is_plan: kind == Some(NewKind::Plan),
        plan,
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    }
}

#[derive(Deserialize)]
struct NewRecipeForm {
    filename: String,
    kind: Option<String>,
    /// Only a plan's form has these.
    #[serde(flatten)]
    plan: PlanFields,
}

/// Helper to build redirect URL with error message, back to the form for the
/// same kind of file; `back` is the rest of the form's query (`NewKind::query`
/// and, for a plan, `PlanFields::query`).
fn new_page_error(
    prefix: &str,
    back: &str,
    error: &str,
    filename: &str,
) -> axum::response::Response {
    let encoded_error = urlencoding::encode(error);
    let encoded_filename = urlencoding::encode(filename);
    axum::response::Redirect::to(&format!(
        "{prefix}/new?error={}&filename={}{back}",
        encoded_error, encoded_filename
    ))
    .into_response()
}

/// Validates that the request came from a page the server trusts: its own
/// address, or a `--cors-origin` (CSRF protection)
fn validate_same_origin(headers: &HeaderMap, host: &str, cors: &super::cors::CorsConfig) -> bool {
    // Origin first: it is the header a browser always sends on a form POST,
    // and the one an attacker cannot forge.
    if let Some(origin) = headers.get(header::ORIGIN) {
        return origin
            .to_str()
            .is_ok_and(|origin| cors.trusts(origin, host));
    }

    // Referer is less reliable but better than nothing. The write guard lets
    // a request without Origin through, so this is the only check it gets.
    if let Some(referer) = headers.get(header::REFERER) {
        return referer
            .to_str()
            .ok()
            .and_then(super::cors::origin_of)
            .is_some_and(|origin| cors.trusts(&origin, host));
    }

    // Neither header: reject. Browsers always send one for a form submission,
    // unlike the API, where a missing Origin just means a non-browser client.
    false
}

async fn create_recipe(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Extension(lang): Extension<LanguageIdentifier>,
    headers: HeaderMap,
    Form(form): Form<NewRecipeForm>,
) -> impl IntoResponse {
    // The raw `Host` header, not axum-extra's `Host` extractor: that one
    // prefers `X-Forwarded-Host`, which any client can set.
    let host = super::cors::host_header(&headers).unwrap_or_default();
    if state.csrf_check && !validate_same_origin(&headers, host, &state.cors) {
        tracing::warn!("CSRF validation failed for create_recipe request");
        return (StatusCode::FORBIDDEN, "Invalid request origin").into_response();
    }

    let Some(kind) = NewKind::parse(form.kind.as_deref()) else {
        return (StatusCode::BAD_REQUEST, "Unknown kind of file").into_response();
    };
    let noun = kind.noun();
    let back = match kind {
        NewKind::Plan => format!("{}{}", kind.query(), form.plan.query()),
        _ => kind.query().to_string(),
    };

    let original_filename = form.filename.clone();

    let plan = match kind {
        NewKind::Plan => match form.plan.plan(&lang) {
            Ok(plan) => Some(plan),
            Err(error) => {
                return new_page_error(&state.url_prefix, &back, &error, &original_filename);
            }
        },
        _ => None,
    };

    // Validate input before sanitization
    if form.filename.trim().is_empty() {
        return new_page_error(
            &state.url_prefix,
            &back,
            &format!("{} name cannot be empty", capitalize(noun)),
            &original_filename,
        );
    }

    let recipe_path = new_file_path(&form.filename);
    if recipe_path.is_empty() {
        return new_page_error(
            &state.url_prefix,
            &back,
            &format!("{} name cannot be empty", capitalize(noun)),
            &original_filename,
        );
    }

    let file_path = state
        .base_path
        .join(format!("{recipe_path}.{}", kind.extension()));

    // The recipe directory, resolved, for the containment check below
    let base_path_clone = state.base_path.clone();
    let base_canonical =
        match tokio::task::spawn_blocking(move || base_path_clone.canonicalize_utf8()).await {
            Ok(Ok(p)) => p,
            _ => {
                return new_page_error(
                    &state.url_prefix,
                    &back,
                    "Internal error: invalid base path",
                    &original_filename,
                );
            }
        };

    // Decide before touching the disk: the folder the file goes in, resolved
    // through any symlink as far as it exists, must sit under the recipe
    // directory. Nothing is created until that holds, so a refusal leaves
    // nothing behind — and nothing that already existed is ever removed. A
    // sub-folder symlinked elsewhere (a NAS share) used to be deleted here by
    // a "clean-up" of the folder this request had not created (#549).
    if let Some(parent) = file_path.parent() {
        let parent_owned = parent.to_owned();
        let base = base_canonical.clone();
        let inside = tokio::task::spawn_blocking(move || {
            super::canonical_or_nearest(&parent_owned).starts_with(&base)
        })
        .await
        .unwrap_or(false);
        if !inside {
            tracing::warn!("Refused to create {file_path}: it resolves outside {base_canonical}");
            return new_page_error(
                &state.url_prefix,
                &back,
                &format!("Invalid {noun} path"),
                &original_filename,
            );
        }

        if !parent.exists() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                tracing::error!("Failed to create directories: {}", e);
                return new_page_error(&state.url_prefix, &back, "Failed to create directory. Check that the recipes folder has write permissions.", &original_filename);
            }
        }
    }

    // Get the recipe name (last component of path) for the title
    let recipe_name = recipe_path
        .split('/')
        .next_back()
        .unwrap_or(&recipe_path)
        .replace(['-', '_'], " ");

    // Start the file with YAML frontmatter
    let template = match &plan {
        Some(plan) => {
            crate::web::plan::plan_starter(&recipe_name, plan.servings, &plan.frame, &lang)
        }
        None => kind.starter(&recipe_name),
    };

    // Use OpenOptions with create_new to atomically check existence and create
    // This prevents TOCTOU race conditions
    use tokio::io::AsyncWriteExt;
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true) // Fails if file exists - atomic check + create
        .open(&file_path)
        .await;

    match file {
        Ok(mut f) => {
            // A tokio `File` hands writes to a background task: without the
            // flush, the redirect below can reach the browser, and the editor
            // load the file, before anything is on disk.
            let written = match f.write_all(template.as_bytes()).await {
                Ok(()) => f.flush().await,
                Err(e) => Err(e),
            };
            if let Err(e) = written {
                tracing::error!("Failed to write recipe: {}", e);
                return new_page_error(
                    &state.url_prefix,
                    &back,
                    &format!("Failed to write {noun} file"),
                    &original_filename,
                );
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return new_page_error(
                &state.url_prefix,
                &back,
                &format!("A {noun} with this name already exists"),
                &original_filename,
            );
        }
        Err(e) => {
            tracing::error!("Failed to create recipe file: {}", e);
            return new_page_error(
                &state.url_prefix,
                &back,
                &format!("Failed to create {noun} file"),
                &original_filename,
            );
        }
    }

    activity::record(
        &viewer,
        format_args!("created {}", activity::file(&state.base_path, &file_path)),
    );

    // Redirect to editor
    axum::response::Redirect::to(&format!(
        "{}/edit/{}.{}",
        state.url_prefix,
        recipe_path,
        kind.extension()
    ))
    .into_response()
}

/// The path, without extension, that `POST /new` creates for the name typed
/// into the form.
///
/// Only letters, digits, spaces, `-`, `_` and `/` are kept. Each folder and
/// the file name is then trimmed and empty ones dropped, so ` Mains / Stew `
/// becomes `Mains/Stew` rather than a folder `Mains ` holding ` Stew.cook`.
/// Spaces inside a name are kept as typed.
fn new_file_path(filename: &str) -> String {
    let kept: String = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '/'))
        .collect();

    kept.split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// `recipe` -> `Recipe`, for an error message that opens with the noun.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

async fn shopping_list_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> impl IntoResponse {
    ShoppingListTemplate {
        active: "shopping".to_string(),
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    }
}

async fn pantry_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut sections = Vec::new();
    let mut counts = PantryCounts::default();
    let mut load_error = None;

    if let Some(path) = state.pantry_path.clone() {
        let ctx = cookcli_core::Context::new(state.base_path.clone())
            .with_pantry(cookcli_core::ConfigSource::Path(path));
        match tokio::task::spawn_blocking(move || cookcli_core::pantry::load(&ctx)).await {
            Ok(Ok(outcome)) => {
                (sections, counts) =
                    pantry_sections(&outcome.value, chrono::Local::now().date_naive());
            }
            Ok(Err(error)) => load_error = Some(error.to_string()),
            Err(error) => {
                tracing::error!("Reading the pantry did not finish: {error}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(PantryTemplate {
        active: "pantry".to_string(),
        configured: state.pantry_path.is_some(),
        sections,
        counts,
        load_error,
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    })
}

/// The pantry as the page lists it: each item with its stock and expiry
/// already judged, by the same rules as `cook pantry depleted` and `cook
/// pantry expiring`, and the counts behind the filters.
fn pantry_sections(
    contents: &cookcli_core::pantry::PantryContents,
    today: chrono::NaiveDate,
) -> (Vec<PantrySection>, PantryCounts) {
    let soon = i64::from(cookcli_core::pantry::ExpiringRequest::default().days);
    let mut counts = PantryCounts::default();
    let sections = contents
        .sections
        .iter()
        .map(|section| PantrySection {
            name: section.name.clone(),
            items: section
                .items
                .iter()
                .map(|item| {
                    let stock = if item.is_out() {
                        "out"
                    } else if item.is_depleted() {
                        "low"
                    } else {
                        "ok"
                    };
                    let days = item.days_until_expiry(today);
                    let expiry = match days {
                        None => "",
                        Some(days) if days < 0 => "expired",
                        Some(0) => "today",
                        Some(days) if days <= soon => "soon",
                        Some(_) => "later",
                    };
                    counts.all += 1;
                    counts.low += usize::from(stock == "low");
                    counts.out += usize::from(stock == "out");
                    counts.expiring += usize::from(matches!(expiry, "expired" | "today" | "soon"));
                    PantryItem {
                        name: item.name.clone(),
                        quantity: item.quantity.clone(),
                        bought: item.bought.clone(),
                        expire: item.expire.clone(),
                        low: item.low.clone(),
                        quantity_text: item
                            .quantity
                            .as_deref()
                            .map(shown_quantity)
                            .unwrap_or_default(),
                        low_text: item.low.as_deref().map(shown_quantity).unwrap_or_default(),
                        stock,
                        expiry,
                        expiry_days: days.map_or(0, |days| {
                            usize::try_from(days.unsigned_abs()).unwrap_or(usize::MAX)
                        }),
                    }
                })
                .collect(),
        })
        .collect();
    (sections, counts)
}

/// A pantry quantity as people read it: `250 g` for `250%g`, the `%` that
/// separates number and unit in the file becoming a space.
fn shown_quantity(quantity: &str) -> String {
    quantity
        .split('%')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

async fn aisles_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> axum::response::Response {
    let aisles = match crate::server::handlers::aisles::load(&state).await {
        Ok(aisles) => aisles,
        Err((status, _)) => {
            let mut response = error_page(
                lang,
                &state.url_prefix,
                "Failed to read the aisle file",
                features,
                viewer,
            );
            *response.status_mut() = status;
            return response;
        }
    };

    AislesTemplate {
        active: "shopping".to_string(),
        aisles: serde_json::to_value(aisles).unwrap_or_default(),
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    }
    .into_response()
}

async fn preferences_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> impl IntoResponse {
    // The sync section names the linked cook.md account and can link or
    // unlink one, so only admins get it.
    let sync_enabled = cfg!(feature = "sync") && viewer.can_admin();
    #[cfg(feature = "sync")]
    let (sync_logged_in, sync_email, sync_syncing) = if sync_enabled {
        let (logged_in, email, syncing, _reason) = state.sync_status().await;
        (logged_in, email, syncing)
    } else {
        (false, None, false)
    };
    #[cfg(not(feature = "sync"))]
    let (sync_logged_in, sync_email, sync_syncing) = (false, None, false);

    // Full paths tell the server's directory layout and, usually, the
    // account it runs as, so only admins get them. Everyone else sees where
    // a file is inside the recipe directory, and a recipes-only visitor, who
    // gets the language picker alone, sees nothing.
    let tr = Tr::new(lang);
    let path = |path: Option<&camino::Utf8PathBuf>| match path {
        _ if viewer.recipes_only() => String::new(),
        None => tr.t("pref-not-configured"),
        Some(path) if viewer.can_admin() => path.to_string(),
        Some(path) => match path.strip_prefix(&state.base_path) {
            Ok(relative) => relative.to_string(),
            Err(_) => tr.t("pref-global-config"),
        },
    };
    let aisle_path = path(state.aisle_file().as_ref());
    let pantry_path = path(state.pantry_path.as_ref());
    let base_path = if viewer.can_admin() {
        state.base_path.to_string()
    } else {
        String::new()
    };

    PreferencesTemplate {
        active: "preferences".to_string(),
        aisle_path,
        pantry_path,
        base_path,
        version: format!("{} - in food we trust", env!("CARGO_PKG_VERSION")),
        tr,
        sync_enabled,
        sync_logged_in,
        sync_email,
        sync_syncing,
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    }
}

async fn atom_feed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    Extension(lang): Extension<LanguageIdentifier>,
) -> axum::response::Response {
    feed_response(
        crate::build::feed::FeedFormat::Atom,
        &state,
        &headers,
        &uri,
        lang,
    )
    .await
}

async fn rss_feed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    Extension(lang): Extension<LanguageIdentifier>,
) -> axum::response::Response {
    feed_response(
        crate::build::feed::FeedFormat::Rss,
        &state,
        &headers,
        &uri,
        lang,
    )
    .await
}

async fn json_feed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    Extension(lang): Extension<LanguageIdentifier>,
) -> axum::response::Response {
    feed_response(
        crate::build::feed::FeedFormat::Json,
        &state,
        &headers,
        &uri,
        lang,
    )
    .await
}

/// Absolute URL of the site root for links in a feed.
///
/// Feeds need absolute links. Like `api_docs_page`, the host is the authority
/// the request was addressed to, never `X-Forwarded-Host` (see
/// `cors::request_authority`). `X-Forwarded-Proto: https` is honoured so a
/// TLS-terminating proxy yields https links; it can only change the scheme
/// of the response sent back to the same client.
fn feed_base_url(headers: &HeaderMap, uri: &Uri, url_prefix: &str) -> Option<String> {
    let host = super::cors::request_authority(headers, uri)?;
    let scheme = if forwarded_https(headers) {
        "https"
    } else {
        "http"
    };
    let base = format!("{scheme}://{host}{url_prefix}/");
    // Reject a Host that would smuggle a path, query or credentials into the
    // links: it must parse back to exactly scheme + authority + prefix.
    let parsed = url::Url::parse(&base).ok()?;
    let clean = parsed.host().is_some()
        && parsed.username().is_empty()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
        && parsed.path() == format!("{url_prefix}/");
    clean.then_some(base)
}

/// Whether a TLS-terminating proxy says the client connected over https
/// (`X-Forwarded-Proto: https`). Anyone can send the header, so it only ever
/// decides things about the response to that same client.
pub(super) fn forwarded_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"))
}

async fn feed_response(
    format: crate::build::feed::FeedFormat,
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    lang: LanguageIdentifier,
) -> axum::response::Response {
    use fluent_templates::Loader;

    let Some(base) = feed_base_url(headers, uri, &state.url_prefix) else {
        return (StatusCode::BAD_REQUEST, "Missing or invalid Host header").into_response();
    };
    let title = crate::web::i18n::LOCALES.lookup(&lang, "recipes-title");
    let lang_tag = lang.to_string();
    let base_path = state.base_path.clone();

    // Walking the recipe tree reads every file: keep it off the async runtime.
    let rendered = tokio::task::spawn_blocking(move || {
        let tree = cooklang_find::build_tree(&base_path)
            .map_err(|e| anyhow::anyhow!("Failed to build recipe tree: {e}"))?;
        Ok::<_, anyhow::Error>(crate::build::feed::render_feed(
            format,
            &tree,
            crate::build::feed::PageUrls::Server,
            &base,
            &title,
            &lang_tag,
        ))
    })
    .await;

    match rendered {
        Ok(Ok(xml)) => ([(header::CONTENT_TYPE, format.content_type())], xml).into_response(),
        Ok(Err(e)) => {
            tracing::error!("Failed to render feed: {e:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to render feed").into_response()
        }
        Err(e) => {
            tracing::error!("Feed task failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to render feed").into_response()
        }
    }
}

async fn api_docs_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Extension(viewer): Extension<Viewer>,
) -> impl IntoResponse {
    // Rendered so integrators can copy a working URL rather than a relative
    // path. The authority the request was addressed to, not `Forwarded` /
    // `X-Forwarded-Host`: see `cors::request_authority`. Behind a proxy that
    // rewrites `Host` this shows the internal name, which is the same
    // trade-off the CSRF guard makes. A request that carries no usable
    // authority at all is malformed under HTTP/1.1, so fall back to the
    // relative path rather than printing `http:///api`.
    let base_url = match super::cors::request_authority(&headers, &uri) {
        Some(host) => format!("http://{host}{}/api", state.url_prefix),
        None => format!("{}/api", state.url_prefix),
    };

    ApiDocsTemplate {
        active: "preferences".to_string(),
        base_url,
        preamble: crate::web::api_docs::preamble(),
        sections: crate::web::api_docs::sections(),
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        repo_url: None,
        features,
        viewer,
    }
}

#[cfg(test)]
mod new_file_path_tests {
    use super::new_file_path;

    #[test]
    fn names_and_folders_are_trimmed() {
        assert_eq!(new_file_path(" Soup "), "Soup");
        assert_eq!(new_file_path(" Mains / Stew "), "Mains/Stew");
        assert_eq!(new_file_path("\tMains\t/Stew\n"), "Mains/Stew");
    }

    #[test]
    fn inner_spaces_are_kept() {
        assert_eq!(new_file_path("Beef  and Beer Stew"), "Beef  and Beer Stew");
    }

    #[test]
    fn empty_and_blank_folders_are_dropped() {
        assert_eq!(new_file_path("/Mains//  /Stew/"), "Mains/Stew");
        assert_eq!(new_file_path(" / "), "");
        assert_eq!(new_file_path("../.."), "");
    }
}

#[cfg(test)]
mod feed_url_tests {
    use super::feed_base_url;
    use axum::http::{HeaderMap, HeaderValue, Uri};

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(*name, HeaderValue::from_static(value));
        }
        map
    }

    fn uri() -> Uri {
        "/atom.xml".parse().unwrap()
    }

    #[test]
    fn uses_host_and_prefix() {
        let h = headers(&[("host", "cook.lan:9080")]);
        assert_eq!(
            feed_base_url(&h, &uri(), "").as_deref(),
            Some("http://cook.lan:9080/")
        );
        assert_eq!(
            feed_base_url(&h, &uri(), "/cook").as_deref(),
            Some("http://cook.lan:9080/cook/")
        );
    }

    #[test]
    fn honours_forwarded_proto_but_not_forwarded_host() {
        let h = headers(&[
            ("host", "cook.example.com"),
            ("x-forwarded-proto", "https"),
            ("x-forwarded-host", "evil.test"),
        ]);
        assert_eq!(
            feed_base_url(&h, &uri(), "").as_deref(),
            Some("https://cook.example.com/")
        );
    }

    #[test]
    fn rejects_missing_or_smuggling_hosts() {
        assert_eq!(feed_base_url(&headers(&[]), &uri(), ""), None);
        for bad in [
            "evil.test/path",
            "user@evil.test",
            "evil.test?x=1",
            "evil.test#x",
        ] {
            let h = headers(&[("host", bad)]);
            assert_eq!(feed_base_url(&h, &uri(), ""), None, "{bad}");
        }
    }
}
