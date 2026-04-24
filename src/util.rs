// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello::Scene;
use vello::kurbo::{Affine, BezPath, PathEl, Point, Rect, Stroke};
use vello::peniko::color::{DynamicColor, palette};
use vello::peniko::{Brush, Color, Fill};

#[cfg(feature = "image")]
use vello::peniko::{Blob, ImageBrush};

pub fn to_affine(ts: &usvg::Transform) -> Affine {
    let usvg::Transform {
        sx,
        kx,
        ky,
        sy,
        tx,
        ty,
    } = ts;
    Affine::new([sx, ky, kx, sy, tx, ty].map(|&x| f64::from(x)))
}

pub fn to_stroke(stroke: &usvg::Stroke) -> Stroke {
    let mut conv_stroke = Stroke::new(stroke.width().get() as f64)
        .with_caps(match stroke.linecap() {
            usvg::LineCap::Butt => vello::kurbo::Cap::Butt,
            usvg::LineCap::Round => vello::kurbo::Cap::Round,
            usvg::LineCap::Square => vello::kurbo::Cap::Square,
        })
        .with_join(match stroke.linejoin() {
            usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => vello::kurbo::Join::Miter,
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
    conv_stroke
}

pub fn to_bez_path(path: &usvg::Path) -> BezPath {
    BezPath::from_iter(path_elements(path))
}

/// Iterate the [`kurbo::PathEl`](vello::kurbo::PathEl)s of a [`usvg::Path`].
///
/// The semantics of SVG paths don't line up with `BezPath` (an SVG subpath that
/// `Z`-closes and then begins a new segment without an intervening `M` must
/// emit an extra `MoveTo` on the `BezPath` side), so this does more than a
/// straight segment-to-element map.
///
/// Using this with [`kurbo::BezPath`](vello::kurbo::BezPath)'s [`Extend`] impl
/// lets callers retain a single `BezPath` buffer across frames:
///
/// ```no_run
/// use vello_svg::vello::kurbo::BezPath;
/// # let svg = "";
/// # let tree = vello_svg::usvg::Tree::from_str(svg, &Default::default()).unwrap();
/// # fn some_path(t: &vello_svg::usvg::Tree) -> &vello_svg::usvg::Path { unimplemented!() }
/// let mut buf = BezPath::new();
/// // Every frame, for each path:
/// let path = some_path(&tree);
/// buf.truncate(0); // retains capacity
/// buf.extend(vello_svg::util::path_elements(path));
/// // ... hand `buf` to the scene, then reuse it for the next path ...
/// ```
///
/// This matches the [`vello::Scene::reset`](vello::Scene::reset) pattern:
/// callers are expected to clear the destination buffer themselves between
/// uses.
pub fn path_elements(path: &usvg::Path) -> PathElements<'_> {
    PathElements {
        inner: path.data().segments(),
        just_closed: false,
        most_recent_initial: Point::ZERO,
        pending: None,
    }
}

/// Iterator returned by [`path_elements`].
#[derive(Clone)]
pub struct PathElements<'a> {
    inner: usvg::tiny_skia_path::PathSegmentsIter<'a>,
    just_closed: bool,
    most_recent_initial: Point,
    pending: Option<PathEl>,
}

