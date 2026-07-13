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
pub mod templates;

/// Static assets (CSS, JS, icons) embedded into the binary at compile time.
#[derive(RustEmbed)]
#[folder = "static/"]
pub struct StaticFiles;
