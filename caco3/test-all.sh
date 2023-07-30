#!/bin/sh
set -ex

cargo test
cargo +nightly miri test

