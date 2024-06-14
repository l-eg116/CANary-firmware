let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rustVersion = "latest";
  rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
in
pkgs.mkShell {
  name = "rust embed";

  buildInputs = [
    rust
  ] ++ (with pkgs; [
    pkg-config udev
    cargo-binutils
    cargo-expand
    probe-rs
    gcc
    gdb
  ]);
  RUST_BACKTRACE = 1;

  shellHook = ''
    # SHELL=fish code .
  '';
}