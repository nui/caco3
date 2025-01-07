#!/bin/sh
set -ex

cargo test --all --all-features
MIRIFLAGS=-Zmiri-ignore-leaks cargo +nightly miri test

