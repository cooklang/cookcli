use camino::Utf8Path;

/// Given an output file path like "index.html" or "recipe/Breakfast/Pancakes.html",
/// return the relative prefix that resolves from that file back to the output root.
/// Examples:
///   "index.html"                            -> "."
///   "directory/Breakfast.html"              -> ".."
///   "recipe/Breakfast/Pancakes.html"        -> "../.."
///   "menu/Sunday/Brunch.html"               -> "../.."
pub fn relative_prefix(output_relpath: &Utf8Path) -> String {
    let depth = output_relpath.components().count().saturating_sub(1); // last component is the filename
    if depth == 0 {
        ".".to_string()
    } else {
        std::iter::repeat_n("..", depth)
            .collect::<Vec<_>>()
            .join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_index() {
        assert_eq!(relative_prefix(Utf8Path::new("index.html")), ".");
    }

    #[test]
    fn one_level_deep() {
        assert_eq!(
            relative_prefix(Utf8Path::new("directory/Breakfast.html")),
            ".."
        );
    }

    #[test]
    fn two_levels_deep() {
        assert_eq!(
            relative_prefix(Utf8Path::new("recipe/Breakfast/Pancakes.html")),
            "../.."
        );
    }

    #[test]
    fn three_levels_deep() {
        assert_eq!(
            relative_prefix(Utf8Path::new("recipe/A/B/C.html")),
            "../../.."
        );
    }
}
