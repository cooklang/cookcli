//! Resolved configuration for a set of recipe operations.

use crate::{ConfigSource, CoreError};
use camino::{Utf8Path, Utf8PathBuf};

const APP_NAME: &str = "cook";
const LOCAL_CONFIG_DIR: &str = "config";
const AUTO_AISLE: &str = "aisle.conf";
const AUTO_PANTRY: &str = "pantry.conf";

/// The configuration bundle every command operates against.
///
/// [`Context::new`] performs no filesystem access. Ambient configuration
/// discovery is opt-in through [`Context::discover`], so a caller that already
/// knows its configuration — an editor holding an unsaved buffer, say — never
/// has the user's `~/.config` read behind its back.
#[derive(Debug, Clone)]
pub struct Context {
    base_path: Utf8PathBuf,
    aisle: ConfigSource,
    pantry: ConfigSource,
}

impl Context {
    /// A context with no aisle or pantry configuration. Touches nothing.
    pub fn new(base_path: Utf8PathBuf) -> Self {
        Self {
            base_path,
            aisle: ConfigSource::None,
            pantry: ConfigSource::None,
        }
    }

    /// A context with aisle and pantry resolved using CookCLI's search order:
    /// `<base>/config/<name>`, then the platform config directory.
    ///
    /// This is the only constructor that reads ambient state, and it is
    /// explicitly opted into.
    pub fn discover(base_path: Utf8PathBuf) -> Self {
        let aisle = Self::discover_one(&base_path, AUTO_AISLE);
        let pantry = Self::discover_one(&base_path, AUTO_PANTRY);
        Self {
            base_path,
            aisle,
            pantry,
        }
    }

    fn discover_one(base_path: &Utf8Path, name: &str) -> ConfigSource {
        // A global path that cannot be determined at all is simply one fewer
        // place to look, exactly as in the CLI.
        Self::search(base_path, name, global_file_path(name).ok().as_deref())
    }

    /// The search order, with the global candidate passed in.
    ///
    /// Injecting it keeps the ordering testable without the result depending
    /// on what the machine running the tests has in its home directory.
    /// `global_file_path` supplies it in production, and computing it eagerly
    /// is harmless because it only inspects environment variables.
    fn search(base_path: &Utf8Path, name: &str, global: Option<&Utf8Path>) -> ConfigSource {
        let local = base_path.join(LOCAL_CONFIG_DIR).join(name);
        tracing::trace!("checking local config file: {local}");
        if local.is_file() {
            return ConfigSource::Path(local);
        }

        match global {
            Some(global) => {
                tracing::trace!("checking global config file: {global}");
                if global.is_file() {
                    ConfigSource::Path(global.to_owned())
                } else {
                    ConfigSource::None
                }
            }
            None => ConfigSource::None,
        }
    }

    /// Replace the aisle configuration, whatever discovery found.
    pub fn with_aisle(mut self, source: ConfigSource) -> Self {
        self.aisle = source;
        self
    }

    /// Replace the pantry configuration, whatever discovery found.
    pub fn with_pantry(mut self, source: ConfigSource) -> Self {
        self.pantry = source;
        self
    }

    /// The directory recipe paths and searches are resolved against.
    pub fn base_path(&self) -> &Utf8PathBuf {
        &self.base_path
    }

    /// The aisle configuration to categorise shopping list ingredients with.
    pub fn aisle(&self) -> &ConfigSource {
        &self.aisle
    }

    /// The pantry configuration to filter already-stocked ingredients with.
    pub fn pantry(&self) -> &ConfigSource {
        &self.pantry
    }
}

/// Resolve a global configuration file path (e.g. `~/.config/cook/{name}`).
///
/// The path is returned whether or not anything exists there.
///
/// # Errors
///
/// [`CoreError::Config`] if there is no home directory to resolve against, or
/// if the platform configuration directory is not valid UTF-8.
pub fn global_file_path(name: &str) -> Result<Utf8PathBuf, CoreError> {
    let dirs =
        directories::ProjectDirs::from("", "", APP_NAME).ok_or_else(|| CoreError::Config {
            path: Some(Utf8PathBuf::from(name)),
            message: "could not determine home directory path".to_string(),
        })?;
    let config = Utf8Path::from_path(dirs.config_dir()).ok_or_else(|| CoreError::Config {
        path: Some(Utf8PathBuf::from(name)),
        message: "cook only supports UTF-8 paths".to_string(),
    })?;
    Ok(config.join(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigSource;

    #[test]
    fn new_touches_nothing() {
        let ctx = Context::new(Utf8PathBuf::from("/nonexistent"));
        assert!(ctx.aisle().is_unset());
        assert!(ctx.pantry().is_unset());
        assert_eq!(ctx.base_path(), "/nonexistent");
    }

    #[test]
    fn with_aisle_overrides() {
        let ctx = Context::new(Utf8PathBuf::from("/tmp"))
            .with_aisle(ConfigSource::Inline("[produce]\nleek".to_string()));
        assert_eq!(
            ctx.aisle().read().unwrap().as_deref(),
            Some("[produce]\nleek")
        );
    }

    #[test]
    fn discover_finds_local_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir(&config_dir).unwrap();
        std::fs::write(config_dir.join("aisle.conf"), "[produce]\nleek").unwrap();

        let base = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let ctx = Context::discover(base.clone());

        let aisle_path = ctx.aisle().path().expect("local aisle found").clone();
        assert_eq!(aisle_path, base.join("config").join("aisle.conf"));
    }

    /// The search order is tested through `Context::search`, which takes the
    /// global directory as a parameter. Going through `discover` instead would
    /// make the result depend on whether the developer's machine happens to
    /// have `~/.config/cook/aisle.conf`.
    fn write(path: &Utf8Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn utf8(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    #[test]
    fn local_config_wins_over_global() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        let local = base.join("config").join("aisle.conf");
        let global = base.join("global").join("aisle.conf");
        write(&local, "[produce]\nleek");
        write(&global, "[dairy]\nmilk");

        let found = Context::search(&base, "aisle.conf", Some(&global));
        assert_eq!(found, ConfigSource::Path(local));
    }

    #[test]
    fn global_config_is_used_when_there_is_no_local_one() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        let global = base.join("global").join("pantry.conf");
        write(&global, "[freezer]\npeas = \"1kg\"");

        let found = Context::search(&base, "pantry.conf", Some(&global));
        assert_eq!(found, ConfigSource::Path(global));
    }

    #[test]
    fn absent_everywhere_is_unset() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        let global = base.join("global").join("pantry.conf");

        assert!(Context::search(&base, "pantry.conf", Some(&global)).is_unset());
        assert!(Context::search(&base, "pantry.conf", None).is_unset());
    }
}
