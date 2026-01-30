# Repository Guidelines

## Project Structure & Module Organization
Source lives under `src/` and is intentionally minimal:
- `src/main.rs` - CLI entry point; `src/lib.rs` - shared library API for tests
- `src/cli.rs`, `src/config.rs`, `src/constants.rs` - CLI args, config, defaults
- `src/pipeline.rs` - orchestration of gathering, chunking, output
- `src/context/` - file discovery, chunking, headers, XML
- `src/tokenizer.rs` - token counting
- `src/io/` - clipboard and I/O helpers; `src/ui/` - TUI
Tests are in `tests/` (integration-style); `tests/common.rs` holds helpers.
Keep new code within these modules unless there is a clear reason to expand.

Keep `src/` under ~24k tokens. To count, run `just tokens`
(which executes `context-gather --no-clipboard src/ 2>&1 | tail -n 1`) and read the final summary
line (e.g., `OK 17 files • 23800 tokens • 1 chunk • copied=none`).

Before implementing changes, propose a concrete plan with estimated deltas
(tokens and lines added/removed). Wait for explicit approval of the plan and
token budget before editing.

## Build, Test, and Development Commands
- `just tokens` prints the `src/` token summary line (captured from stderr).
- `just fmt` runs Rustfmt.
- `just clippy` runs lint checks.
- `just test` runs the full test suite.
- `just verify` runs format, lint, and tests in order.
- `cargo build` compiles the CLI.
- `cargo run -- .` runs against the current directory (use `--help` for flags).
- `cargo test` runs all integration and property tests.
- `cargo fmt` and `cargo clippy` format and lint the codebase.

## Coding Style & Naming Conventions
Standard Rust naming applies: `CamelCase` for types, `snake_case` for functions
and modules, `SCREAMING_SNAKE_CASE` for constants. Rustfmt enforces a 100-column
max width and reordered imports/modules; run `cargo fmt` before committing.

## Testing Guidelines
Tests live under `tests/` and use `assert_cmd`, `predicates`, and `proptest`.
Prefer deterministic fixtures and reuse helpers in `tests/common.rs`. Run
`cargo test` locally before opening a PR.

## Commit & Pull Request Guidelines
Commit history uses short, imperative, sentence-case subjects (e.g., "Fix ...",
"Refactor ...") without prefixes; keep subjects under ~72 characters. PRs should
summarize behavior changes, note relevant flags (e.g., `--git-info`,
`--exclude-paths`), and list the tests you ran.

## Security & Configuration Tips
Be cautious about sensitive files in gathered output. Use `--exclude-paths` and
`--max-size`, and prefer `--no-clipboard` or `--stdout` when clipboard use is
undesirable.
