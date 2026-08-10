//! Input sources. Core never touches the filesystem unless handed a path.

use camino::Utf8PathBuf;

/// Where a recipe comes from.
///
/// `Content` exists so editors can pass an unsaved buffer straight in — the
/// case a path-only API cannot serve.
#[derive(Debug, Clone)]
pub enum RecipeSource {
    /// A path or bare recipe name, resolved through `cooklang-find`.
    Path(Utf8PathBuf),
    /// In-memory recipe text. `name` is used in diagnostics and titles.
    Content { text: String, name: String },
}

/// Where an aisle or pantry configuration comes from.
#[derive(Debug, Clone, Default)]
pub enum ConfigSource {
    Path(Utf8PathBuf),
    Inline(String),
    #[default]
    None,
}

impl ConfigSource {
    /// Read the configuration text, if any.
    ///
    /// Returns `Ok(None)` for `ConfigSource::None`.
    pub fn read(&self) -> Result<Option<String>, crate::CoreError> {
        match self {
            ConfigSource::None => Ok(None),
            ConfigSource::Inline(text) => Ok(Some(text.clone())),
            ConfigSource::Path(path) => {
                let text = std::fs::read_to_string(path)?;
                Ok(Some(text))
            }
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, ConfigSource::None)
    }

    /// The path this source reads from, when it is path-backed.
    pub fn path(&self) -> Option<&Utf8PathBuf> {
        match self {
            ConfigSource::Path(p) => Some(p),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_reads_as_none() {
        assert!(ConfigSource::None.read().unwrap().is_none());
        assert!(ConfigSource::None.is_none());
    }

    #[test]
    fn inline_reads_its_text() {
        let source = ConfigSource::Inline("[produce]\ntomato".to_string());
        assert_eq!(source.read().unwrap().as_deref(), Some("[produce]\ntomato"));
    }

    #[test]
    fn path_reads_from_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("aisle.conf");
        std::fs::write(&path, "[dairy]\nmilk").unwrap();
        let utf8 = camino::Utf8PathBuf::from_path_buf(path).unwrap();

        let source = ConfigSource::Path(utf8);
        assert_eq!(source.read().unwrap().as_deref(), Some("[dairy]\nmilk"));
    }

    #[test]
    fn missing_path_is_an_io_error() {
        let source = ConfigSource::Path(camino::Utf8PathBuf::from("/nonexistent/aisle.conf"));
        assert!(matches!(source.read(), Err(crate::CoreError::Io(_))));
    }
}
