{
  description = "A Rust devshell";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            rust
            pkg-config
            pre-commit
            binaryen
            docker
            # wasm-bindgen-cli    # cargo install -f wasm-bindgen-cli --version 0.2.105
          ];
        };
      }
    );
}
