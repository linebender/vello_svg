// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::util;
use kurbo::{Affine, Rect, Shape, Stroke};
use peniko::{BlendMode, Brush, Fill, Mix};

#[cfg(feature = "vello")]
mod vello;
#[cfg(feature = "vello")]
pub use vello::{render, render_tree};

/// A rendering backend for SVG documents.
///
/// Brush transforms are relative to the shape, before `transform` is applied.
pub trait RenderSink {
    /// Push a compositing layer clipped to `shape`. Paired with [`Self::pop_layer`].
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    );

    fn pop_layer(&mut self);

    fn fill(
        &mut self,
        fill: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    );

    fn stroke(
        &mut self,
        stroke: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    );

    fn draw_image(&mut self, _image: &usvg::Image, _transform: Affine) {}

    /// Called before rendering an SVG group, including the document root.
    fn begin_group(&mut self, _group: &usvg::Group) {}

    /// Called after rendering an SVG group and popping its child layers.
    fn end_group(&mut self) {}
}

pub(crate) fn render_group<S: RenderSink, F: FnMut(&mut S, &usvg::Node, Affine)>(
    scene: &mut S,
    group: &usvg::Group,
    base_transform: Affine,
    error_handler: &mut F,
) {
    scene.begin_group(group);
    for node in group.children() {
        let transform = base_transform * util::to_affine(&node.abs_transform());
        match node {
            usvg::Node::Group(g) => {
                let alpha = g.opacity().get();
                let blend_mode: BlendMode = match g.blend_mode() {
                    usvg::BlendMode::Normal => Mix::Normal.into(),
                    usvg::BlendMode::Multiply => Mix::Multiply.into(),
                    usvg::BlendMode::Screen => Mix::Screen.into(),
                    usvg::BlendMode::Overlay => Mix::Overlay.into(),
                    usvg::BlendMode::Darken => Mix::Darken.into(),
                    usvg::BlendMode::Lighten => Mix::Lighten.into(),
                    usvg::BlendMode::ColorDodge => Mix::ColorDodge.into(),
                    usvg::BlendMode::ColorBurn => Mix::ColorBurn.into(),
                    usvg::BlendMode::HardLight => Mix::HardLight.into(),
                    usvg::BlendMode::SoftLight => Mix::SoftLight.into(),
                    usvg::BlendMode::Difference => Mix::Difference.into(),
                    usvg::BlendMode::Exclusion => Mix::Exclusion.into(),
                    usvg::BlendMode::Hue => Mix::Hue.into(),
                    usvg::BlendMode::Saturation => Mix::Saturation.into(),
                    usvg::BlendMode::Color => Mix::Color.into(),
                    usvg::BlendMode::Luminosity => Mix::Luminosity.into(),
                };

                match g
                    .clip_path()
                    // support clip-path with a single path
                    .and_then(|path| path.root().children().first())
                {
                    Some(usvg::Node::Path(clip_path)) => {
                        let local_path = util::to_bez_path(clip_path);
                        scene.push_layer(blend_mode, alpha, transform, &local_path);
                    }
                    _ => {
                        // Use bounding box as the clip path.
                        let bounding_box = g.layer_bounding_box();
                        let rect = Rect::from_origin_size(
                            (bounding_box.x(), bounding_box.y()),
                            (bounding_box.width() as f64, bounding_box.height() as f64),
                        );
                        scene.push_layer(blend_mode, alpha, transform, &rect);
                    }
                };

                // usvg's absolute transforms already include the ancestor groups.
                render_group(scene, g, base_transform, error_handler);

                scene.pop_layer();
            }
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }
                let local_path = util::to_bez_path(path);

                let do_fill = |scene: &mut S, error_handler: &mut F| {
                    if let Some(fill) = &path.fill() {
                        if let Some((brush, brush_transform)) =
                            util::to_brush(fill.paint(), fill.opacity())
                        {
                            scene.fill(
                                match fill.rule() {
                                    usvg::FillRule::NonZero => Fill::NonZero,
                                    usvg::FillRule::EvenOdd => Fill::EvenOdd,
                                },
                                transform,
                                &brush,
                                brush_transform,
                                &local_path,
                            );
                        } else {
                            error_handler(scene, node, transform);
                        }
                    }
                };
                let do_stroke = |scene: &mut S, error_handler: &mut F| {
                    if let Some(stroke) = &path.stroke() {
                        if let Some((brush, brush_transform)) =
                            util::to_brush(stroke.paint(), stroke.opacity())
                        {
                            let conv_stroke = util::to_stroke(stroke);
                            scene.stroke(
                                &conv_stroke,
                                transform,
                                &brush,
                                brush_transform,
                                &local_path,
                            );
                        } else {
                            error_handler(scene, node, transform);
                        }
                    }
                };
                match path.paint_order() {
                    usvg::PaintOrder::FillAndStroke => {
                        do_fill(scene, error_handler);
                        do_stroke(scene, error_handler);
                    }
                    usvg::PaintOrder::StrokeAndFill => {
                        do_stroke(scene, error_handler);
                        do_fill(scene, error_handler);
                    }
                }
            }
            usvg::Node::Image(img) => {
                if !img.is_visible() {
                    continue;
                }
                match img.kind() {
                    usvg::ImageKind::JPEG(_)
                    | usvg::ImageKind::PNG(_)
                    | usvg::ImageKind::GIF(_)
                    | usvg::ImageKind::WEBP(_) => {
                        scene.draw_image(img, transform);
                    }
                    usvg::ImageKind::SVG(svg) => {
                        render_group(scene, svg.root(), transform, error_handler);
                    }
                }
            }
            usvg::Node::Text(text) => {
                render_group(scene, text.flattened(), transform, error_handler);
            }
        }
    }
    scene.end_group();
}
