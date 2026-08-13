//! Resolved configuration for a set of recipe operations.

use crate::{ConfigSource, CoreError};
use camino::{Utf8Path, Utf8PathBuf};

const APP_NAME: &str = "cook";
pub(crate) const LOCAL_CONFIG_DIR: &str = "config";
const AUTO_AISLE: &str = "aisle.conf";
pub(crate) const AUTO_PANTRY: &str = "pantry.conf";

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
    /// `<base>/config/<name>` first, then the platform configuration directory
    /// ([`global_config_path`] — `~/.config/cook/<name>` on Linux, the platform
    /// equivalent elsewhere).
    ///
    /// This is the only constructor that reads ambient state, and it is
    /// explicitly opted into.
    ///
    /// Two things to know before relying on it:
    ///
    /// - **It reports no errors.** A configuration directory that cannot be
    ///   resolved at all — no home directory, or a non-UTF-8 path — is treated
    ///   as one fewer place to look, exactly as the CLI treats it. An unset
    ///   [`ConfigSource`] therefore does not distinguish "the user has no
    ///   config file" from "this machine has no home directory". Call
    ///   [`global_config_path`] directly if you need to tell those apart.
    /// - **It stats, it does not read.** Discovery only checks that each
    ///   candidate is a file; the contents are read later by
    ///   [`ConfigSource::read`]. A `Context` held across an editing session can
    ///   therefore name a file that has since been deleted, which surfaces as a
    ///   [`CoreError::Io`] from whichever command reads it rather than from
    ///   here. Re-run `discover` if the configuration may have changed.
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
        // place to look, exactly as in the CLI. This is part of `discover`'s
        // documented contract.
        Self::search(base_path, name, global_config_path(name).ok().as_deref())
    }

    /// The search order, with the global candidate passed in.
    ///
    /// Injecting it keeps the ordering testable without the result depending
    /// on what the machine running the tests has in its home directory.
    /// `global_config_path` supplies it in production. Resolving it eagerly,
    /// where the CLI resolves it lazily, makes no observable difference: it
    /// works out the configuration directory without touching the filesystem,
    /// and the trace output is unchanged either way.
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
    ///
    /// Returned exactly as it was supplied. Unlike the CLI, which canonicalises
    /// it and rejects a non-directory before building a `Context`, core neither
    /// resolves nor validates it — so a relative path is interpreted against
    /// the *process* working directory, which for an in-process editor
    /// integration is the editor's, not the project's. Pass an absolute path
    /// unless you mean that.
    pub fn base_path(&self) -> &Utf8Path {
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

/// Resolve `name` inside the platform configuration directory for `cook`, e.g.
/// `~/.config/cook/aisle.conf` on Linux.
///
/// The path is returned whether or not anything exists there.
///
/// # Errors
///
/// [`CoreError::Config`] if there is no home directory to resolve against, or
/// if the platform configuration directory is not valid UTF-8. Both carry no
/// path, because the failure is that no path could be built.
pub fn global_config_path(name: &str) -> Result<Utf8PathBuf, CoreError> {
    let dirs =
        directories::ProjectDirs::from("", "", APP_NAME).ok_or_else(|| CoreError::Config {
            path: None,
            message: format!("could not determine the home directory to locate {name}"),
        })?;
    let config = Utf8Path::from_path(dirs.config_dir()).ok_or_else(|| CoreError::Config {
        path: None,
        message: format!(
            "the configuration directory holding {name} is not valid utf-8, \
             and cook only supports utf-8 paths"
        ),
    })?;
    Ok(config.join(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigSource;

    /// Config files are planted where discovery *would* find them, so this
    /// fails if `new` ever grows a filesystem lookup. Asserting only that the
    /// sources come back unset would pass even if `new` called `discover`.
    #[test]
    fn new_touches_nothing() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        write(&base.join("config").join("aisle.conf"), "[produce]\nleek");
        write(
            &base.join("config").join("pantry.conf"),
            "[freezer]\npeas = \"1kg\"",
        );

        let ctx = Context::new(base.clone());
        assert!(ctx.aisle().is_unset(), "new must not discover local config");
        assert!(
            ctx.pantry().is_unset(),
            "new must not discover local config"
        );
        assert_eq!(ctx.base_path(), base);
    }

    #[test]
    fn with_aisle_overrides() {
        let ctx = Context::new(Utf8PathBuf::from("/tmp"))
            .with_aisle(ConfigSource::Inline("[produce]\nleek".to_string()));
        assert_eq!(
            ctx.aisle().read().unwrap().as_deref(),
            Some("[produce]\nleek")
        );
        assert!(
            ctx.pantry().is_unset(),
            "with_aisle must not set the pantry"
        );
    }

    #[test]
    fn with_pantry_overrides() {
        let ctx = Context::new(Utf8PathBuf::from("/tmp")).with_pantry(ConfigSource::Inline(
            "[freezer]\npeas = \"1kg\"".to_string(),
        ));
        assert_eq!(
            ctx.pantry().read().unwrap().as_deref(),
            Some("[freezer]\npeas = \"1kg\"")
        );
        assert!(ctx.aisle().is_unset(), "with_pantry must not set the aisle");
    }

    #[test]
    fn discover_finds_local_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = utf8(&dir);
        write(&base.join("config").join("aisle.conf"), "[produce]\nleek");
        write(
            &base.join("config").join("pantry.conf"),
            "[freezer]\npeas = \"1kg\"",
        );

        let ctx = Context::discover(base.clone());

        assert_eq!(
            ctx.aisle().path(),
            Some(base.join("config").join("aisle.conf").as_path())
        );
        assert_eq!(
            ctx.pantry().path(),
            Some(base.join("config").join("pantry.conf").as_path())
        );
    }

    #[test]
    fn global_config_path_joins_the_app_name() {
        // Asserted as properties rather than a fixed suffix, because the shape
        // of the prefix is the platform's: `~/.config/cook/aisle.conf` on
        // Linux and `…/Application Support/cook/aisle.conf` on macOS put the
        // app name immediately before the file, but on Windows `directories`
        // yields `…\Roaming\cook\config`, so the last component before the
        // file is `config`. What holds everywhere is that the file is named
        // last, somewhere under a directory belonging to `cook`.
        let path = global_config_path("aisle.conf").expect("a home directory");
        assert_eq!(
            path.file_name(),
            Some("aisle.conf"),
            "the name asked for must be the last component: {path}"
        );
        assert!(
            path.components().any(|c| c.as_str() == APP_NAME),
            "expected a `{APP_NAME}` component in {path}"
        );
        assert!(
            path.is_absolute(),
            "the platform config directory is absolute: {path}"
        );
    }

    fn write(path: &Utf8Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn utf8(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    // The search order is exercised through `Context::search`, which takes the
    // global candidate as a parameter. Going through `discover` instead would
    // make these depend on whether the machine running them happens to have a
    // `~/.config/cook/aisle.conf`.
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
