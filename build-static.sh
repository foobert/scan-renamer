#!/bin/bash
# requires: rustup target add x86_64-unknown-linux-musl
cargo build --release --target=x86_64-unknown-linux-musl
ls -l target/x86_64-unknown-linux-musl/release/scan-renamer
