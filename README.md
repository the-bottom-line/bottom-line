# The Bottom Line

_The Bottom Line_ is a card game created by the Maastricht University aimed at business and economics students to engage with some of the class material. In the game, players try to maximize the value of their company and beat the other players by using economic principles to their advantage.

## Getting Started

Install rust: https://rust-lang.org/tools/install/
Simply installing the stable version will do.

If using nix: `nix develop` or `direnv allow` installs all necessary dependencies.

Then to run, simply run `cargo run`.

## Running Tests

To run the server tests, run `cargo test-server`
To run the game state tests, run `cargo test-game`
To run all tests, run `cargo test --all`

## Exporting Typescript Types for the Frontend

The current frontend is hosted at [github.com/OliDo99/TheBottomLine](https://github.com/OliDo99/TheBottomLine). It uses typescript types exported from the backend using [ts-rs](https://crates.io/crates/ts-rs). To export those types, run

```sh
cargo export-ts
```

The type definitions can be found in `/shared-ts/index.ts`.

## Building WASM responses

There have been some experiments with building WASM to send requests to this backend from the frontend. To use it, install wasm-opt (if you're using nix, skip this step):

```sh
cargo install wasm-opt --locked
```

Then, in a bash shell run

```sh
source build-wasm.sh
```

Functions that allow the creation of all `FrontendRequest`s in about 60kB can be found inside of `wasm-responses/responses`
