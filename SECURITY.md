# Security Policy

`context-gather` is a local CLI that reads files from paths you provide and can
copy gathered content to the system clipboard. Treat gathered output as
sensitive when the selected files may contain credentials, private notes, or
proprietary source code.

## Reporting Issues

Please report security issues privately to the maintainer before opening a
public issue. Include the affected version or commit, a minimal reproduction,
and the impact you believe the issue has.

## Supported Versions

This project is pre-1.0. Security fixes target the current `main` branch until
formal release support is defined.

## Safe Usage

- Use `--stdout --no-clipboard` when you want to inspect output before sharing.
- Use `--exclude-paths` for secrets such as `.env`, private keys, and tokens.
- Use `--max-size` and `--select` to limit accidental inclusion.
- Use `--escape-xml` when downstream tooling needs parseable XML-like content.
