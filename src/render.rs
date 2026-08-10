// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::util;
use vello::Scene;
use vello::kurbo::{Affine, BezPath, Rect};
use vello::peniko::{BlendMode, Compose, Fill, Mix};

pub(crate) fn render_group<F: FnMut(&mut Scene, &usvg::Node)>(
    scene: &mut Scene,
    group: &usvg::Group,
    transform: Affine,
    error_handler: &mut F,
) {
    for node in group.children() {
        let transform = transform * util::to_affine(&node.abs_transform());
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

                // Build the group's clip shape: either the union of every
                // path inside `clip-path` (including paths nested in groups
                // within the clip-path), or the group's bounding box as a
                // fallback when no clip-path is set. Previously this only
                // honoured a single-child clip-path and silently degraded
                // anything more complex to a bbox-only clip — see
                // linebender/vello_svg#72.
                let (clip_shape, is_compound) = build_clip_shape(g);

                if is_compound {
                    scene.push_layer(
                        Fill::NonZero,
                        blend_mode,
                        alpha,
                        transform,
                        clip_shape.as_path(),
                    );
                } else {
                    scene.push_layer(
                        Fill::NonZero,
                        blend_mode,
                        alpha,
                        transform,
                        clip_shape.as_rect(),
                    );
                }

                render_group(scene, g, Affine::IDENTITY, error_handler);

                // Apply the group's `<mask>` (if any) by drawing the mask's
                // rendered content as either a luminance or alpha mask over
                // the group's content. Previously masks were ignored.
                if let Some(mask) = g.mask() {
                    apply_mask(scene, mask, transform, error_handler);
                }

                scene.pop_layer();
            }
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }
                let local_path = util::to_bez_path(path);

                let do_fill = |scene: &mut Scene, error_handler: &mut F| {
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
                                Some(brush_transform),
                                &local_path,
                            );
                        } else {
                            error_handler(scene, node);
                        }
                    }
                };
                let do_stroke = |scene: &mut Scene, error_handler: &mut F| {
                    if let Some(stroke) = &path.stroke() {
                        if let Some((brush, brush_transform)) =
                            util::to_brush(stroke.paint(), stroke.opacity())
                        {
                            let conv_stroke = util::to_stroke(stroke);
                            scene.stroke(
                                &conv_stroke,
                                transform,
                                &brush,
                                Some(brush_transform),
                                &local_path,
                            );
                        } else {
                            error_handler(scene, node);
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
                        #[cfg(feature = "image")]
                        {
                            let Ok(decoded_image) = util::decode_raw_raster_image(img.kind())
                            else {
                                error_handler(scene, node);
                                continue;
                            };
                            let image = util::into_image(decoded_image);
                            let image_ts = util::to_affine(&img.abs_transform());
                            scene.draw_image(&image, image_ts);
                        }

                        #[cfg(not(feature = "image"))]
                        {
                            error_handler(scene, node);
                            continue;
                        }
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
}

/// The clip shape for a group: either a compound `BezPath` built from every
/// path inside the group's `clip-path`, or the group's `layer_bounding_box`
/// rect as a fallback when no `clip-path` is set.
enum ClipShape {
    Compound(BezPath),
    BBox(Rect),
}

impl ClipShape {
    fn as_path(&self) -> &BezPath {
        match self {
            Self::Compound(p) => p,
            Self::BBox(_) => unreachable!("caller checked is_compound"),
        }
    }
    fn as_rect(&self) -> &Rect {
        match self {
            Self::BBox(r) => r,
            Self::Compound(_) => unreachable!("caller checked is_compound"),
        }
    }
}

fn build_clip_shape(g: &usvg::Group) -> (ClipShape, bool) {
    if let Some(clip_path) = g.clip_path() {
        let mut compound = BezPath::new();
        collect_clip_paths(clip_path.root(), &mut compound);
        if !compound.elements().is_empty() {
            return (ClipShape::Compound(compound), true);
        }
    }
    let bb = g.layer_bounding_box();
    let rect = Rect::from_origin_size((bb.x(), bb.y()), (bb.width() as f64, bb.height() as f64));
    (ClipShape::BBox(rect), false)
}

/// Walk a clip-path's tree appending every `<path>` into `out`. Nested
/// groups (e.g. clip-path with `<g><path/><path/></g>`) recurse so all
/// reachable paths contribute. Non-path nodes (images, text) are ignored —
/// SVG only allows shape elements in clipPaths, but usvg may expose
/// flattened text as groups of paths, which this picks up naturally.
fn collect_clip_paths(group: &usvg::Group, out: &mut BezPath) {
    for child in group.children() {
        match child {
            usvg::Node::Path(p) => {
                out.extend(util::to_bez_path(p).iter());
            }
            usvg::Node::Group(g) => collect_clip_paths(g, out),
            _ => {}
        }
    }
}

/// Composite a usvg `<mask>` onto the current layer. Uses vello's
/// `push_luminance_mask_layer` for luminance-mode masks (the SVG default)
/// and a `Compose::DestIn` layer for alpha-mode masks.
///
/// The mask's own `mask.mask()` (a mask on the mask) is currently not
/// honoured — nested masking is rare and left as a follow-up.
fn apply_mask<F: FnMut(&mut Scene, &usvg::Node)>(
    scene: &mut Scene,
    mask: &usvg::Mask,
    transform: Affine,
    error_handler: &mut F,
) {
    let mr = mask.rect();
    let mask_rect = Rect::from_origin_size(
        (mr.x() as f64, mr.y() as f64),
        (mr.width() as f64, mr.height() as f64),
    );
    match mask.kind() {
        usvg::MaskType::Luminance => {
            scene.push_luminance_mask_layer(Fill::NonZero, 1.0, transform, &mask_rect);
        }
        usvg::MaskType::Alpha => {
            scene.push_layer(
                Fill::NonZero,
                BlendMode::new(Mix::Normal, Compose::DestIn),
                1.0,
                transform,
                &mask_rect,
            );
        }
    }
    render_group(scene, mask.root(), Affine::IDENTITY, error_handler);
    scene.pop_layer();
}
