# Changelog

<!-- Instructions

This changelog follows the patterns described here: <https://keepachangelog.com/en/1.0.0/>.

Subheadings to categorize changes are `added, changed, deprecated, removed, fixed, security`.

-->

## [Unreleased][]

This release has an [MSRV][] of 1.75.

### Added

- Support for rendering basic text ([#26] by [@nicoburns])

### Fixed

- Transform of nested SVGs ([#26] by [@nicoburns])

### Changed

- Updated to vello 0.2.1 ([#28] by [@waywardmonkeys])

## [0.3.0][] (2024-07-04)

This release has an [MSRV][] of 1.75.

### Added

- Added `vello_svg::Error`, which is returned by new functions that read text into a `usvg::Tree`.
- Added `vello_svg::render`, which takes an svg string and renders to a new vello scene.
- Added `vello_svg::append`, which takes an svg string and renders to a provided vello scene.
- Added `vello_svg::append_with`, which takes an svg string and renders to a provided vello scene with and error handler.
- Added `vello_svg::render_tree`, which takes a usvg::Tree and renders to a provided vello scene with and error handler.

### Changed

- Updated to vello 0.2
- Updated to usvg 0.42
- Renamed `render_tree` to `append_tree`
- Renamed `render_tree_with` to `append_tree_with` and removed the `Result<(), E>` return type for the error handler.

### Removed

- All code and related profiling (`wgpu_profiler`) used in examples.

## [0.2.0][] (2024-05-26)

This release has an [MSRV][] of 1.75.

### Added

- Make `util` module public and some minor doc fixes. [#12](https://github.com/linebender/vello_svg/pull/12)

### Changed

- Updated `usvg` to 0.41
- Disable `vello`'s default `wgpu` feature, and provide a `wgpu` passthrough feature to turn it back on. [#10](https://github.com/linebender/vello_svg/pull/10)

### Fixed

- The image viewBox is now properly translated
- `vello_svg::render_tree_with` no longer takes a transform parameter. This is to make it consistent with the documentation and `vello_svg::render_tree`.


### Removed

- MPL 2.0 is no longer a license requirement
- The root image viewBox clipping was removed, to be added back at a later time

## [0.1.0][] (2024-03-11)

This release has an [MSRV][] of 1.75.

- Initial release

[@nicoburns]: https://github.com/nicoburns
[@waywardmonkeys]: https://github.com/waywardmonkeys

[#26]: https://github.com/linebender/vello_svg/pull/26
[#28]: https://github.com/linebender/vello_svg/pull/28

[Unreleased]: https://github.com/linebender/vello_svg/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/linebender/vello_svg/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/linebender/vello_svg/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/linebender/vello_svg/releases/tag/v0.1.0

[MSRV]: README.md#minimum-supported-rust-version-msrv
