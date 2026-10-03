//! What renaming a `.cook` or `.menu` file has to carry along: the pictures
//! named after it and the references other files make to it.
//!
//! Everything here works on names and text, so that it can be tested without a
//! server and so that the handler (`handlers::recipe_rename`) decides alone
//! what touches the disk.
//!
//! A rename stays in the file's own folder, so a reference to the file only
//! ever needs its last segment changed: `@../Sauces/Tomato{}` becomes
//! `@../Sauces/Passata{}` whatever the writer's folder is.

use camino::{Utf8Component, Utf8Path};
use cooklang::{
    parser::{Event, PullParser},
    Extensions,
};
use std::ops::Range;

/// The extensions `cooklang-find` reads a title or step picture from.
pub const PICTURE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

const RECIPE_EXTENSIONS: [&str; 2] = ["cook", "menu"];

/// Whether `picture` is a picture of the recipe whose stem is `stem`, as
/// `cooklang-find` finds them: its title picture `stem.jpg`, or a step picture
/// `stem.N.jpg` / `stem.S.N.jpg`, in any of [`PICTURE_EXTENSIONS`].
pub fn is_picture_of(picture: &str, stem: &str) -> bool {
    let Some(rest) = picture
        .strip_prefix(stem)
        .and_then(|rest| rest.strip_prefix('.'))
    else {
        return false;
    };
    let parts: Vec<&str> = rest.split('.').collect();
    let Some((ext, numbers)) = parts.split_last() else {
        return false;
    };
    PICTURE_EXTENSIONS.contains(ext)
        && numbers.len() <= 2
        && numbers
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The stems of the recipes and menus among `names`, a folder's file names,
/// leaving out `except`.
fn recipe_stems<'a>(names: &'a [String], except: &'a str) -> impl Iterator<Item = &'a str> {
    names
        .iter()
        .filter(move |name| name.as_str() != except)
        .filter_map(|name| {
            let (stem, ext) = name.rsplit_once('.')?;
            RECIPE_EXTENSIONS.contains(&ext).then_some(stem)
        })
}

/// The pictures among `names`, the file names of a folder, that belong to the
/// recipe file `file` in it and so move with it.
///
/// A picture another recipe or menu in the folder can claim stays where it is:
/// `Pasta.2.jpg` is the second step of `Pasta.cook` but also the title picture
/// of `Pasta.2.cook`, and `Pasta.jpg` is the title of `Pasta.menu` as much as
/// of `Pasta.cook`.
pub fn pictures_of(names: &[String], file: &str) -> Vec<String> {
    let Some((stem, _)) = file.rsplit_once('.') else {
        return Vec::new();
    };
    names
        .iter()
        .filter(|name| is_picture_of(name, stem))
        .filter(|name| !recipe_stems(names, file).any(|other| is_picture_of(name, other)))
        .cloned()
        .collect()
}

/// A recipe or menu in `names` other than `file` that would show the picture
/// `picture` as its own, if any. A moved picture must not land on one.
pub fn other_claimant<'a>(names: &'a [String], file: &'a str, picture: &str) -> Option<&'a str> {
    recipe_stems(names, file).find(|other| is_picture_of(picture, other))
}

/// The file a rename moves, as the references to it name it.
pub struct Target<'a> {
    /// Its folder, relative to the collection root; empty at the root.
    pub dir: &'a Utf8Path,
    pub old_stem: &'a str,
    pub new_stem: &'a str,
    /// `cook` or `menu`.
    pub ext: &'a str,
    /// The file is a menu with a `.cook` of the same name beside it, which an
    /// extensionless reference reaches first: `@./Plan{}` is not this file.
    pub shadowed: bool,
    /// An extensionless reference to the new name would not reach the file —
    /// the new stem holds a `.`, so it would be taken as a file name with an
    /// extension, or the file is a menu with a `.cook` of the new name beside
    /// it — so rewritten references spell the extension out.
    pub spell_extension: bool,
}

