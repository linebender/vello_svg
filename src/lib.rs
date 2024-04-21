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

mod util;

use std::convert::Infallible;
use vello::peniko::{BlendMode, Fill};
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
    render_tree_with::<_, Infallible>(scene, svg, &mut util::default_error_handler)
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
    error_handler: &mut F,
) -> Result<(), E> {
    render_tree_impl(
        scene,
        svg,
        &svg.view_box(),
        &usvg::Transform::identity(),
        error_handler,
    )
}

// A helper function to render a tree with a given transform.
fn render_tree_impl<F: FnMut(&mut Scene, &usvg::Node) -> Result<(), E>, E>(
    scene: &mut Scene,
    tree: &usvg::Tree,
    view_box: &usvg::ViewBox,
    ts: &usvg::Transform,
    error_handler: &mut F,
) -> Result<(), E> {
    let ts = &ts.pre_concat(view_box.to_transform(tree.size()));
    let transform = util::to_affine(ts);
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
    render_group(
        scene,
        tree.root(),
        &ts.pre_concat(tree.root().transform()),
        error_handler,
    )?;
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
        let transform = util::to_affine(ts);
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

                render_group(scene, g, &ts.pre_concat(g.transform()), error_handler)?;

                if pushed_clip {
                    scene.pop_layer();
                }
            }
            usvg::Node::Path(path) => {
                if path.visibility() != usvg::Visibility::Visible {
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
                            return error_handler(scene, node);
                        }
                    }
                    Ok(())
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
                        let Ok(decoded_image) = util::decode_raw_raster_image(img.kind()) else {
                            error_handler(scene, node)?;
                            continue;
                        };
                        let image = util::into_image(decoded_image);
                        let Some(size) =
                            usvg::Size::from_wh(image.width as f32, image.height as f32)
                        else {
                            error_handler(scene, node)?;
                            continue;
                        };
                        let view_box = img.view_box();
                        let new_size = view_box.rect.size();
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
                        let image_ts = util::to_affine(&ts.pre_concat(view_box_transform));
                        scene.draw_image(&image, image_ts);

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
