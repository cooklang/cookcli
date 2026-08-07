//! Content for the `/api-docs` page.
//!
//! Verify each entry against a running server before adding it — the examples
//! here are meant to be real payloads, not plausible ones.
//!
//! When you add or change an API route in `src/server/mod.rs`, update this
//! file: the `every_router_path_is_documented` test fails until the new route
//! has an entry, and `no_documented_path_is_stale` fails if an entry names a
//! route that no longer exists.

use crate::web::templates::ApiSection;

/// The full API reference, in display order.
pub fn sections() -> Vec<ApiSection> {
    Vec::new()
}
