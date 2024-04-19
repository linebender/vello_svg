// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Render a [`usvg::Tree`] to a Vello [`Scene`](crate::vello::Scene)
//!
//! This currently lacks support for a [number of important](crate#unsupported-features) SVG features.
//! This is because this integration was developed for examples, which only need to support enough SVG
//! to demonstrate Vello.
//!
//! However, this is also intended to be the preferred integration between Vello and [usvg], so [consider
//! contributing](https://github.com/linebender/vello_svg) if you need a feature which is missing.
//!
//! [`render_tree_with`] is the primary entry point function, which supports choosing the behaviour
//! when [unsupported features](crate#unsupported-features) are detected. In a future release where there are
//! no unsupported features, this may be phased out
//!
//! [`render_tree`] is a convenience wrapper around [`render_tree_with`] which renders an indicator around not
//! yet supported features
//!
//! This crate also re-exports [`usvg`], to make handling dependency versions easier
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

mod geom;

use std::convert::Infallible;
use std::sync::Arc;
use vello::kurbo::{Affine, BezPath, Point, Rect, Stroke};
use vello::peniko::{BlendMode, Blob, Brush, Color, Fill, Image};
use vello::Scene;

/// Re-export vello.
pub use vello;

/// Re-export usvg.
pub use usvg;

/// Append a [`usvg::Tree`] into a Vello [`Scene`], with default error handling
/// This will draw a red box over (some) unsupported elements
///
/// Calls [`render_tree_with`] with an error handler implementing the above.
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn render_tree(scene: &mut Scene, svg: &usvg::Tree) {
    render_tree_with::<_, Infallible>(
        scene,
        svg,
        &usvg::Transform::identity(),
        &mut default_error_handler,
    )
    .unwrap_or_else(|e| match e {});
}

/// Append a [`usvg::Tree`] into a Vello [`Scene`].
///
/// Calls [`render_tree_with`] with [`default_error_handler`].
/// This will draw a red box over unsupported element types.
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn render_tree_with<F: FnMut(&mut Scene, &usvg::Node) -> Result<(), E>, E>(
    scene: &mut Scene,
    svg: &usvg::Tree,
    ts: &usvg::Transform,
    error_handler: &mut F,
) -> Result<(), E> {
    render_tree_impl(scene, svg, &svg.view_box(), ts, error_handler)
}

fn render_tree_impl<F: FnMut(&mut Scene, &usvg::Node) -> Result<(), E>, E>(
    scene: &mut Scene,
    svg: &usvg::Tree,
    view_box: &usvg::ViewBox,
    ts: &usvg::Transform,
    error_handler: &mut F,
) -> Result<(), E> {
    let ts = &ts.pre_concat(view_box.to_transform(svg.size()));
    let transform = to_affine(ts);
    scene.push_layer(
        BlendMode {
            mix: vello::peniko::Mix::Clip,
            compose: vello::peniko::Compose::SrcOver,
        },
        1.0,
        transform,
        &vello::kurbo::Rect::new(
            view_box.rect.left().into(),
            view_box.rect.top().into(),
            view_box.rect.right().into(),
            view_box.rect.bottom().into(),
        ),
    );
    let (view_box_transform, clip) =
        geom::view_box_to_transform_with_clip(view_box, svg.size().to_int_size());
    let view_box_transform = view_box_transform.pre_concat(view_box.to_transform(svg.size()));
    if let Some(clip) = clip {
        scene.push_layer(
            BlendMode {
                mix: vello::peniko::Mix::Clip,
                compose: vello::peniko::Compose::SrcOver,
            },
            1.0,
            transform,
            &vello::kurbo::Rect::new(
                clip.left().into(),
                clip.top().into(),
                clip.right().into(),
                clip.bottom().into(),
            ),
        );
    }
    render_group(
        scene,
        svg.root(),
        &ts.pre_concat(view_box_transform)
            .pre_concat(svg.root().transform()),
        error_handler,
    )?;
    if clip.is_some() {
        scene.pop_layer();
    }
    scene.pop_layer();

    Ok(())
}

