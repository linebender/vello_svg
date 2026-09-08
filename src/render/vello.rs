// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::RenderSink;
use crate::{Error, append_tree};
use kurbo::{Affine, Shape, Stroke};
use peniko::{BlendMode, Brush, Fill};

impl RenderSink for vello::Scene {
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    ) {
        self.push_layer(Fill::NonZero, blend, alpha, transform, shape);
    }

    fn pop_layer(&mut self) {
        self.pop_layer();
    }

    fn fill(
        &mut self,
        fill: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        self.fill(fill, transform, brush, Some(brush_transform), shape);
    }

    fn stroke(
        &mut self,
        stroke: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        self.stroke(stroke, transform, brush, Some(brush_transform), shape);
    }
}

/// Render an SVG string to a new Vello scene.
///
/// Raster images are skipped. This will draw a red box over (some) other
/// unsupported elements.
pub fn render(svg: &str) -> Result<vello::Scene, Error> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())?;
    Ok(render_tree(&tree))
}

/// Render a [`usvg::Tree`] to a new Vello scene.
///
/// Raster images are skipped. This will draw a red box over (some) other
/// unsupported elements.
pub fn render_tree(svg: &usvg::Tree) -> vello::Scene {
    let mut scene = vello::Scene::new();
    append_tree(&mut scene, svg);
    scene
}
