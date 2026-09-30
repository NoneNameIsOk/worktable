#!/bin/sh
# Source this only when using the optional project-local Rust installation.
export CARGO_HOME="$(pwd)/.tools/cargo"
export RUSTUP_HOME="$(pwd)/.tools/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
