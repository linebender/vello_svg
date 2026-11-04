// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::util;
use kurbo::{Affine, BezPath, Point, Rect, Shape, Stroke};
use peniko::{BlendMode, Brush, Color, Compose, Fill, Mix};

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

                let mut mask = None;
                let pushed_layer = if let Some(clip_path) = g.clip_path() {
                    // The clip path's shapes, in the group's coordinates. usvg puts a
                    // shape that has a transform into a group of its own.
                    let mut shapes = Vec::new();
                    collect_clip(
                        clip_path.root(),
                        util::to_affine(&clip_path.transform()),
                        &mut shapes,
                    );
                    match shapes.as_slice() {
                        [] => {
                            // An empty clip path hides the whole group.
                            continue;
                        }
                        [(clip, Fill::NonZero)] => {
                            if clips_nothing(clip, g) {
                                // CAD exports often put a page-sized rectangle clip around
                                // every element. Each is a layer, and thousands of large
                                // layers can exhaust the renderer's memory, so leave these
                                // out.
                                if needs_layer(g) {
                                    push_bounding_box_layer(scene, g, blend_mode, alpha, transform);
                                    true
                                } else {
                                    false
                                }
                            } else {
                                scene.push_layer(blend_mode, alpha, transform, clip);
                                true
                            }
                        }
                        _ if merging_unites(&shapes) => {
                            let mut clip = BezPath::new();
                            for (shape, _) in &shapes {
                                clip.extend(shape.elements().iter().copied());
                            }
                            scene.push_layer(blend_mode, alpha, transform, &clip);
                            true
                        }
                        _ => {
                            // SVG unites the shapes of a clip path, each with its own clip
                            // rule. One non-zero path would leave holes where oppositely
                            // wound shapes overlap, so draw the shapes into a mask instead,
                            // after the content.
                            let bounds = shapes
                                .iter()
                                .map(|(shape, _)| shape.bounding_box())
                                .reduce(|a, b| a.union(b))
                                .unwrap_or_default();
                            scene.push_layer(blend_mode, alpha, transform, &bounds);
                            mask = Some((shapes, bounds));
                            true
                        }
                    }
                } else if needs_layer(g) {
                    push_bounding_box_layer(scene, g, blend_mode, alpha, transform);
                    true
                } else {
                    // A plain group needs no layer. Clipping it to its bounding box would
                    // cut off the outer half of strokes lying on the edge of that box.
                    false
                };

                // usvg's absolute transforms already include the ancestor groups.
                render_group(scene, g, base_transform, error_handler);

                if let Some((shapes, bounds)) = mask {
                    // Keep the content only where a clip shape covers it.
                    scene.push_layer(
                        BlendMode::new(Mix::Normal, Compose::DestIn),
                        1.0,
                        transform,
                        &bounds,
                    );
                    let brush = Brush::Solid(Color::BLACK);
                    for (shape, rule) in &shapes {
                        scene.fill(*rule, transform, &brush, Affine::IDENTITY, shape);
                    }
                    scene.pop_layer();
                }
                if pushed_layer {
                    scene.pop_layer();
                }
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
                // Like groups, the flattened text's paths have absolute transforms that
                // already include the text element's own transform.
                render_group(scene, text.flattened(), base_transform, error_handler);
            }
        }
    }
    scene.end_group();
}

/// Whether a group without a clip path has to be drawn as a layer of its own.
fn needs_layer(group: &usvg::Group) -> bool {
    group.opacity().get() != 1.0
        || group.blend_mode() != usvg::BlendMode::Normal
        || group.isolate()
        || group.mask().is_some()
        || !group.filters().is_empty()
}

/// Appends the shapes of a clip path and their clip rules to `out`, transformed by
/// `transform`.
fn collect_clip(group: &usvg::Group, transform: Affine, out: &mut Vec<(BezPath, Fill)>) {
    for node in group.children() {
        match node {
            usvg::Node::Path(path) if path.is_visible() => {
                let mut shape = util::to_bez_path(path);
                shape.apply_affine(transform);
                let rule = match path.fill().map(|fill| fill.rule()) {
                    Some(usvg::FillRule::EvenOdd) => Fill::EvenOdd,
                    _ => Fill::NonZero,
                };
                out.push((shape, rule));
            }
            usvg::Node::Group(g) => {
                collect_clip(g, transform * util::to_affine(&g.transform()), out);
            }
            usvg::Node::Text(text) => collect_clip(text.flattened(), transform, out),
            _ => {}
        }
    }
}

