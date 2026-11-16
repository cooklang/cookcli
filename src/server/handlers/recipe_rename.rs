//! `POST /api/recipe_rename/{*path}`: renames a `.cook` or `.menu` file in its
//! own folder, moves its pictures along, and rewrites the references other
//! recipes and menus make to it.
//!
//! Not a method on `/api/recipes/{*path}`: a rename answers with more than a
//! save does, and a route of its own keeps it out of that catch-all's way the
//! way `/api/recipe_image/` is.
//!
//! What it may touch, and nothing else:
//!
//! - the file itself, resolved by [`recipe_file`] like every other recipe
//!   endpoint, so only an existing `.cook` or `.menu` file and never a link;
//! - its pictures ([`rename::pictures_of`]);
//! - `.cook` and `.menu` files holding a reference that names it exactly.
//!
//! Nothing is ever replaced: every move refuses an existing destination
//! ([`fs_atomic::move_file`]), and a reference is only rewritten in a file that
//! has not changed since it was read.

use crate::server::{
    activity, fs_atomic,
    handlers::common::{check_path, json_error, new_file_name, recipe_file, ApiError, RecipeFile},
    rename::{self, Target},
    AppState,
};
use crate::web::viewer::Viewer;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use std::collections::HashSet;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Held by everything that writes a recipe or menu file, or a picture beside
/// one, through the API: a save landing in the middle of a rename could be
/// overwritten by a rewritten reference, or put a file back under the old
/// name between two moves.
pub static RECIPE_FILES: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    /// The new name, without folder; the extension is optional.
    name: String,
}

/// A file holding references to rewrite, with the text they were found in.
struct ReferencingFile {
    path: Utf8PathBuf,
    read: String,
    rewritten: String,
    count: usize,
}

/// What [`find_references`] found across the collection.
#[derive(Default)]
struct References {
    files: Vec<ReferencingFile>,
    /// `{ file, line }` of the references left alone; `line` is null when it
    /// is the whole file.
    skipped: Vec<serde_json::Value>,
}

pub async fn recipe_rename(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(request): Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Run apart from the request: a client that goes away drops this future,
    // and a rename stopped between two moves would neither finish nor roll
    // back. A spawned task runs to its end either way.
    let base = state.base_path.clone();
    tokio::spawn(rename(base, viewer, path, request.name))
        .await
        .map_err(|e| io_error(io::Error::other(e)))?
}

