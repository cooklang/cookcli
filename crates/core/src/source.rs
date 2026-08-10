//! Input sources. Core never touches the filesystem unless handed a path.

use crate::CoreError;
use camino::Utf8PathBuf;

/// Where a recipe comes from.
///
/// `Content` exists so editors can pass an unsaved buffer straight in — the
/// case a path-only API cannot serve.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipeSource {
    /// A path or bare recipe name, resolved through `cooklang-find`.
    Path(Utf8PathBuf),
    /// In-memory recipe text.
    Content {
        /// The recipe source text.
        text: String,
        /// The name to use in diagnostics and titles.
        name: String,
    },
}

/// Where an aisle or pantry configuration comes from.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConfigSource {
    /// Read the configuration from this file.
    Path(Utf8PathBuf),
    /// Use this configuration text directly.
    Inline(String),
    /// No configuration. Commands fall back to their unconfigured behaviour.
    #[default]
    None,
}

impl ConfigSource {
    /// Read the configuration text, if any.
    ///
    /// Returns `Ok(None)` for [`ConfigSource::None`].
    ///
    /// # Errors
    ///
    /// [`CoreError::Io`] if a path-backed source cannot be read.
    pub fn read(&self) -> Result<Option<String>, CoreError> {
        match self {
            ConfigSource::None => Ok(None),
            ConfigSource::Inline(text) => Ok(Some(text.clone())),
            ConfigSource::Path(path) => match std::fs::read_to_string(path) {
                Ok(text) => Ok(Some(text)),
                Err(source) => Err(CoreError::Io {
                    path: path.clone(),
                    source,
                }),
            },
        }
    }

    /// True when this source carries no configuration at all.
    pub fn is_unset(&self) -> bool {
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
        assert!(ConfigSource::None.is_unset());
    }

    #[test]
    fn inline_reads_its_text() {
        let source = ConfigSource::Inline("[produce]\ntomato".to_string());
        assert_eq!(source.read().unwrap().as_deref(), Some("[produce]\ntomato"));
        assert!(!source.is_unset());
        assert_eq!(source.path(), None);
    }

    #[test]
    fn path_reads_from_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("aisle.conf");
        std::fs::write(&path, "[dairy]\nmilk").unwrap();
        let utf8 = camino::Utf8PathBuf::from_path_buf(path).unwrap();

        let source = ConfigSource::Path(utf8.clone());
        assert_eq!(source.read().unwrap().as_deref(), Some("[dairy]\nmilk"));
        assert_eq!(source.path(), Some(&utf8));
    }

    #[test]
    fn missing_path_is_an_io_error_naming_the_path() {
        let missing = camino::Utf8PathBuf::from("/nonexistent/aisle.conf");
        let source = ConfigSource::Path(missing.clone());

        match source.read() {
            Err(CoreError::Io { path, source }) => {
                assert_eq!(path, missing);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected CoreError::Io, got {other:?}"),
        }
    }
}
