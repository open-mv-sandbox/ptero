## Pterodactil

Dacti/Daicon related projects incubation workspace.

This used to contain a daicon implementation, but this has since been moved to the daicon repository.

## Dacti Viewer

To explore the development of dacti, this repository contains a reference viewer implementation.
This viewer may be eventually split out and renamed to become its own application in the future.

## Building Standalone JS Library

This project uses wasm-bindgen to create the JS bindings around the WebAssembly based package.
To generate the package for use in projects, run `wasm-pack build` in the JS crate's directory.
See the [wasm-pack documentation](https://rustwasm.github.io/docs/wasm-pack/) for more information.

You do not need to build the library manually for the web example.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License (Expat) ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
