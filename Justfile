# Simple helpers for this repo.

tokens:
    context-gather --no-clipboard src/ 2>&1 | tail -n 1

fmt:
    cargo fmt

clippy:
    cargo clippy

test:
    cargo test

verify:
    cargo fmt
    cargo clippy
    cargo test
