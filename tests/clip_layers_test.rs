// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(missing_docs, reason = "tests for the layers pushed for groups")]

use vello_svg::RenderSink;
use vello_svg::kurbo::{Affine, Rect, Shape, Stroke};
use vello_svg::peniko::{BlendMode, Brush, Fill};

/// Records the layers pushed (their clip shapes' bounds, in canvas coordinates) and the
/// number of shapes drawn.
#[derive(Default)]
struct Recorder {
    layers: Vec<Rect>,
    drawn: usize,
}

impl RenderSink for Recorder {
    fn push_layer(
        &mut self,
        _blend: impl Into<BlendMode>,
        _alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    ) {
        self.layers
            .push(transform.transform_rect_bbox(shape.bounding_box()));
    }

    fn pop_layer(&mut self) {}

    fn fill(&mut self, _: Fill, _: Affine, _: &Brush, _: Affine, _: &impl Shape) {
        self.drawn += 1;
    }

    fn stroke(&mut self, _: &Stroke, _: Affine, _: &Brush, _: Affine, _: &impl Shape) {
        self.drawn += 1;
    }
}

fn record(svg: &str) -> Recorder {
    let mut recorder = Recorder::default();
    vello_svg::append(&mut recorder, svg).unwrap();
    recorder
}

fn assert_rect_eq(actual: Rect, expected: Rect) {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
    assert!(
        close(actual.x0, expected.x0)
            && close(actual.y0, expected.y0)
            && close(actual.x1, expected.x1)
            && close(actual.y1, expected.y1),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn plain_groups_push_no_layer() {
    // A layer clipped to a group's bounding box cuts the outer half of a stroke lying on
    // the edge of that box. Plain groups don't need one.
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <g id="a"><g id="b" transform="scale(1.5)">
                <path d="M 10 10 H 50" stroke="black" stroke-width="0.5" fill="none"/>
            </g></g>
        </svg>"#,
    );
    assert!(recorder.layers.is_empty(), "{:?}", recorder.layers);
    assert_eq!(recorder.drawn, 1);
}

#[test]
fn group_opacity_pushes_a_layer() {
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <g opacity="0.5"><rect x="10" y="10" width="20" height="20"/></g>
        </svg>"#,
    );
    assert_eq!(recorder.layers.len(), 1);
}

#[test]
fn clip_path_with_a_transformed_shape() {
    // usvg puts the transformed rectangle into a group inside the clip path.
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <clipPath id="c"><rect x="5" y="5" width="10" height="10" transform="scale(2)"/></clipPath>
            <g clip-path="url(#c)"><rect width="100" height="100"/></g>
        </svg>"#,
    );
    assert_eq!(recorder.layers.len(), 1);
    assert_rect_eq(recorder.layers[0], Rect::new(10.0, 10.0, 30.0, 30.0));
}

#[test]
fn clip_path_on_a_transformed_element() {
    // The clip path is in the user space of the element, after its own transform.
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <clipPath id="c"><rect width="10" height="10"/></clipPath>
            <rect width="50" height="50" transform="translate(20 30)" clip-path="url(#c)"/>
        </svg>"#,
    );
    assert_eq!(recorder.layers.len(), 1);
    assert_rect_eq(recorder.layers[0], Rect::new(20.0, 30.0, 30.0, 40.0));
}

#[test]
fn clip_path_with_several_shapes() {
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <clipPath id="c">
                <rect x="10" y="10" width="10" height="10"/>
                <rect x="60" y="70" width="10" height="10"/>
            </clipPath>
            <g clip-path="url(#c)"><rect width="100" height="100"/></g>
        </svg>"#,
    );
    assert_eq!(recorder.layers.len(), 1);
    assert_rect_eq(recorder.layers[0], Rect::new(10.0, 10.0, 70.0, 80.0));
}

#[test]
fn clip_path_with_object_bounding_box_units() {
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <clipPath id="c" clipPathUnits="objectBoundingBox">
                <rect width="0.5" height="0.5"/>
            </clipPath>
            <rect x="20" y="20" width="40" height="40" clip-path="url(#c)"/>
        </svg>"#,
    );
    assert_eq!(recorder.layers.len(), 1);
    assert_rect_eq(recorder.layers[0], Rect::new(20.0, 20.0, 40.0, 40.0));
}

#[test]
fn clip_path_that_clips_nothing_pushes_no_layer() {
    // CAD exports put a rectangle like this around every element.
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <clipPath id="c"><rect width="100" height="100" transform="scale(1 -1) translate(0 -100)"/></clipPath>
            <g clip-path="url(#c)"><rect x="10" y="10" width="20" height="20"/></g>
        </svg>"#,
    );
    assert!(recorder.layers.is_empty(), "{:?}", recorder.layers);
    assert_eq!(recorder.drawn, 1);
}

#[test]
fn rotated_clip_path_is_kept() {
    // A rotated square is not a rectangle on the canvas, so it is kept even though its
    // bounds contain the content.
    let recorder = record(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <clipPath id="c"><rect x="-20" y="-20" width="40" height="40" transform="translate(50 50) rotate(45)"/></clipPath>
            <g clip-path="url(#c)"><rect x="25" y="25" width="50" height="50"/></g>
        </svg>"#,
    );
    assert_eq!(recorder.layers.len(), 1);
}
