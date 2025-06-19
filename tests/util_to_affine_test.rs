// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(missing_docs, reason = "regression test for util::to_affine")]
#[cfg(test)]
pub mod util_to_affine_test {
    use usvg::Transform;
    use vello::kurbo::Affine;
    use vello_svg::util::to_affine;

    #[test]
    fn regression_test() {
        let usvg_transform = Transform {
            sx: 1.,
            kx: 2.,
            ky: 3.,
            sy: 4.,
            tx: 5.,
            ty: 6.,
        };

        let result = to_affine(&usvg_transform);
        let expected = Affine::new([1.0, 3.0, 2.0, 4.0, 5.0, 6.0]);

        assert_eq!(result, expected);
    }
}
