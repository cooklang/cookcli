// Re-export modules for testing

// Commands - make them available as public modules
pub mod build;
pub mod doctor;
#[cfg(feature = "import")]
pub mod import;
#[cfg(feature = "sync")]
pub mod login;
#[cfg(feature = "sync")]
pub mod logout;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod pantry;
pub mod recipe;
pub mod report;
pub mod search;
pub mod seed;
#[cfg(feature = "server")]
pub mod server;
pub mod shopping_list;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "self-update")]
pub mod update;
pub mod web;

// Other modules
pub mod args;
pub mod util;

/// The one `Context` definition, shared with every other consumer of the
/// library.
///
/// The binary and this library are two crate roots over the same sources, so
/// each used to carry its own copy — and they had drifted apart, one searching
/// the platform configuration directory and one not
/// (<https://github.com/cooklang/cookcli/issues/417>). Re-exporting core's
/// leaves nowhere for them to drift.
pub use cookcli_core::Context;

/// The core library, re-exported so a consumer of `cookcli` can name its types
/// without adding a `cookcli-core` dependency that could resolve to a
/// different version.
pub use cookcli_core;
