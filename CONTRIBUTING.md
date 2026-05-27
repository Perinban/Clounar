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

## Build & Debug (for personal study)

```bash
cargo build --release
RUST_LOG=clounar=debug ./target/release/clounar
```

See [docs/configuration.md](docs/configuration.md) for all log targets.