# Changelog

<!-- Instructions

This changelog follows the patterns described here: <https://keepachangelog.com/en/1.0.0/>.

Subheadings to categorize changes are `added, changed, deprecated, removed, fixed, security`.

-->

## Unreleased

### changed

- Updated `usvg` to 0.41
- MPL 2.0 is no longer a license requirement
- The root image viewBox clipping was removed, to be added back at a later time

### fixed

- The image viewBox is now properly translated
- `vello_svg::render_tree_with` no longer takes a transform parameter. This is to make it consistent with the documentation and `vello_svg::render_tree`.

## 0.1 (2024-03-11)

- Initial release
