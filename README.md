# vello_svg

[![Linebender Zulip](https://img.shields.io/badge/Linebender-%23gpu-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/197075-gpu)
[![dependency status](https://deps.rs/repo/github/linebender/vello/status.svg)](https://deps.rs/repo/github/linebender/vello)
[![MIT/Apache 2.0+MPL 2.0](https://img.shields.io/badge/license-MIT%2FApache+MPL2-blue.svg)](#license)
[![Build status](https://github.com/linebender/vello/workflows/CI/badge.svg)](https://github.com/linebender/vello/actions)
<!-- [![Crates.io](https://img.shields.io/crates/v/vello.svg)](https://crates.io/crates/vello) -->
<!-- [![Docs](https://docs.rs/vello/badge.svg)](https://docs.rs/vello) -->

An integration to parse SVG files and render them with [Vello](https://vello.dev).

## Examples

See [vello](https://github.com/linebender/vello) for more information about limitations.

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
