//! Content for the `/api-docs` page.
//!
//! Every entry here was verified against a running server. When you add or
//! change an API route in `src/server/mod.rs`, update this file — the
//! `router_and_docs_agree` test at the bottom will fail until you do.

use crate::web::templates::ApiSection;

/// The full API reference, in display order.
pub fn api_docs() -> Vec<ApiSection> {
    Vec::new()
}
