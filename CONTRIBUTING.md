# Contributing

Thanks for improving `context-gather`. The project is intentionally small, so
changes should preserve predictable CLI behavior and keep source size under the
budget documented in `AGENTS.md`.

## Development Setup

Install the Rust toolchain selected by `rust-toolchain.toml`, then run:

```bash
cargo build
cargo test
```

Common checks:

```bash
just tokens
cargo fmt --check
cargo clippy
cargo test
```

`just verify` runs formatting, linting, and tests. Use `cargo fmt --check` when
you only want to inspect formatting without rewriting files.

## Pull Requests

- Keep behavior changes explicit in the PR summary.
- Mention affected flags such as `--stdout`, `--no-clipboard`, `--chunk-size`,
  `--multi-step`, `--git-info`, `--escape-xml`, or `--exclude-paths`.
- Add focused tests for CLI behavior, chunking behavior, or TUI event handling.
- List the verification commands you ran.