async fn rename(
    base: Utf8PathBuf,
    viewer: Viewer,
    path: String,
    name: String,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _writing = RECIPE_FILES.lock().await;

    let RecipeFile::Existing(old) = recipe_file(&base, &path)? else {
        return Err(not_found(&path));
    };
    if is_link(&old) {
        return Err(bad_request(format!(
            "{path} is a symbolic link; rename the file it points to instead"
        )));
    }

    let (Some(dir), Some(old_name), Some(old_stem), Some(ext)) = (
        old.parent(),
        old.file_name(),
        old.file_stem(),
        old.extension(),
    ) else {
        return Err(not_found(&path));
    };
    let dir = dir.to_owned();
    // A folder that is a link to somewhere else would take the moves with it.
    let inside = match (base.canonicalize_utf8(), dir.canonicalize_utf8()) {
        (Ok(base), Ok(dir)) => dir.starts_with(base),
        _ => false,
    };
    if !inside {
        return Err(bad_request(format!(
            "{path} is not inside the recipe directory once links are followed"
        )));
    }
    let new_stem = new_file_name(&name, ext)?;
    if new_stem == old_stem {
        return Err(bad_request(format!("{path} already has that name")));
    }

    let new_name = format!("{new_stem}.{ext}");
    let new = dir.join(&new_name);
    let relative_dir = relative(&base, &dir)?;
    let old_relative = relative_dir.join(old_name);
    let new_relative = relative_dir.join(&new_name);
    // Belt and braces: the new name passed `new_file_name`, so this holds.
    check_path(new_relative.as_str())?;

    // `pasta` to `Pasta`: on a case-insensitive file system the new name is
    // the file itself, which is not a clash.
    let case_only = new_stem.to_lowercase() == old_stem.to_lowercase();
    let same_file = |from: &Utf8Path, to: &Utf8Path| {
        case_only && same_file::is_same_file(from, to).unwrap_or(false)
    };

    if exists(&new) && !same_file(&old, &new) {
        return Err(conflict(format!("{new_relative} already exists")));
    }
    if ext == "cook" && !case_only && dir.join(format!("{new_stem}.menu")).is_file() {
        // `@./Week{}` reaches `Week.cook` before `Week.menu`, so every
        // reference to the menu would quietly start naming this recipe.
        return Err(conflict(format!(
            "A menu named {new_stem} is in that folder; references to it would \
             reach this recipe instead"
        )));
    }

    // Pictures: each must be a plain file, and land on a free name that no
    // other recipe or menu would show as its own.
    let names = folder_files(&dir).await.map_err(io_error)?;
    let mut moves: Vec<(Utf8PathBuf, Utf8PathBuf)> = Vec::new();
    let mut names_after: Vec<String> = names
        .iter()
        .filter(|name| name.as_str() != old_name)
        .cloned()
        .collect();
    names_after.push(new_name.clone());
    for picture in rename::pictures_of(&names, old_name) {
        let from = dir.join(&picture);
        if is_link(&from) {
            return Err(bad_request(format!(
                "{} is a symbolic link; move it by hand first",
                relative_dir.join(&picture)
            )));
        }
        let renamed = format!("{new_stem}{}", &picture[old_stem.len()..]);
        let to = dir.join(&renamed);
        if exists(&to) && !same_file(&from, &to) {
            return Err(conflict(format!(
                "{} already exists",
                relative_dir.join(&renamed)
            )));
        }
        if let Some(claimant) = rename::other_claimant(&names_after, &new_name, &renamed) {
            return Err(conflict(format!(
                "The picture {renamed} would belong to {claimant} as well"
            )));
        }
        moves.push((from, to));
    }
    moves.push((old.clone(), new.clone()));

    // Every reference is found before anything moves, and a collection that
    // cannot be read in full is not renamed: a reference in the part that
    // could not be read would be left behind without anyone knowing.
    let shadowed = ext == "menu" && dir.join(format!("{old_stem}.cook")).is_file();
    let spell_extension =
        new_stem.contains('.') || (ext == "menu" && dir.join(format!("{new_stem}.cook")).is_file());
    let references = {
        let (base, relative_dir) = (base.clone(), relative_dir.clone());
        let (old_stem, new_stem, ext) = (old_stem.to_string(), new_stem.clone(), ext.to_string());
        tokio::task::spawn_blocking(move || {
            find_references(
                &base,
                &Target {
                    dir: &relative_dir,
                    old_stem: &old_stem,
                    new_stem: &new_stem,
                    ext: &ext,
                    shadowed,
                    spell_extension,
                },
            )
        })
        .await
        .map_err(|e| io_error(io::Error::other(e)))?
        .map_err(|e| {
            tracing::error!("Not renaming {path}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(format!(
                    "Nothing was renamed: the references to {path} could not all be \
                     looked for ({e})"
                )),
            )
        })?
    };

    // Move, undoing what was done if any step fails.
    for (done, (from, to)) in moves.iter().enumerate() {
        if let Err(e) = fs_atomic::move_file_async(from.clone(), to.clone()).await {
            tracing::error!("Failed to move {from} to {to}: {e}");
            for (from, to) in moves[..done].iter().rev() {
                if let Err(e) = fs_atomic::move_file_async(to.clone(), from.clone()).await {
                    tracing::error!("Failed to move {to} back to {from}: {e}");
                }
            }
            return Err(if e.kind() == io::ErrorKind::AlreadyExists {
                conflict(format!("{} already exists", relative(&base, to)?))
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json_error(format!("Failed to rename {path}: {e}")),
                )
            });
        }
    }
    activity::record(
        &viewer,
        format_args!(
            "renamed {} to {}",
            activity::file(&base, &old),
            activity::file(&base, &new)
        ),
    );

    // The references. The rename stands whatever happens here: some files may
    // already name the new file.
    let mut skipped = references.skipped;
    let mut updated = Vec::new();
    let mut failed = Vec::new();
    for file in references.files {
        // The file being renamed may name itself.
        let path = if file.path == old {
            new.clone()
        } else {
            file.path
        };
        let shown = slashed(&relative(&base, &path)?);
        match rewrite(&path, &file.read, file.rewritten).await {
            Ok(true) => {
                activity::record(
                    &viewer,
                    format_args!(
                        "updated {} reference(s) in {}",
                        file.count,
                        activity::file(&base, &path)
                    ),
                );
                updated.push(shown);
            }
            Ok(false) => skipped.push(serde_json::json!({ "file": shown, "line": null })),
            Err(e) => {
                tracing::error!("Failed to update the references in {path}: {e}");
                failed.push(shown);
            }
        }
    }

    let shopping_list_stale = tokio::fs::read_to_string(base.join(".shopping-list"))
        .await
        .is_ok_and(|list| rename::shopping_list_names(&list, &old_relative));

    Ok(Json(serde_json::json!({
        "status": "success",
        "path": slashed(&new_relative),
        "references_updated": updated,
        "references_skipped": skipped,
        "references_failed": failed,
        "shopping_list_stale": shopping_list_stale,
    })))
}

