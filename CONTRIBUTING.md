# Contributing

This project is available for educational and research purposes only and is not open for general contributions.

## Bug Reports

If you find a bug, you are welcome to open an issue describing:
- What you expected to happen
- What actually happened
- Relevant log output (redact any personal paths or credentials)

## Feature Requests

Feature requests are welcome as issues. Note that this is a personal project and there is no obligation to implement requests.

## Note on Third-Party Services

This software interfaces with Anthropic Claude Code and Perplexity AI. Any issues related to those services' behaviour or terms of service are out of scope.

## Reference Files

The following files in the repo root are provided for reference and study purposes:

- **`claude_tools.json`** — a raw captured request from Claude Code to the server. Contains the full tool list exactly as Claude Code sends it, useful as a reference for understanding the tool schema and request structure.
- **`compressed_tools.json`** — a pre-built cache of compressed tool descriptions produced by the `compress` prompt. On first run, clounar compresses each tool via Perplexity and writes the result to `~/.clounar/compressed_tools.json`. You can copy this file there manually to skip that cost: `cp compressed_tools.json ~/.clounar/compressed_tools.json`.
- **`settings.json.example`** — the exact content written to `~/.claude/settings.json` on first run. Review this to understand how Claude Code is pointed at the local server before running clounar for the first time.
- **`.default_ignore`** — the default file-index exclusion patterns embedded in the binary and written to `~/.clounar/.default_ignore` on first run. Edit the copy in `~/.clounar/` to customise exclusions; the repo-root file is for reference only.

## Build & Debug (for personal study)

```bash
cargo build --release
RUST_LOG=clounar=debug ./target/release/clounar
```

See [docs/configuration.md](docs/configuration.md) for all log targets.