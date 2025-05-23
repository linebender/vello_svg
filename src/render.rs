// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::util;
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{BlendMode, Fill};

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
                let mut pushed_clip = false;
                if let Some(clip_path) = g.clip_path() {
                    if let Some(usvg::Node::Path(clip_path)) = clip_path.root().children().first() {
                        // support clip-path with a single path
                        let local_path = util::to_bez_path(clip_path);
                        scene.push_layer(
                            BlendMode {
                                mix: vello::peniko::Mix::Clip,
                                compose: vello::peniko::Compose::SrcOver,
                            },
                            1.0,
                            transform,
                            &local_path,
                        );
                        pushed_clip = true;
                    }
                }

                render_group(scene, g, Affine::IDENTITY, error_handler);

                if pushed_clip {
                    scene.pop_layer();
                }
            }
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }
                let local_path = util::to_bez_path(path);

                let do_fill = |scene: &mut Scene, error_handler: &mut F| {
                    if let Some(fill) = &path.fill() {
                        match util::to_brush(fill.paint(), fill.opacity()) {
                            Some((brush, brush_transform)) => {
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
                            }
                            _ => {
                                error_handler(scene, node);
                            }
                        }
                    }
                };
                let do_stroke = |scene: &mut Scene, error_handler: &mut F| {
                    if let Some(stroke) = &path.stroke() {
                        match util::to_brush(stroke.paint(), stroke.opacity()) {
                            Some((brush, brush_transform)) => {
                                let conv_stroke = util::to_stroke(stroke);
                                scene.stroke(
                                    &conv_stroke,
                                    transform,
                                    &brush,
                                    Some(brush_transform),
                                    &local_path,
                                );
                            }
                            _ => {
                                error_handler(scene, node);
                            }
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