/// Every `.cook` and `.menu` file in the collection that names `target`, with
/// its rewritten text, and the references that were found but cannot be
/// rewritten.
///
/// A file reached through a symbolic link, or lying outside the collection
/// once links are followed, is never written: its references are reported as
/// skipped. Fails when a folder or a recipe cannot be read.
fn find_references(base: &Utf8Path, target: &Target) -> io::Result<References> {
    let canonical_base = base.canonicalize_utf8()?;
    let mut found = References::default();

    for (path, linked) in recipe_files(base, &canonical_base)? {
        let Ok(relative) = path.strip_prefix(base) else {
            continue;
        };
        let read = match std::fs::read_to_string(&path) {
            Ok(read) => read,
            // Not UTF-8: no recipe either, the parser cannot read it.
            Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
            Err(_) if linked => continue,
            Err(e) => return Err(io::Error::new(e.kind(), format!("{relative}: {e}"))),
        };
        let shown = slashed(relative);
        let writer_dir = relative.parent().unwrap_or(Utf8Path::new(""));
        let rewrite = rename::rewrite_references(&read, writer_dir, target);

        for line in &rewrite.skipped_lines {
            found
                .skipped
                .push(serde_json::json!({ "file": shown, "line": line }));
        }
        let Some(rewritten) = rewrite.text else {
            continue;
        };
        let inside = path
            .canonicalize_utf8()
            .is_ok_and(|canonical| canonical.starts_with(&canonical_base));
        if is_link(&path) || !inside {
            tracing::warn!("Not updating references in {path}: it is a link or outside {base}");
            found
                .skipped
                .push(serde_json::json!({ "file": shown, "line": null }));
            continue;
        }
        found.files.push(ReferencingFile {
            path,
            read,
            rewritten,
            count: rewrite.rewritten,
        });
    }
    Ok(found)
}