fn render_group<F: FnMut(&mut Scene, &usvg::Node) -> Result<(), E>, E>(
    scene: &mut Scene,
    group: &usvg::Group,
    ts: &usvg::Transform,
    error_handler: &mut F,
) -> Result<(), E> {
    for node in group.children() {
        let transform = to_affine(ts);
        match node {
            usvg::Node::Group(g) => {
                let mut pushed_clip = false;
                if let Some(clip_path) = g.clip_path() {
                    if let Some(usvg::Node::Path(clip_path)) = clip_path.root().children().first() {
                        // support clip-path with a single path
                        let local_path = to_bez_path(clip_path);
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

                render_group(scene, g, &ts.pre_concat(g.transform()), error_handler)?;

                if pushed_clip {
                    scene.pop_layer();
                }
            }
            usvg::Node::Path(path) => {
                if path.visibility() != usvg::Visibility::Visible {
                    continue;
                }
                let local_path = to_bez_path(path);

                let do_fill = |scene: &mut Scene, error_handler: &mut F| {
                    if let Some(fill) = &path.fill() {
                        if let Some((brush, brush_transform)) =
                            paint_to_brush(fill.paint(), fill.opacity())
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
                            return error_handler(scene, node);
                        }
                    }
                    Ok(())
                };
                let do_stroke = |scene: &mut Scene, error_handler: &mut F| {
                    if let Some(stroke) = &path.stroke() {
                        if let Some((brush, brush_transform)) =
                            paint_to_brush(stroke.paint(), stroke.opacity())
                        {
                            let mut conv_stroke = Stroke::new(stroke.width().get() as f64)
                                .with_caps(match stroke.linecap() {
                                    usvg::LineCap::Butt => vello::kurbo::Cap::Butt,
                                    usvg::LineCap::Round => vello::kurbo::Cap::Round,
                                    usvg::LineCap::Square => vello::kurbo::Cap::Square,
                                })
                                .with_join(match stroke.linejoin() {
                                    usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => {
                                        vello::kurbo::Join::Miter
                                    }
                                    usvg::LineJoin::Round => vello::kurbo::Join::Round,
                                    usvg::LineJoin::Bevel => vello::kurbo::Join::Bevel,
                                })
                                .with_miter_limit(stroke.miterlimit().get() as f64);
                            if let Some(dash_array) = stroke.dasharray().as_ref() {
                                conv_stroke = conv_stroke.with_dashes(
                                    stroke.dashoffset() as f64,
                                    dash_array.iter().map(|x| *x as f64),
                                );
                            }
                            scene.stroke(
                                &conv_stroke,
                                transform,
                                &brush,
                                Some(brush_transform),
                                &local_path,
                            );
                        } else {
                            return error_handler(scene, node);
                        }
                    }
                    Ok(())
                };
                match path.paint_order() {
                    usvg::PaintOrder::FillAndStroke => {
                        do_fill(scene, error_handler)?;
                        do_stroke(scene, error_handler)?;
                    }
                    usvg::PaintOrder::StrokeAndFill => {
                        do_stroke(scene, error_handler)?;
                        do_fill(scene, error_handler)?;
                    }
                }
            }
            usvg::Node::Image(img) => {
                if img.visibility() != usvg::Visibility::Visible {
                    continue;
                }
                match img.kind() {
                    usvg::ImageKind::JPEG(_)
                    | usvg::ImageKind::PNG(_)
                    | usvg::ImageKind::GIF(_) => {
                        let Ok(decoded_image) = decode_raw_raster_image(img.kind()) else {
                            error_handler(scene, node)?;
                            continue;
                        };
                        let Some(size) = usvg::Size::from_wh(
                            decoded_image.width() as f32,
                            decoded_image.height() as f32,
                        ) else {
                            error_handler(scene, node)?;
                            continue;
                        };
                        let view_box = img.view_box();
                        let new_size = geom::fit_view_box(size, &view_box);
                        let (tx, ty) = usvg::utils::aligned_pos(
                            view_box.aspect.align,
                            view_box.rect.x(),
                            view_box.rect.y(),
                            view_box.rect.width() - new_size.width(),
                            view_box.rect.height() - new_size.height(),
                        );
                        let (sx, sy) = (
                            new_size.width() / size.width(),
                            new_size.height() / size.height(),
                        );
                        let view_box_transform =
                            usvg::Transform::from_row(sx, 0.0, 0.0, sy, tx, ty);
                        let (width, height) = (decoded_image.width(), decoded_image.height());
                        scene.push_layer(
                            BlendMode {
                                mix: vello::peniko::Mix::Clip,
                                compose: vello::peniko::Compose::SrcOver,
                            },
                            1.0,
                            transform,
                            &vello::kurbo::Rect::new(
                                view_box.rect.left().into(),
                                view_box.rect.top().into(),
                                view_box.rect.right().into(),
                                view_box.rect.bottom().into(),
                            ),
                        );

                        let image_ts = to_affine(&ts.pre_concat(view_box_transform));
                        let image_data: Arc<Vec<u8>> = decoded_image.into_vec().into();
                        scene.draw_image(
                            &Image::new(
                                Blob::new(image_data),
                                vello::peniko::Format::Rgba8,
                                width,
                                height,
                            ),
                            image_ts,
                        );

                        scene.pop_layer();
                    }
                    usvg::ImageKind::SVG(svg) => {
                        render_tree_impl(scene, svg, &img.view_box(), ts, error_handler)?;
                    }
                }
            }
            usvg::Node::Text(_) => {
                error_handler(scene, node)?;
            }
        }
    }

    Ok(())
}

fn decode_raw_raster_image(img: &usvg::ImageKind) -> Result<image::RgbaImage, image::ImageError> {
    let res = match img {
        usvg::ImageKind::JPEG(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
        }
        usvg::ImageKind::PNG(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::Png)
        }
        usvg::ImageKind::GIF(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::Gif)
        }
        usvg::ImageKind::SVG(_) => unreachable!(),
    }?
    .into_rgba8();
    Ok(res)
}

