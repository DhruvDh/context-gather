# context-gather

`context-gather` is a Rust CLI for collecting selected text files into an
LLM-friendly XML-like context bundle. It can copy the bundle to the clipboard,
print it to stdout, count tokens, split large contexts into chunks, and expose a
TUI for selecting files before output.

The tool is built for code-review and research workflows where the important
property is not just "read files," but "produce a predictable, inspectable
payload that can be pasted into a model without silently losing structure."

## Install And Build

This repository uses Rust 1.85 or newer because the crate is configured for
Edition 2024:

```bash
rustup toolchain install stable
cargo build
```

From this repository, install the local binary with:

```bash
cargo install --path .
```

If the crate is published in your environment, `cargo install context-gather`
also works.

## Quick Start

Gather the current directory and copy the result to the clipboard:

```bash
context-gather .
```

Print the XML to stdout and avoid clipboard access:

```bash
context-gather --stdout --no-clipboard .
```

Gather a few paths:

```bash
context-gather README.md src tests
```

Use a glob:

```bash
context-gather 'src/**/*.rs'
```

Open the file-selection TUI:

```bash
context-gather --select .
```

## Output And Clipboard Behavior

By default, `context-gather` copies the generated context to the clipboard and
prints a human-readable summary to stderr:

```text
OK 22 files • 21164 tokens • 1 chunk • copied=0
```

Use `--stdout` to print the XML payload to stdout. Summaries, warnings, and
errors stay on stderr so stdout remains machine-readable.

If clipboard access fails and `--stdout` is not set, the command exits with an
error. If `--stdout` is set, clipboard failure is only a warning. Use
`--no-clipboard` when clipboard access is undesirable or unavailable.

## Paths, Globs, And Excludes

Arguments are file paths, directory paths, or glob patterns. Existing literal
paths take precedence over glob parsing, so filenames containing characters such
as `[` or `*` are accepted when the path exists.

For directory arguments, the tool recursively discovers files with the
`ignore` crate. Standard filters are enabled, so `.gitignore` rules, hidden
files, and common ignored directories are respected.

Exclude patterns are matched against paths relative to the current working
directory and against absolute paths. Use `**` when a pattern must span
directories:

```bash
context-gather --exclude-paths 'target/**' --exclude-paths '**/*.lock' .
```

Files larger than `--max-size` are skipped. The default is 1 MiB:

```bash
context-gather --max-size 262144 .
```

Invalid UTF-8 files are treated as binary and skipped with a warning.

## XML-Like Output And Escaping

File contents are raw by default, because raw code is usually easier for a model
to read. XML attributes are always escaped when needed.

Raw output is intentionally XML-like, not guaranteed parseable XML. If raw file
contents contain context wrapper markers such as `</file-contents>`, the tool
warns that the structure may be ambiguous.

Use `--escape-xml` when a downstream parser needs escaped text content:

```bash
context-gather --stdout --no-clipboard --escape-xml src/main.rs
```

The root element is `<shared-context>`. Non-chunked output includes a
`<file-map>` followed by `<folder>` and `<file-contents>` elements.

## Chunked Context

Use `--chunk-size` to split output into token-bounded chunks:

```bash
context-gather --stdout --no-clipboard --chunk-size 39000 .
```

Chunk `0` contains a `<shared-context-header>` with file metadata and
instructions. Later chunks contain `<context-chunk>` elements. Files are kept
intact when possible; oversized files are split by line and marked with
`part="p/N"`.

Print or copy one chunk by index:

```bash
context-gather --stdout --no-clipboard --chunk-size 39000 --chunk-index 2 .
```

Use `--chunk-index -1` to build and summarize chunks without printing or
copying any chunk.

## Streaming Mode

Use `--stream` with `--chunk-size` for an interactive chunk-copy REPL:

```bash
context-gather --stream --chunk-size 39000 .
```

Streaming commands are:

- press Enter to move to the next chunk,
- type a chunk number to jump,
- type `q` to quit.

`--interactive` is shorthand for file selection and, when `--chunk-size` is
provided, streaming:

```bash
context-gather --interactive --chunk-size 39000 .
```

## Multi-Step Mode

Multi-step mode copies or prints only the header first, then lets you request
files on demand by id, path, or glob:

```bash
context-gather --multi-step --stdout --no-clipboard .
```

At the prompt, enter `2`, `src/main.rs`, `*.rs`, or `q` to quit. Multi-step mode
cannot be combined with `--chunk-size`.

## Git Metadata

Pass `--git-info` with chunked or multi-step output to include the current
branch, recent commit subjects, and changed filenames in the header:

```bash
context-gather --stdout --no-clipboard --chunk-size 39000 --git-info .
```

Changed files are compared against the first available base in this order:
the upstream branch, `origin/HEAD`, local `main`, local `master`,
`origin/main`, then `origin/master`. If no base is available, the header says
that changed files are unavailable instead of inventing a default.

`--git-info` does not include full diff bodies.

## Tokenizer

Token counts use a shared `tiktoken-rs` tokenizer. The default model name is
`gpt-5.2`, which maps to `o200k_base`; `gpt-5` is also accepted as an alias.
Models known directly to `tiktoken-rs` are accepted. Unsupported names fail
fast so token-count mistakes are visible.

Override the model with either the CLI flag or environment variable:

```bash
context-gather --tokenizer-model gpt-5.2 .
CG_TOKENIZER_MODEL=gpt-5.2 context-gather .
```

The CLI flag takes precedence over `CG_TOKENIZER_MODEL`.

Use `--no-model-context` to suppress token summaries and model-context warnings,
or `--model-context` to set a different warning threshold.

## Privacy And Sensitive Files

Always inspect what you are about to send to a model. The tool respects standard
ignore rules, but it cannot know which files are sensitive in your project.

Useful safeguards:

```bash
context-gather --stdout --no-clipboard --exclude-paths '.env' --exclude-paths '**/*.pem' .
context-gather --select .
context-gather --max-size 262144 .
```

Prefer `--stdout --no-clipboard` when you want to inspect or pipe output without
touching the system clipboard.

## Contributing And Security

This project is licensed under the MIT License. See `CONTRIBUTING.md` for local
development expectations and `SECURITY.md` for reporting and safe-usage notes.

## Development

Common commands:

```bash
just tokens
cargo fmt --check
cargo clippy
cargo test
```

`just verify` runs `cargo fmt`, `cargo clippy`, and `cargo test`. Use
`cargo fmt --check` for review-only work when you do not want the formatter to
rewrite files.
