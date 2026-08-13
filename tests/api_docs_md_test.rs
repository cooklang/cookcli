//! Keeps `docs/api.md` in step with `src/web/api_docs.rs`.
//!
//! The `/api-docs` page is already guarded against the router drifting away
//! from it; this closes the remaining gap, where the checked-in Markdown (and
//! so the cooklang.org page synced from it) falls behind both.

#![cfg(feature = "server")]

use std::path::PathBuf;

fn docs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/api.md")
}

#[test]
fn docs_api_md_is_up_to_date() {
    let path = docs_path();
    let expected = cookcli::web::api_docs_md::render();

    if std::env::var_os("UPDATE_API_DOCS").is_some() {
        std::fs::write(&path, &expected).expect("failed to write docs/api.md");
        eprintln!("regenerated {}", path.display());
        return;
    }

    let actual = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "could not read {}: {e}\n\
             Run `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test` to generate it.",
            path.display()
        )
    });
    // Git on Windows may check the file out with CRLF endings; compare content,
    // not line endings.
    let actual = actual.replace("\r\n", "\n");

    assert_eq!(
        actual, expected,
        "\ndocs/api.md is stale — it no longer matches src/web/api_docs.rs.\n\
         Run `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test` to regenerate it,\n\
         and re-run cooklang.org's scripts/sync-cli-docs.sh to update the website page.\n"
    );
}