fn to_affine(ts: &usvg::Transform) -> Affine {
    let usvg::Transform {
        sx,
        kx,
        ky,
        sy,
        tx,
        ty,
    } = ts;
    Affine::new([sx, kx, ky, sy, tx, ty].map(|&x| f64::from(x)))
}

fn to_bez_path(path: &usvg::Path) -> BezPath {
    let mut local_path = BezPath::new();
    // The semantics of SVG paths don't line up with `BezPath`; we
    // must manually track initial points
    let mut just_closed = false;
    let mut most_recent_initial = (0., 0.);
    for elt in path.data().segments() {
        match elt {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                most_recent_initial = (p.x.into(), p.y.into());
                local_path.move_to(most_recent_initial)
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.line_to(Point::new(p.x as f64, p.y as f64))
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.quad_to(
                    Point::new(p1.x as f64, p1.y as f64),
                    Point::new(p2.x as f64, p2.y as f64),
                )
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.curve_to(
                    Point::new(p1.x as f64, p1.y as f64),
                    Point::new(p2.x as f64, p2.y as f64),
                    Point::new(p3.x as f64, p3.y as f64),
                )
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                just_closed = true;
                local_path.close_path()
            }
        }
    }

    local_path
}

/// Error handler function for [`render_tree_with`] which draws a transparent red box
/// instead of unsupported SVG features
pub fn default_error_handler(scene: &mut Scene, node: &usvg::Node) -> Result<(), Infallible> {
    let bb = node.bounding_box();
    let rect = Rect {
        x0: bb.left() as f64,
        y0: bb.top() as f64,
        x1: bb.right() as f64,
        y1: bb.bottom() as f64,
    };
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::RED.with_alpha_factor(0.5),
        None,
        &rect,
    );

    Ok(())
}

fn paint_to_brush(paint: &usvg::Paint, opacity: usvg::Opacity) -> Option<(Brush, Affine)> {
    match paint {
        usvg::Paint::Color(color) => Some((
            Brush::Solid(Color::rgba8(
                color.red,
                color.green,
                color.blue,
                opacity.to_u8(),
            )),
            Affine::IDENTITY,
        )),
        usvg::Paint::LinearGradient(gr) => {
            let stops: Vec<vello::peniko::ColorStop> = gr
                .stops()
                .iter()
                .map(|stop| {
                    let mut cstop = vello::peniko::ColorStop::default();
                    cstop.color.r = stop.color().red;
                    cstop.color.g = stop.color().green;
                    cstop.color.b = stop.color().blue;
                    cstop.color.a = (stop.opacity() * opacity).to_u8();
                    cstop.offset = stop.offset().get();
                    cstop
                })
                .collect();
            let start = Point::new(gr.x1() as f64, gr.y1() as f64);
            let end = Point::new(gr.x2() as f64, gr.y2() as f64);
            let arr = [
                gr.transform().sx,
                gr.transform().ky,
                gr.transform().kx,
                gr.transform().sy,
                gr.transform().tx,
                gr.transform().ty,
            ]
            .map(f64::from);
            let transform = Affine::new(arr);
            let gradient =
                vello::peniko::Gradient::new_linear(start, end).with_stops(stops.as_slice());
            Some((Brush::Gradient(gradient), transform))
        }
        usvg::Paint::RadialGradient(gr) => {
            let stops: Vec<vello::peniko::ColorStop> = gr
                .stops()
                .iter()
                .map(|stop| {
                    let mut cstop = vello::peniko::ColorStop::default();
                    cstop.color.r = stop.color().red;
                    cstop.color.g = stop.color().green;
                    cstop.color.b = stop.color().blue;
                    cstop.color.a = (stop.opacity() * opacity).to_u8();
                    cstop.offset = stop.offset().get();
                    cstop
                })
                .collect();

            let start_center = Point::new(gr.cx() as f64, gr.cy() as f64);
            let end_center = Point::new(gr.fx() as f64, gr.fy() as f64);
            let start_radius = 0_f32;
            let end_radius = gr.r().get();
            let arr = [
                gr.transform().sx,
                gr.transform().ky,
                gr.transform().kx,
                gr.transform().sy,
                gr.transform().tx,
                gr.transform().ty,
            ]
            .map(f64::from);
            let transform = Affine::new(arr);
            let gradient = vello::peniko::Gradient::new_two_point_radial(
                start_center,
                start_radius,
                end_center,
                end_radius,
            )
            .with_stops(stops.as_slice());
            Some((Brush::Gradient(gradient), transform))
        }
        usvg::Paint::Pattern(_) => None,
    }
}