/// Every `.cook` and `.menu` file under `base`, hidden names left out, sorted,
/// each with whether a symbolic link led to it.
///
/// Walked here rather than through `cooklang_find::build_tree`, which keys a
/// folder's entries by title and so drops one of two recipes sharing a title,
/// or a recipe and a folder of the same name. A folder that is a link is
/// walked when it leads outside the collection — what is found there is only
/// reported, never written — and skipped when it leads back inside, which the
/// walk reaches anyway; each real folder is walked once.
///
/// A folder of the collection that cannot be read fails the walk; one reached
/// through such a link is passed over, so that a link to a large tree cannot
/// stop every rename.
fn recipe_files(
    base: &Utf8Path,
    canonical_base: &Utf8Path,
) -> io::Result<Vec<(Utf8PathBuf, bool)>> {
    let mut files = Vec::new();
    let mut visited = HashSet::new();
    // Each folder, and whether it was reached through a link leading out.
    let mut folders = vec![(base.to_owned(), false)];

    while let Some((folder, linked)) = folders.pop() {
        let entries = folder
            .canonicalize_utf8()
            .and_then(|canonical| Ok((visited.insert(canonical), std::fs::read_dir(&folder)?)));
        let entries = match entries {
            Ok((false, _)) => continue,
            Ok((true, entries)) => entries,
            Err(e) if linked => {
                tracing::debug!("Not looking for references in {folder}: {e}");
                continue;
            }
            Err(e) => return Err(io::Error::new(e.kind(), format!("{folder}: {e}"))),
        };
        for entry in entries {
            let entry = match entry.and_then(|entry| Ok((entry.file_type()?, entry))) {
                Ok((file_type, entry)) => (file_type, entry),
                Err(_) if linked => continue,
                Err(e) => return Err(io::Error::new(e.kind(), format!("{folder}: {e}"))),
            };
            let (file_type, entry) = entry;
            // A name that is not UTF-8 can be no recipe a reference reaches.
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let path = folder.join(&name);
            if file_type.is_dir() {
                folders.push((path, linked));
            } else if file_type.is_symlink() && path.is_dir() {
                let leads_outside = path
                    .canonicalize_utf8()
                    .is_ok_and(|target| !target.starts_with(canonical_base));
                if leads_outside {
                    folders.push((path, true));
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext == "cook" || ext == "menu")
            {
                files.push((path, linked || file_type.is_symlink()));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Writes `rewritten` over `path` if it still holds `read`; `Ok(false)` when
/// it changed in the meantime and was left alone.
async fn rewrite(path: &Utf8Path, read: &str, rewritten: String) -> io::Result<bool> {
    if is_link(path) || tokio::fs::read_to_string(path).await? != read {
        return Ok(false);
    }
    fs_atomic::write_replace_async(path.to_owned(), rewritten.into_bytes()).await?;
    Ok(true)
}

/// Names of the plain files in `dir` (links included, folders not); names
/// that are not UTF-8 cannot be a recipe's picture and are left out.
async fn folder_files(dir: &Utf8Path) -> io::Result<Vec<String>> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    let mut names = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            continue;
        }
        if let Ok(name) = entry.file_name().into_string() {
            names.push(name);
        }
    }
    Ok(names)
}

/// Whether anything is at `path`, a dangling link included.
fn exists(path: &Utf8Path) -> bool {
    path.symlink_metadata().is_ok()
}

fn is_link(path: &Utf8Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
}

fn relative(base: &Utf8Path, path: &Utf8Path) -> Result<Utf8PathBuf, ApiError> {
    path.strip_prefix(base)
        .map(Utf8Path::to_owned)
        .map_err(|_| io_error(io::Error::other(format!("{path} is not under {base}"))))
}

/// `path` with `/` between its components on every platform, as the API and
/// the URLs spell paths.
fn slashed(path: &Utf8Path) -> String {
    path.components()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn not_found(path: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        json_error(format!("Recipe file not found: {path}")),
    )
}

fn bad_request(message: String) -> ApiError {
    (StatusCode::BAD_REQUEST, json_error(message))
}

fn conflict(message: String) -> ApiError {
    (StatusCode::CONFLICT, json_error(message))
}

fn io_error(e: io::Error) -> ApiError {
    tracing::error!("Rename failed: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        json_error(format!("Failed to rename: {e}")),
    )
}
