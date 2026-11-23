//! Cooking mode keeps notes in file order without turning them into steps.
use camino::Utf8Path;
use cookcli::web::builders::{build_recipe_template, RecipeBuildInput, RecipeBuildOutput};
use cookcli::web::language::{FeatureFlags, EN_US};
use cookcli::web::templates::RecipeTemplate;
use serde_json::Value;
use tempfile::TempDir;

const ACCEPTANCE: &str = "= Sear\n\n> A\n\nSalt @chicken thighs{4}.\n\n> B\n\n> C\n\nFry in a #pan{} for ~{12%minutes}.\n\n> D\n\n= Rest\n\n> E\n";

fn recipe(text: &str, static_mode: bool) -> Box<RecipeTemplate> {
    let dir = TempDir::new().unwrap();
    let base = Utf8Path::from_path(dir.path()).unwrap();
    std::fs::write(base.join("Notes.cook"), text).unwrap();
    match build_recipe_template(RecipeBuildInput {
        base_path: base,
        url_prefix: "",
        recipe_path: "Notes",
        aisle_path: None,
        scale: 1.0,
        servings: None,
        lang: EN_US,
        static_mode,
        repo_url: None,
        features: FeatureFlags::default(),
        viewer: Default::default(),
    })
    .unwrap()
    {
        RecipeBuildOutput::Recipe(template) => template,
        RecipeBuildOutput::Menu(_) => panic!("expected a recipe"),
    }
}

#[test]
fn cooking_data_preserves_notes_and_legacy_steps_in_file_order() {
    for static_mode in [false, true] {
        let data: Value =
            serde_json::from_str(&recipe(ACCEPTANCE, static_mode).cooking_mode_json()).unwrap();
        let sections = data["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        let items = sections[0]["items"]
            .as_array()
            .expect("ordered notes and steps");
        assert_eq!(
            items
                .iter()
                .map(|v| v["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["note", "step", "note", "note", "step", "note"]
        );
        assert_eq!(items[0]["text"], "A");
        assert_eq!(items[2]["text"], "B");
        assert_eq!(items[3]["text"], "C");
        assert_eq!(items[5]["text"], "D");
        assert_eq!(items[1]["globalNumber"], 1);
        assert_eq!(items[4]["globalNumber"], 2);
        let steps: Vec<_> = items
            .iter()
            .filter(|v| v["type"] == "step")
            .cloned()
            .collect();
        assert_eq!(sections[0]["steps"], serde_json::json!(steps));
        assert_eq!(
            sections[1]["items"],
            serde_json::json!([{"type":"note", "text":"E"}])
        );
        assert!(sections[1]["steps"].as_array().unwrap().is_empty());
        assert_eq!(data["noteLabel"], "Note");
    }
}

#[test]
fn cooking_notes_remain_plain_text_and_cannot_close_the_data_script() {
    let template = recipe(
        "> <script>alert('note')</script> & https://example.com/technique?q=1&lang=en\n",
        false,
    );
    let json = template.cooking_mode_json();
    assert!(!json.contains("</script>"));
    let data: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        data["sections"][0]["items"][0]["text"],
        "<script>alert('note')</script> & https://example.com/technique?q=1&lang=en"
    );
}

#[test]
fn steps_after_note_only_sections_keep_global_numbers() {
    let template = recipe(
        "= Before\n\n> Only a note\n\n= Cook\n\nMix @water{1%l}.\n\nRest.\n",
        false,
    );
    let data: Value = serde_json::from_str(&template.cooking_mode_json()).unwrap();
    let steps = data["sections"][1]["items"]
        .as_array()
        .expect("steps after notes");
    assert_eq!(steps[0]["number"], 1);
    assert_eq!(steps[0]["globalNumber"], 1);
    assert_eq!(steps[1]["globalNumber"], 2);
    assert_eq!(data["sections"][1]["steps"], serde_json::json!(steps));
}
