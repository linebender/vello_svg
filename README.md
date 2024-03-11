<div align="center">

# Vello SVG

**An integration to parse and render SVG with [Vello](https://vello.dev).**

[![Linebender Zulip](https://img.shields.io/badge/Linebender-%23gpu-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/197075-gpu)
[![dependency status](https://deps.rs/repo/github/linebender/vello_svg/status.svg)](https://deps.rs/repo/github/linebender/vello_svg)
[![(MIT/Apache 2.0)+MPL 2.0](https://img.shields.io/badge/license-(MIT%2FApache)+MPL2-blue.svg)](#license)
[![wgpu version](https://img.shields.io/badge/wgpu-v0.19-orange.svg)](https://crates.io/crates/wgpu)
[![vello version](https://img.shields.io/badge/vello-v0.1.0-purple.svg)](https://crates.io/crates/vello)

[![Crates.io](https://img.shields.io/crates/v/vello_svg.svg)](https://crates.io/crates/vello_svg)
[![Docs](https://docs.rs/vello_svg/badge.svg)](https://docs.rs/vello_svg)
[![Build status](https://github.com/linebender/vello_svg/workflows/CI/badge.svg)](https://github.com/linebender/vello_svg/actions)

</div>

> [!WARNING]
> The goal of this crate is to provide decent coverage of the (large) SVG spec, up to what vello will support, for use in interactive graphics. If you are looking for a correct SVG renderer, see [resvg](https://github.com/RazrFalcon/resvg). See [vello](https://github.com/linebender/vello) for more information about limitations.

## Examples

### Native

```shell
cargo run -p with_winit
```

You can also load an entire folder or individual files.

```shell
cargo run -p with_winit -- examples/assets
```

### Web

Because Vello relies heavily on compute shaders, we rely on the emerging WebGPU standard to run on the web.
Until browser support becomes widespread, it will probably be necessary to use development browser versions (e.g. Chrome Canary) and explicitly enable WebGPU.

This uses [`cargo-run-wasm`](https://github.com/rukai/cargo-run-wasm) to build the example for web, and host a local server for it

```shell
# Make sure the Rust toolchain supports the wasm32 target
rustup target add wasm32-unknown-unknown

# The binary name must also be explicitly provided as it differs from the package name
cargo run_wasm -p with_winit --bin with_winit_bin
```

There is also a web demo [available here](https://linebender.github.io/vello_svg) on supporting web browsers.

> [!WARNING]
> The web is not currently a primary target for Vello, and WebGPU implementations are incomplete, so you might run into issues running this example.

## License

This project is licensed under your choice of [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) licenses, with [MPL 2.0](LICENSE-MPL).
