// MIT License
//
// Copyright (c) 2024 cooklang
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Recipe rendering: Askama templates, view-model builders, i18n and the
//! embedded static assets.
//!
//! Shared by `cook server` (renders them over HTTP) and `cook build` (renders
//! them to a static site), so this layer stays compiled even when the `server`
//! feature is off.

use rust_embed::RustEmbed;

pub mod builders;
mod i18n;
pub mod language;
pub mod menus;
mod nutrition;
pub mod templates;

/// API reference content for the `/api-docs` page. Server-only.
#[cfg(feature = "server")]
pub mod api_docs;

/// The same reference, rendered to the Markdown checked in as `docs/api.md`.
///
/// Only `tests/api_docs_md_test.rs` calls this. The `cook` binary declares its
/// own copy of this module tree and never reaches the renderer, so without the
/// allow every helper in here is dead code in that build.
#[cfg(feature = "server")]
#[allow(dead_code)]
pub mod api_docs_md;

/// Static assets (CSS, JS, icons) embedded into the binary at compile time.
#[derive(RustEmbed)]
#[folder = "static/"]
pub struct StaticFiles;
