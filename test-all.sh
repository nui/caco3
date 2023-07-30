#!/bin/sh
set -ex

cargo test --all --all-features
cargo +nightly miri test