/// Pushes a layer for opacity or blending, clipped to the group's bounding box.
fn push_bounding_box_layer<S: RenderSink>(
    scene: &mut S,
    group: &usvg::Group,
    blend_mode: BlendMode,
    alpha: f32,
    transform: Affine,
) {
    let bounding_box = group.layer_bounding_box();
    let rect = Rect::from_origin_size(
        (bounding_box.x(), bounding_box.y()),
        (bounding_box.width() as f64, bounding_box.height() as f64),
    );
    scene.push_layer(blend_mode, alpha, transform, &rect);
}

/// Whether `clip`, in the group's coordinates, is a rectangle on the canvas that contains
/// everything the group draws, so that clipping changes nothing.
fn clips_nothing(clip: &BezPath, group: &usvg::Group) -> bool {
    let mut on_canvas = clip.clone();
    on_canvas.apply_affine(util::to_affine(&group.abs_transform()));
    let Some(bounds) = axis_aligned_rect(&on_canvas) else {
        return false;
    };
    let content = group.abs_stroke_bounding_box();
    bounds.x0 <= f64::from(content.left())
        && bounds.y0 <= f64::from(content.top())
        && bounds.x1 >= f64::from(content.right())
        && bounds.y1 >= f64::from(content.bottom())
}

/// The rectangle `path` outlines, if it is one closed outline of four straight,
/// axis-aligned sides.
fn axis_aligned_rect(path: &BezPath) -> Option<Rect> {
    use kurbo::PathEl;

    let mut points = [Point::ZERO; 5];
    let mut count = 0;
    for (i, element) in path.elements().iter().enumerate() {
        let point = match (i, element) {
            (0, PathEl::MoveTo(p)) | (1.., PathEl::LineTo(p)) => *p,
            (1.., PathEl::ClosePath) => continue,
            _ => return None,
        };
        if count == points.len() {
            return None;
        }
        points[count] = point;
        count += 1;
    }
    if count == 5 && points[4] == points[0] {
        count = 4;
    }
    if count != 4 {
        return None;
    }
    // Sides alternate between horizontal and vertical, and none has zero length.
    let is_rectangle = (0..4).all(|i| {
        let (a, b, c) = (points[i], points[(i + 1) % 4], points[(i + 2) % 4]);
        (a.x == b.x || a.y == b.y) && (a.x == b.x) != (b.x == c.x) && a != b
    });
    is_rectangle.then(|| path.bounding_box())
}

/// Whether filling `shapes` as one non-zero path gives their union: every shape is non-zero
/// and every subpath is made of straight lines that all turn the same way (rectangles and
/// other convex polygons, all wound in one direction). Overlapping parts then add up instead
/// of cancelling out.
fn merging_unites(shapes: &[(BezPath, Fill)]) -> bool {
    use kurbo::PathEl;

    let mut direction = 0.0_f64;
    for (shape, rule) in shapes {
        if *rule != Fill::NonZero {
            return false;
        }
        let mut points: Vec<Point> = Vec::new();
        let mut check = |points: &mut Vec<Point>| -> bool {
            if points.len() > 1 && points.first() == points.last() {
                points.pop();
            }
            let n = points.len();
            for i in 0..n {
                let (a, b, c) = (points[i], points[(i + 1) % n], points[(i + 2) % n]);
                let turn = (b - a).cross(c - b);
                if turn != 0.0 {
                    if direction * turn < 0.0 {
                        return false;
                    }
                    direction = turn;
                }
            }
            points.clear();
            true
        };
        for element in shape.elements() {
            match element {
                PathEl::MoveTo(p) => {
                    if !check(&mut points) {
                        return false;
                    }
                    points.push(*p);
                }
                PathEl::LineTo(p) => {
                    if points.last() != Some(p) {
                        points.push(*p);
                    }
                }
                PathEl::ClosePath => {}
                _ => return false,
            }
        }
        if !check(&mut points) {
            return false;
        }
    }
    true
}
