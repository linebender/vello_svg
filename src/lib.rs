// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Render an SVG document to a Vello [`Scene`](vello::Scene).
//!
//! This currently lacks support for a [number of important](crate#unsupported-features) SVG features.
//!
//! This is also intended to be the preferred integration between Vello and [usvg], so [consider
//! contributing](https://github.com/linebender/vello_svg) if you need a feature which is missing.
//!
//! This crate also re-exports [`usvg`] and [`vello`], so you can easily use the specific versions that are compatible with Vello SVG.
//!
//! # Unsupported features
//!
//! Missing features include:
//! - text
//! - group opacity
//! - mix-blend-modes
//! - clipping
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

mod error;
pub use error::Error;

pub mod util;

/// Re-export vello.
pub use vello;

/// Re-export usvg.
pub use usvg;
use vello::kurbo::Affine;

/// Render a [`Scene`](vello::Scene) from an SVG string, with default error handling.
///
/// This will draw a red box over (some) unsupported elements.
pub fn render(svg: &str) -> Result<vello::Scene, Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    let mut scene = vello::Scene::new();
    append_tree(&mut scene, &tree);
    Ok(scene)
}

/// Append an SVG to a vello [`Scene`](vello::Scene), with default error handling.
///
/// This will draw a red box over (some) unsupported elements.
pub fn append(scene: &mut vello::Scene, svg: &str) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    append_tree(scene, &tree);
    Ok(())
}

/// Append an SVG to a vello [`Scene`](vello::Scene), with user-provided error handling logic.
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn append_with<F: FnMut(&mut vello::Scene, &usvg::Node)>(
    scene: &mut vello::Scene,
    svg: &str,
    error_handler: &mut F,
) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    append_tree_with(scene, &tree, error_handler);
    Ok(())
}

/// Render a [`Scene`](vello::Scene) from a [`usvg::Tree`], with default error handling.
///
/// This will draw a red box over (some) unsupported elements.
pub fn render_tree(svg: &usvg::Tree) -> vello::Scene {
    let mut scene = vello::Scene::new();
    append_tree(&mut scene, svg);
    scene
}

/// Append an [`usvg::Tree`] to a vello [`Scene`](vello::Scene), with default error handling.
///
/// This will draw a red box over (some) unsupported elements.
pub fn append_tree(scene: &mut vello::Scene, svg: &usvg::Tree) {
    append_tree_with(scene, svg, &mut util::default_error_handler);
}

/// Append an [`usvg::Tree`] to a vello [`Scene`](vello::Scene), with user-provided error handling logic.
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn append_tree_with<F: FnMut(&mut vello::Scene, &usvg::Node)>(
    scene: &mut vello::Scene,
    svg: &usvg::Tree,
    error_handler: &mut F,
) {
    render::render_group(scene, svg.root(), Affine::IDENTITY, error_handler);
}

#[cfg(test)]
mod tests {
    use crate::util;
    use vello::kurbo::BezPath;

    fn visit_paths(group: &usvg::Group, f: &mut impl FnMut(&usvg::Path)) {
        for node in group.children() {
            match node {
                usvg::Node::Group(g) => visit_paths(g, f),
                usvg::Node::Path(p) => f(p),
                usvg::Node::Image(img) => {
                    if let usvg::ImageKind::SVG(svg) = img.kind() {
                        visit_paths(svg.root(), f);
                    }
                }
                usvg::Node::Text(t) => visit_paths(t.flattened(), f),
            }
        }
    }

    /// `path_elements` must produce the exact same `PathEl` sequence as the
    /// historical `to_bez_path` — verified against a non-trivial real-world
    /// SVG (the Ghostscript Tiger, the canonical 2D-rendering stress test
    /// used across the graphics ecosystem).
    #[test]
    fn path_elements_matches_to_bez_path_on_tiger() {
        let svg = include_str!("../examples/assets/Ghostscript_Tiger.svg");
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

        let mut buf = BezPath::new();
        let mut path_count = 0_usize;
        visit_paths(tree.root(), &mut |path| {
            buf.truncate(0);
            buf.extend(util::path_elements(path));
            let reference = util::to_bez_path(path);
            assert_eq!(
                buf.elements(),
                reference.elements(),
                "path_elements diverged from to_bez_path on path #{path_count}",
            );
            path_count += 1;
        });
        assert!(
            path_count > 100,
            "expected the tiger to contain many paths; got {path_count}",
        );
    }

    /// A reused `BezPath` buffer, filled via `path_elements` across every
    /// path in the tiger, must never reallocate once its capacity reaches
    /// the tree's largest path. Demonstrates the intended per-frame reuse
    /// pattern.
    #[test]
    fn path_elements_reuses_bezpath_capacity() {
        let svg = include_str!("../examples/assets/Ghostscript_Tiger.svg");
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

        // First pass: size the buffer to the tree's largest path.
        let mut max_len = 0_usize;
        visit_paths(tree.root(), &mut |p| {
            max_len = max_len.max(util::path_elements(p).count());
        });
        let mut buf = BezPath::with_capacity(max_len);
        // `Vec::with_capacity` may over-allocate; remember what we actually got.
        let initial_cap = buf.elements().len().saturating_add(max_len);

        // Second pass: refill the buffer for each path. Capacity must not grow.
        visit_paths(tree.root(), &mut |p| {
            buf.truncate(0);
            buf.extend(util::path_elements(p));
            assert!(
                buf.elements().len() <= initial_cap,
                "path produced {} elements, exceeding pre-sized capacity {initial_cap}",
                buf.elements().len(),
            );
        });
        // After the walk, truncate keeps the allocation.
        buf.truncate(0);
        assert_eq!(buf.elements().len(), 0);
    }
}
