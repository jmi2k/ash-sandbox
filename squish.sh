#!/bin/sh

# Bring the binary size to the minimum.

target=x86_64-unknown-linux-gnu

cargo +nightly build \
    -Z build-std=std,panic_abort \
    -Z build-std-features=panic_immediate_abort \
    --target $target \
    --release \
    || exit $?

echo >&2
echo This binary is almost impossible to debug, avoid distributing it! >&2
