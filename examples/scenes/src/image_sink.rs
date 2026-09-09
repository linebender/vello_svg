// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::HashMap;
use std::sync::Arc;

use vello::Scene;
use vello_svg::kurbo::{Affine, Shape, Stroke};
use vello_svg::peniko::{
    BlendMode, Blob, Brush, Fill, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
};
use vello_svg::{RenderSink, usvg};

/// A Vello sink with raster decoding provided by the `image` crate.
#[derive(Default)]
pub(crate) struct ImageSink {
    pub(crate) scene: Scene,
    // Retain failed decodes too, so repeated images only report an error once.
    images: HashMap<Arc<Vec<u8>>, Option<ImageBrush>>,
}

impl RenderSink for ImageSink {
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    ) {
        self.scene
            .push_layer(Fill::NonZero, blend, alpha, transform, shape);
    }

    fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    fn fill(
        &mut self,
        fill: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        self.scene
            .fill(fill, transform, brush, Some(brush_transform), shape);
    }

    fn stroke(
        &mut self,
        stroke: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        self.scene
            .stroke(stroke, transform, brush, Some(brush_transform), shape);
    }

    fn draw_image(&mut self, image: &usvg::Image, transform: Affine) {
        let (data, format) = match image.kind() {
            usvg::ImageKind::JPEG(data) => (data, image::ImageFormat::Jpeg),
            usvg::ImageKind::PNG(data) => (data, image::ImageFormat::Png),
            usvg::ImageKind::GIF(data) => (data, image::ImageFormat::Gif),
            usvg::ImageKind::WEBP(data) => (data, image::ImageFormat::WebP),
            usvg::ImageKind::SVG(_) => return,
        };
        let decoded = self.images.entry(Arc::clone(data)).or_insert_with(|| {
            match decode_image(data, format) {
                Ok(decoded) => Some(decoded),
                Err(error) => {
                    eprintln!("Failed to decode SVG image {:?}: {error}", image.id());
                    None
                }
            }
        });
        if let Some(decoded) = decoded {
            self.scene.draw_image(&*decoded, transform);
        }
    }
}

fn decode_image(data: &[u8], format: image::ImageFormat) -> image::ImageResult<ImageBrush> {
    let decoded = image::load_from_memory_with_format(data, format)?.into_rgba8();
    let (width, height) = decoded.dimensions();
    Ok(ImageData {
        data: Blob::new(Arc::new(decoded.into_raw())),
        format: ImageFormat::Rgba8,
        // `into_rgba8` produces straight, not premultiplied, alpha.
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    }
    .into())
}