/// What [`rewrite_references`] made of one file.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rewrite {
    /// The file's new text, or `None` when nothing in it changes.
    pub text: Option<String>,
    /// How many references were rewritten.
    pub rewritten: usize,
    /// The lines of references to the file that were left alone because they
    /// cannot be rewritten exactly: written with an escape, a comment or a
    /// line break inside the name, or spelled in another letter case.
    pub skipped_lines: Vec<usize>,
}

enum Match {
    No,
    /// It names the file exactly; `true` when it spells the extension.
    Exactly(bool),
    /// It names the file only once letter case is ignored, which reaches it on
    /// some file systems and not on others.
    Loosely,
}

/// Rewrites the references to `target` in `source`, the text of a recipe or
/// menu in the folder `writer_dir` (relative to the collection root).
///
/// The text is read the way `cook` reads it — the parser with no extensions —
/// and a reference is resolved with [`cookcli_core::resolve_reference`] from
/// the writer's folder, so only what really names the file is changed. Only
/// the bytes of the name's last segment are replaced; every other byte of the
/// text stays as it was.
pub fn rewrite_references(source: &str, writer_dir: &Utf8Path, target: &Target) -> Rewrite {
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut skipped_lines = Vec::new();

    for event in PullParser::new(source, Extensions::empty()) {
        let Event::Ingredient(ingredient) = event else {
            continue;
        };
        let name = &ingredient.name;
        let reference = name.text_trimmed();
        if !is_reference(&reference) {
            continue;
        }

        let span = name.span();
        let line = source[..span.start()].matches('\n').count() + 1;
        let spells_extension = match matches(&reference, writer_dir, target) {
            Match::No => continue,
            Match::Loosely => {
                skipped_lines.push(line);
                continue;
            }
            Match::Exactly(spells_extension) => spells_extension,
        };

        // The name's bytes must be exactly its text, or replacing them would
        // also drop whatever the parser skipped over (an escape, a comment) or
        // join what it split.
        let raw = &source[span.range()];
        if name.fragments().len() != 1 || raw != name.text() || raw.trim() != reference {
            skipped_lines.push(line);
            continue;
        }

        let start = span.start() + (raw.len() - raw.trim_start().len());
        let written = raw.trim();
        let last_segment = written.rfind(['/', '\\']).map_or(0, |i| i + 1);
        // The last segment has to be the name itself. `./Pasta/.` and
        // `./x/../Pasta/..` resolve to the file too, but their last segment is
        // not its name, and replacing it would point somewhere else.
        let spelled = if spells_extension {
            format!("{}.{}", target.old_stem, target.ext)
        } else {
            target.old_stem.to_string()
        };
        if written[last_segment..] != spelled {
            skipped_lines.push(line);
            continue;
        }
        let replacement = if spells_extension || target.spell_extension {
            format!("{}.{}", target.new_stem, target.ext)
        } else {
            target.new_stem.to_string()
        };
        edits.push((start + last_segment..start + written.len(), replacement));
    }

    let rewritten = edits.len();
    let text = (!edits.is_empty()).then(|| {
        let mut text = String::with_capacity(source.len());
        let mut copied = 0;
        for (range, replacement) in edits {
            text.push_str(&source[copied..range.start]);
            text.push_str(&replacement);
            copied = range.end;
        }
        text.push_str(&source[copied..]);
        text
    });

    Rewrite {
        text,
        rewritten,
        skipped_lines,
    }
}