// `PathSegmentsIter` does not implement `Debug` upstream.
impl core::fmt::Debug for PathElements<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PathElements")
            .field("just_closed", &self.just_closed)
            .field("most_recent_initial", &self.most_recent_initial)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl Iterator for PathElements<'_> {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        if let Some(pending) = self.pending.take() {
            return Some(pending);
        }
        let seg = self.inner.next()?;
        let just_closed = std::mem::take(&mut self.just_closed);
        Some(match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                let new_initial = Point::new(p.x as f64, p.y as f64);
                if just_closed {
                    let prior_initial = self.most_recent_initial;
                    self.most_recent_initial = new_initial;
                    self.pending = Some(PathEl::MoveTo(new_initial));
                    PathEl::MoveTo(prior_initial)
                } else {
                    self.most_recent_initial = new_initial;
                    PathEl::MoveTo(new_initial)
                }
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                let pt = Point::new(p.x as f64, p.y as f64);
                if just_closed {
                    self.pending = Some(PathEl::LineTo(pt));
                    PathEl::MoveTo(self.most_recent_initial)
                } else {
                    PathEl::LineTo(pt)
                }
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                let a = Point::new(p1.x as f64, p1.y as f64);
                let b = Point::new(p2.x as f64, p2.y as f64);
                if just_closed {
                    self.pending = Some(PathEl::QuadTo(a, b));
                    PathEl::MoveTo(self.most_recent_initial)
                } else {
                    PathEl::QuadTo(a, b)
                }
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                let a = Point::new(p1.x as f64, p1.y as f64);
                let b = Point::new(p2.x as f64, p2.y as f64);
                let c = Point::new(p3.x as f64, p3.y as f64);
                if just_closed {
                    self.pending = Some(PathEl::CurveTo(a, b, c));
                    PathEl::MoveTo(self.most_recent_initial)
                } else {
                    PathEl::CurveTo(a, b, c)
                }
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                self.just_closed = true;
                PathEl::ClosePath
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.inner.size_hint();
        // Each inner segment expands to 1 or 2 `PathEl`s, plus any pending item.
        let pending = usize::from(self.pending.is_some());
        (
            lo.saturating_add(pending),
            hi.and_then(|h| h.checked_mul(2)?.checked_add(pending)),
        )
    }
}

#[cfg(feature = "image")]
pub fn into_image(image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> ImageBrush {
    use vello::peniko::ImageAlphaType;
    use vello::peniko::ImageData;

    let (width, height) = (image.width(), image.height());
    let image_data: Vec<u8> = image.into_vec();
    ImageData {
        data: Blob::new(std::sync::Arc::new(image_data)),
        format: vello::peniko::ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::AlphaPremultiplied,
        width,
        height,
    }
    .into()
}

pub fn to_brush(paint: &usvg::Paint, opacity: usvg::Opacity) -> Option<(Brush, Affine)> {
    match paint {
        usvg::Paint::Color(color) => Some((
            Brush::Solid(Color::from_rgba8(
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
                .map(|stop| vello::peniko::ColorStop {
                    offset: stop.offset().get(),
                    color: DynamicColor::from_alpha_color(Color::from_rgba8(
                        stop.color().red,
                        stop.color().green,
                        stop.color().blue,
                        (stop.opacity() * opacity).to_u8(),
                    )),
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
                .map(|stop| vello::peniko::ColorStop {
                    offset: stop.offset().get(),
                    color: DynamicColor::from_alpha_color(Color::from_rgba8(
                        stop.color().red,
                        stop.color().green,
                        stop.color().blue,
                        (stop.opacity() * opacity).to_u8(),
                    )),
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

/// Error handler function for [`super::append_tree_with`] which draws a transparent red box
/// instead of unsupported SVG features
pub fn default_error_handler(scene: &mut Scene, node: &usvg::Node) {
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
        palette::css::RED.with_alpha(0.5),
        None,
        &rect,
    );
}

#[cfg(feature = "image")]
pub fn decode_raw_raster_image(
    img: &usvg::ImageKind,
) -> Result<image::RgbaImage, image::ImageError> {
    // All `image::ImageFormat` variants exist even if the feature in the image crate is disabled,
    // but `image::load_from_memory_with_format` will fail with an Unsupported error if the
    // image crate feature flag is disabled. So we don't need any of our own feature handling here.
    let (data, format) = match img {
        usvg::ImageKind::JPEG(data) => (data, image::ImageFormat::Jpeg),
        usvg::ImageKind::PNG(data) => (data, image::ImageFormat::Png),
        usvg::ImageKind::GIF(data) => (data, image::ImageFormat::Gif),
        usvg::ImageKind::WEBP(data) => (data, image::ImageFormat::WebP),
        usvg::ImageKind::SVG(_) => unreachable!(),
    };

    let dyn_image = image::load_from_memory_with_format(data, format)?;
    Ok(dyn_image.into_rgba8())
}
