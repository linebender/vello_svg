// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Render an SVG document to any backend implementing [`RenderSink`].
//!
//! Vello support is enabled by default. Raster images require a custom
//! [`RenderSink::draw_image`] implementation.
//!
//! This currently lacks support for a [number of important](crate#unsupported-features) SVG features.
//!
//! This is also intended to be the preferred integration between Vello and [usvg], so [consider
//! contributing](https://github.com/linebender/vello_svg) if you need a feature which is missing.
//!
//! ## Usage
//!
//! ```
//! # #[cfg(feature = "vello")] {
//! let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
//!     <circle cx="50" cy="50" r="40" fill="red"/>
//! </svg>"#;
//! let scene = vello_svg::render(svg).expect("valid SVG");
//! # }
//! ```
//!
//! Use [`append`] or [`append_tree`] to render into an existing scene or custom sink.
//!
//! # Unsupported features
//!
//! Unsupported or incomplete features include:
//! - clipping beyond a single path
//! - masking
//! - filter effects
//! - group background
//! - path shape-rendering
//! - patterns

// LINEBENDER LINT SET - lib.rs - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// END LINEBENDER LINT SET
// The following lints are part of the Linebender standard set,
// but resolving them has been deferred for now.
// Feel free to send a PR that solves one or more of these.
#![allow(
    missing_docs,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    reason = "Deferred"
)]
#![cfg_attr(test, allow(unused_crate_dependencies, reason = "Deferred"))] // Some dev dependencies are only used in tests

mod render;
pub use render::RenderSink;
#[cfg(feature = "vello")]
pub use render::{render, render_tree};

mod error;
pub use error::Error;

pub mod util;

/// Re-export `vello`.
#[cfg(feature = "vello")]
pub use vello;

pub use kurbo;
pub use peniko;

/// Re-export usvg.
pub use usvg;

use kurbo::Affine;

/// Append an SVG to a [`RenderSink`], with default error handling.
///
/// This will draw a red box over (some) unsupported elements.
pub fn append(scene: &mut impl RenderSink, svg: &str) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    append_tree(scene, &tree);
    Ok(())
}

/// Append an SVG to a [`RenderSink`], with user-provided error handling logic.
///
/// See the [unsupported features](crate#unsupported-features).
pub fn append_with<S: RenderSink, F: FnMut(&mut S, &usvg::Node)>(
    scene: &mut S,
    svg: &str,
    error_handler: &mut F,
) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    append_tree_with(scene, &tree, error_handler);
    Ok(())
}

/// Append an [`usvg::Tree`] to a [`RenderSink`], with default error handling.
///
/// This will draw a red box over (some) unsupported elements.
pub fn append_tree(scene: &mut impl RenderSink, svg: &usvg::Tree) {
    append_tree_with_transform(scene, svg, Affine::IDENTITY);
}

/// Append an [`usvg::Tree`] to a [`RenderSink`] with a base transform applied to all elements.
pub fn append_tree_with_transform(
    scene: &mut impl RenderSink,
    svg: &usvg::Tree,
    transform: Affine,
) {
    render::render_group(
        scene,
        svg.root(),
        transform,
        &mut util::default_error_handler_with_transform,
    );
}

/// Append an [`usvg::Tree`] to a [`RenderSink`], with user-provided error handling logic.
///
/// See the [unsupported features](crate#unsupported-features).
pub fn append_tree_with<S: RenderSink, F: FnMut(&mut S, &usvg::Node)>(
    scene: &mut S,
    svg: &usvg::Tree,
    error_handler: &mut F,
) {
    render::render_group(
        scene,
        svg.root(),
        Affine::IDENTITY,
        &mut |scene, node, _| {
            error_handler(scene, node);
        },
    );
}
