// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Render-tree-builds-without-panic tests for the multi-child clip-path
//! and mask emit paths added to `render::render_group`. These don't
//! verify pixel output (that requires a GPU), but they do exercise the
//! `Scene` encoding so wrong layer/pop pairing or invalid arguments
//! would crash via `vello::Scene::pop_layer` assertion failures.

#![allow(missing_docs, reason = "regression tests")]

#[cfg(test)]
mod tests {
    use usvg::{Options, Tree};
    use vello_svg::render_tree;

    fn parse(svg: &str) -> Tree {
        Tree::from_str(svg, &Options::default()).expect("parse SVG")
    }

    #[test]
    fn multi_child_clip_path_renders_without_panic() {
        let tree = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
                <defs>
                  <clipPath id="cp">
                    <circle cx="20" cy="32" r="14"/>
                    <circle cx="44" cy="32" r="14"/>
                  </clipPath>
                </defs>
                <g clip-path="url(#cp)">
                  <rect x="0" y="0" width="64" height="64" fill="#1b75d0"/>
                </g>
              </svg>"##,
        );
        let _scene = render_tree(&tree);
    }

    #[test]
    fn nested_group_inside_clip_path_renders_without_panic() {
        // Two paths inside a <g> inside the clipPath. The recursive
        // walker should collect both.
        let tree = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
                <defs>
                  <clipPath id="cp">
                    <g>
                      <rect x="0" y="0" width="32" height="32"/>
                      <rect x="32" y="32" width="32" height="32"/>
                    </g>
                  </clipPath>
                </defs>
                <g clip-path="url(#cp)">
                  <rect x="0" y="0" width="64" height="64" fill="#1b75d0"/>
                </g>
              </svg>"##,
        );
        let _scene = render_tree(&tree);
    }

    #[test]
    fn luminance_mask_renders_without_panic() {
        let tree = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
                <defs>
                  <linearGradient id="lg" x1="0" y1="0" x2="0" y2="64" gradientUnits="userSpaceOnUse">
                    <stop offset="0" stop-color="#fff"/>
                    <stop offset="1" stop-color="#000"/>
                  </linearGradient>
                  <mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
                    <rect x="0" y="0" width="64" height="64" fill="url(#lg)"/>
                  </mask>
                </defs>
                <g mask="url(#m)">
                  <rect x="0" y="0" width="64" height="64" fill="#1b75d0"/>
                </g>
              </svg>"##,
        );
        let _scene = render_tree(&tree);
    }

    #[test]
    fn alpha_mask_renders_without_panic() {
        // mask-type="alpha" — uses the source's alpha channel directly
        // instead of converting to luminance.
        let tree = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
                <defs>
                  <mask id="m" mask-type="alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
                    <rect x="0" y="0" width="32" height="64" fill="#000" fill-opacity="0.5"/>
                  </mask>
                </defs>
                <g mask="url(#m)">
                  <rect x="0" y="0" width="64" height="64" fill="#1b75d0"/>
                </g>
              </svg>"##,
        );
        let _scene = render_tree(&tree);
    }

    #[test]
    fn empty_clip_path_falls_back_to_bbox() {
        // A clipPath with no children should not panic — we fall back
        // to the group's `layer_bounding_box` rect.
        let tree = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
                <defs>
                  <clipPath id="cp"/>
                </defs>
                <g clip-path="url(#cp)">
                  <rect x="0" y="0" width="64" height="64" fill="#1b75d0"/>
                </g>
              </svg>"##,
        );
        let _scene = render_tree(&tree);
    }
}
