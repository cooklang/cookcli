// Step image resolution for sectioned recipes (issue #374).
//
// Two naming conventions exist for step images:
// - Linear:  Recipe.N.ext   where N counts steps continuously across sections
// - Sectioned: Recipe.S.N.ext  where S is the section and N the step within it
//
// cooklang-find indexes both; the web builder must resolve either.

use camino::Utf8Path;
use cookcli::web::builders::{build_recipe_template, RecipeBuildInput, RecipeBuildOutput};
use cookcli::web::language::{FeatureFlags, EN_US};
use cookcli::web::templates::{RecipeSectionItem, RecipeTemplate};

const TWO_SECTION_RECIPE: &str =
    "= Prep\n\nChop the @onion{1}.\n\n= Cook\n\nFry the onion in @oil{1%tbsp}.\n";

fn build(dir: &Utf8Path, recipe_name: &str) -> RecipeTemplate {
    let output = build_recipe_template(RecipeBuildInput {
        base_path: dir,
        url_prefix: "",
        recipe_path: recipe_name,
        aisle_path: None,
        scale: 1.0,
        lang: EN_US,
        static_mode: false,
        repo_url: None,
        features: FeatureFlags::default(),
    })
    .expect("failed to build recipe template");

    match output {
        RecipeBuildOutput::Recipe(template) => *template,
        RecipeBuildOutput::Menu(_) => panic!("expected a recipe, got a menu"),
    }
}

fn step_image_paths(template: &RecipeTemplate) -> Vec<Option<String>> {
    template
        .sections
        .iter()
        .flat_map(|s| s.items.iter())
        .filter_map(|item| match item {
            RecipeSectionItem::Step(step) => Some(step.image_path.clone()),
            RecipeSectionItem::Note(_) => None,
        })
        .collect()
}

#[test]
fn linear_step_images_resolve_across_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = Utf8Path::from_path(tmp.path()).unwrap();
    std::fs::write(dir.join("Linear.cook"), TWO_SECTION_RECIPE).unwrap();
    std::fs::write(dir.join("Linear.1.jpg"), []).unwrap();
    std::fs::write(dir.join("Linear.2.jpg"), []).unwrap();

    let images = step_image_paths(&build(dir, "Linear"));
    assert_eq!(images.len(), 2);
    assert!(
        images[0].as_deref().unwrap_or("").ends_with("Linear.1.jpg"),
        "step 1 should show Linear.1.jpg, got {:?}",
        images[0]
    );
    assert!(
        images[1].as_deref().unwrap_or("").ends_with("Linear.2.jpg"),
        "step 2 (section 2) should show Linear.2.jpg, got {:?}",
        images[1]
    );
}

#[test]
fn sectioned_step_images_resolve_per_section() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = Utf8Path::from_path(tmp.path()).unwrap();
    std::fs::write(dir.join("Sectioned.cook"), TWO_SECTION_RECIPE).unwrap();
    std::fs::write(dir.join("Sectioned.1.1.jpg"), []).unwrap();
    std::fs::write(dir.join("Sectioned.2.1.jpg"), []).unwrap();

    let images = step_image_paths(&build(dir, "Sectioned"));
    assert_eq!(images.len(), 2);
    assert!(
        images[0]
            .as_deref()
            .unwrap_or("")
            .ends_with("Sectioned.1.1.jpg"),
        "section 1 step 1 should show Sectioned.1.1.jpg, got {:?}",
        images[0]
    );
    assert!(
        images[1]
            .as_deref()
            .unwrap_or("")
            .ends_with("Sectioned.2.1.jpg"),
        "section 2 step 1 should show Sectioned.2.1.jpg, got {:?}",
        images[1]
    );
}

#[test]
fn section_convention_wins_over_linear_for_same_step() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = Utf8Path::from_path(tmp.path()).unwrap();
    std::fs::write(dir.join("Mixed.cook"), TWO_SECTION_RECIPE).unwrap();
    // Both conventions present for the second step (section 2, global step 2).
    std::fs::write(dir.join("Mixed.2.jpg"), []).unwrap();
    std::fs::write(dir.join("Mixed.2.1.jpg"), []).unwrap();

    let images = step_image_paths(&build(dir, "Mixed"));
    assert_eq!(images.len(), 2);
    assert!(
        images[1]
            .as_deref()
            .unwrap_or("")
            .ends_with("Mixed.2.1.jpg"),
        "the section-specific image should win, got {:?}",
        images[1]
    );
}