/// Whether an ingredient name is a recipe reference, by the parser's own rule
/// (`cooklang`'s private `parse_reference`).
fn is_reference(name: &str) -> bool {
    ["./", "../", ".\\", "..\\"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
        && !name.ends_with(['/', '\\'])
}

fn matches(reference: &str, writer_dir: &Utf8Path, target: &Target) -> Match {
    let Some(resolved) = cookcli_core::resolve_reference(writer_dir, reference) else {
        return Match::No;
    };

    // A name with any extension is looked up as it is; one without, as
    // `.cook` and then `.menu`.
    let (named, spells_extension) = match resolved.extension() {
        Some(_) => (format!("{}.{}", target.old_stem, target.ext), true),
        None if target.shadowed => return Match::No,
        None => (target.old_stem.to_string(), false),
    };
    let expected = target.dir.join(named);

    if same_path(&resolved, &expected, |a, b| a == b) {
        Match::Exactly(spells_extension)
    } else if same_path(&resolved, &expected, |a, b| {
        a.to_lowercase() == b.to_lowercase()
    }) {
        Match::Loosely
    } else {
        Match::No
    }
}

/// Compares two relative paths component by component, so `/` and `\` (both
/// separators on Windows) cannot make the same path look different.
fn same_path(a: &Utf8Path, b: &Utf8Path, eq: impl Fn(&str, &str) -> bool) -> bool {
    let names = |path: &Utf8Path| -> Vec<String> {
        path.components()
            .filter(|c| !matches!(c, Utf8Component::CurDir))
            .map(|c| c.as_str().to_string())
            .collect()
    };
    let (a, b) = (names(a), names(b));
    a.len() == b.len() && a.iter().zip(&b).all(|(a, b)| eq(a, b))
}

/// Whether the saved shopping list, `list` (the text of `.shopping-list`),
/// still names the file at `relative` (`Sub/Pasta.cook`) — with or without
/// its extension, as the top-level entry or inside a menu entry. Unreadable
/// text names nothing.
pub fn shopping_list_names(list: &str, relative: &Utf8Path) -> bool {
    use cooklang::shopping_list::ShoppingListItem;

    fn any(items: &[ShoppingListItem], names: &dyn Fn(&str) -> bool) -> bool {
        items.iter().any(|item| match item {
            ShoppingListItem::Recipe(recipe) => names(&recipe.path) || any(&recipe.children, names),
            _ => false,
        })
    }

    let Ok(list) = cooklang::shopping_list::parse(list) else {
        return false;
    };
    let stem = relative.with_extension("");
    let names = |path: &str| {
        let path = Utf8Path::new(path.trim_start_matches("./"));
        let path = path.as_str().replace('\\', "/");
        let path = Utf8Path::new(&path);
        same_path(path, relative, |a, b| a == b) || same_path(path, &stem, |a, b| a == b)
    };
    any(&list.items, &names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn title_and_step_pictures_are_the_recipes() {
        for picture in [
            "Pasta.jpg",
            "Pasta.jpeg",
            "Pasta.png",
            "Pasta.webp",
            "Pasta.3.jpg",
            "Pasta.2.4.png",
        ] {
            assert!(is_picture_of(picture, "Pasta"), "{picture}");
        }
        for picture in [
            "Pasta.cook",
            "Pasta.gif",
            "Pasta.x.jpg",
            "Pasta..jpg",
            "Pasta.1.2.3.jpg",
            "Pastas.jpg",
            "Pasta salad.jpg",
            "pasta.jpg",
            "Pasta.jpg.bak",
        ] {
            assert!(!is_picture_of(picture, "Pasta"), "{picture}");
        }
    }

    #[test]
    fn pictures_another_recipe_can_claim_stay() {
        let folder = names(&[
            "Pasta.cook",
            "Pasta.jpg",
            "Pasta.1.jpg",
            "Pasta.2.jpg",
            "Pasta.2.cook",
            "Pasta.3.1.png",
            "Pasta salad.cook",
            "Pasta salad.jpg",
            "notes.txt",
        ]);
        let mut moved = pictures_of(&folder, "Pasta.cook");
        moved.sort();
        assert_eq!(moved, ["Pasta.1.jpg", "Pasta.3.1.png", "Pasta.jpg"]);

        // A menu of the same name shares the title picture.
        let shared = names(&["Plan.cook", "Plan.menu", "Plan.jpg", "Plan.1.jpg"]);
        assert!(pictures_of(&shared, "Plan.menu").is_empty());
    }

    #[test]
    fn a_moved_picture_must_not_become_another_recipes() {
        let folder = names(&["Pasta.cook", "Soup.cook", "Soup.2.cook"]);
        // Pasta.2.jpg renamed to Soup.2.jpg would be Soup.2.cook's title.
        assert_eq!(
            other_claimant(&folder, "Soup.cook", "Soup.2.jpg"),
            Some("Soup.2")
        );
        assert_eq!(other_claimant(&folder, "Soup.cook", "Soup.jpg"), None);
        assert_eq!(other_claimant(&folder, "Soup.cook", "Soup.1.jpg"), None);

        // A menu renamed beside a recipe of its new name would give it the
        // menu's title picture.
        let beside = names(&["Week.cook", "Week.menu"]);
        assert_eq!(
            other_claimant(&beside, "Week.menu", "Week.jpg"),
            Some("Week")
        );
    }

    fn target<'a>(dir: &'a str, old: &'a str, new: &'a str) -> Target<'a> {
        Target {
            dir: Utf8Path::new(dir),
            old_stem: old,
            new_stem: new,
            ext: "cook",
            shadowed: false,
            spell_extension: false,
        }
    }

    fn rewrite(source: &str, writer_dir: &str, target: &Target) -> Rewrite {
        rewrite_references(source, Utf8Path::new(writer_dir), target)
    }

    fn rewritten(source: &str, writer_dir: &str, target: &Target) -> String {
        rewrite(source, writer_dir, target)
            .text
            .unwrap_or_else(|| panic!("nothing rewritten in {source:?}"))
    }

    #[test]
    fn references_are_rewritten_in_their_last_segment_only() {
        let root = target("", "Pasta", "Noodles");
        assert_eq!(
            rewritten("Cook @./Pasta{}.\n", "", &root),
            "Cook @./Noodles{}.\n"
        );
        assert_eq!(
            rewritten("Cook @./Pasta{2%servings}(al dente).\n", "Sub", &root),
            "Cook @./Noodles{2%servings}(al dente).\n"
        );
        assert_eq!(
            rewritten("Cook @../Pasta{}.\n", "Sub", &root),
            "Cook @../Noodles{}.\n"
        );
        assert_eq!(
            rewritten("Cook @./Pasta.cook{}.\n", "", &root),
            "Cook @./Noodles.cook{}.\n"
        );

        let nested = target("Italian/Pasta", "Carbonara", "Gricia");
        assert_eq!(
            rewritten("@./Italian/Pasta/Carbonara{}\n", "Menus", &nested),
            "@./Italian/Pasta/Gricia{}\n"
        );
        assert_eq!(
            rewritten("@../Pasta/Carbonara{}\n", "Italian/Soups", &nested),
            "@../Pasta/Gricia{}\n"
        );
    }

    #[test]
    fn everything_but_the_references_is_kept_byte_for_byte() {
        let source = "---\ntitle: Week\nsource: ./Pasta\n---\n\n\
                      == Monday ==\r\n\r\nDinner:\r\n- @./Pasta{1}\r\n-- @./Pasta{} in a comment\r\n\
                      [- @./Pasta{} in a block comment -]\r\nPasta is @pasta{100%g} not ./Pasta.\r\n\
                      - @./Pasta{2}  \r\n";
        let expected = source
            .replace("- @./Pasta{1}", "- @./Noodles{1}")
            .replace("- @./Pasta{2}", "- @./Noodles{2}");
        let result = rewrite(source, "", &target("", "Pasta", "Noodles"));
        assert_eq!(result.text.as_deref(), Some(expected.as_str()));
        assert_eq!(result.rewritten, 2);
        assert!(result.skipped_lines.is_empty());
    }

    #[test]
    fn references_to_other_files_are_left_alone() {
        let root = target("", "Pasta", "Noodles");
        for source in [
            "@./Pastas{}",
            "@./Pasta salad{}",
            "@./Sub/Pasta{}",
            "@./Pasta.menu{}",
            "@./Pasta.txt{}",
            "@Pasta{}",
            "@pasta",
            "#./Pasta{}",
            "@/Pasta{}",
            "@./{}",
        ] {
            assert_eq!(rewrite(source, "", &root), Rewrite::default(), "{source}");
        }

        // `..` is read from the writer's folder, and only then.
        assert_eq!(
            rewrite("@../Pasta{}", "", &root),
            Rewrite::default(),
            "climbs out of the collection"
        );
        let nested = target("Sub", "Pasta", "Noodles");
        assert_eq!(
            rewrite("@./Pasta{}", "Sub", &nested),
            Rewrite::default(),
            "./ is the collection root, not the writer's folder"
        );
        assert_eq!(
            rewrite("@../Pasta{}", "Sub", &nested),
            Rewrite::default(),
            "../ from Sub is the root"
        );
        assert_eq!(
            rewritten("@../Sub/Pasta{}", "Other", &nested),
            "@../Sub/Noodles{}"
        );
    }

    #[test]
    fn a_menu_behind_a_recipe_of_the_same_name_is_only_reached_with_its_extension() {
        let menu = Target {
            ext: "menu",
            shadowed: true,
            ..target("", "Plan", "Week")
        };
        assert_eq!(rewrite("@./Plan{}", "", &menu), Rewrite::default());
        assert_eq!(rewritten("@./Plan.menu{}", "", &menu), "@./Week.menu{}");

        let alone = Target {
            ext: "menu",
            ..target("", "Plan", "Week")
        };
        assert_eq!(rewritten("@./Plan{}", "", &alone), "@./Week{}");
    }

    #[test]
    fn the_extension_is_spelled_out_where_a_bare_name_would_miss() {
        let dotted = Target {
            spell_extension: true,
            ..target("", "Pasta", "Pasta v2.0")
        };
        assert_eq!(rewritten("@./Pasta{}", "", &dotted), "@./Pasta v2.0.cook{}");
        assert_eq!(
            rewritten("@./Pasta.cook{}", "", &dotted),
            "@./Pasta v2.0.cook{}"
        );
    }

    #[test]
    fn references_that_cannot_be_rewritten_exactly_are_reported() {
        let root = target("", "Red Beans", "Black Beans");
        let result = rewrite(
            "A @./Red  Beans{}.\nB @./Red Beans{}.\nC @./red beans{}.\n",
            "",
            &root,
        );
        assert_eq!(
            result.text.as_deref(),
            Some("A @./Red  Beans{}.\nB @./Black Beans{}.\nC @./red beans{}.\n")
        );
        assert_eq!(result.rewritten, 1);
        assert_eq!(result.skipped_lines, [1, 3]);

        // `\` escapes the next character, so a backslash separator is written
        // `\\` and the parser splits the name there.
        let pasta = target("", "Pasta", "Noodles");
        for source in [
            "@./Pas\\ta{}",
            "@.\\\\Pasta{}",
            "@./Pas[- x -]ta{}",
            // They resolve to Pasta, but their last segment is not its name.
            "@./Pasta/.{}",
            "@./Pasta/x/..{}",
            "@./Other/../Pasta/.{}",
        ] {
            let escaped = rewrite(source, "", &pasta);
            assert_eq!(escaped.text, None, "{source}");
            assert_eq!(escaped.skipped_lines, [1], "{source}");
        }
    }

    #[test]
    fn the_shopping_list_is_searched_for_the_old_path() {
        let path = Utf8Path::new("Sub/Pasta.cook");
        assert!(shopping_list_names("./Sub/Pasta{2}\n", path));
        assert!(shopping_list_names("./Sub/Pasta.cook\n", path));
        assert!(shopping_list_names(
            "./Week.menu\n  ./Sub/Pasta{1}\n  ./Soup\n",
            path
        ));
        assert!(!shopping_list_names("./Sub/Pasta salad\n./Pasta\n", path));
        assert!(!shopping_list_names("", path));
    }
}
