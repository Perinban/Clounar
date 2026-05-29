<div align="center">

# Clounar

**A local bridge that lets you use your Perplexity Pro subscription as the model backend for Claude Code —
with real-time web search on every response, at zero API cost.**

[![License: Educational](https://img.shields.io/badge/license-Educational%20Only-red.svg)](LICENSE)
[![CI](https://github.com/perinban/clounar/actions/workflows/ci.yml/badge.svg)](https://github.com/perinban/clounar/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)

</div>

---

## How it works

```
Claude Code → ~/.clounar (bridge) → Perplexity Pro (sonar / experimental / ...)
```

Claude Code sends model requests to clounar via `ANTHROPIC_BASE_URL`. clounar translates them into Perplexity queries using your browser session cookies — no API key needed. Claude Code still handles all tools locally (file edits, bash, git). clounar only handles model inference.

---

## Requirements

- Rust 1.80+
- Python 3 with `browser_cookie3`
- A Perplexity Pro account with an active session in Chrome

```bash
pip install browser_cookie3
```

---

## Quick start

> **Note:** This project is for educational and research purposes only. It is not intended for production use.

```bash
git clone https://github.com/perinban/clounar
cd clounar
cargo build --release
./target/release/clounar
```

On first run, clounar will:

- Create `~/.clounar/config.toml` with defaults (only if it doesn't already exist — your edits are never overwritten)
- Create `~/.clounar/.default_ignore` with sensible file-index exclusions (only if it doesn't already exist)
- Create `~/.claude/settings.json` pointing Claude Code to the local server (only if it doesn't already exist — existing Claude Code settings are left untouched)
- Extract your Perplexity session cookies automatically from Chrome

Then launch Claude Code as normal — all model inference routes through Perplexity.

---

## Configuration

Config lives at `~/.clounar/config.toml`:

```toml
[perplexity]
default_mode = "concise"
default_model = "experimental"
incognito = true

[server]
host = "127.0.0.1"
port = 8081
log_level = "info"

# -------- PROMPTS --------

[prompts]
compress = "..."
args = "..."
tool_result = "..."
intent_classify = "..."
hash_select = "..."
web_search = "..."
```

All six prompt keys are multiline string templates embedded directly in `config.toml`. You can reword, retone, or reorder them freely. clounar validates all required placeholders are present at startup and will refuse to start if any are missing — it will print exactly which placeholder is absent.

Changes take effect on the next restart; no recompile is needed.

See [docs/configuration.md](docs/configuration.md) for all options, placeholder reference, and the `.default_ignore` file.

---

## Debugging

```bash
# Full debug output
RUST_LOG=clounar=debug ./target/release/clounar

# Target a specific module
RUST_LOG=clounar::workflow::result=debug ./target/release/clounar
RUST_LOG=clounar::perplexity=debug ./target/release/clounar
RUST_LOG=clounar::server::messages=debug ./target/release/clounar
```

`RUST_LOG` takes precedence over `log_level` in `config.toml`.

---

## Notes

- **Session expiry** — if you get auth errors, re-open Perplexity in Chrome and restart clounar. Cookies are re-extracted automatically on every startup.
- **Port conflict** — if the configured port is busy, clounar will find a free port and ask permission to update `~/.claude/settings.json` automatically.
- **Cloudflare** — Perplexity occasionally blocks requests at the handshake level. clounar retries 3 times automatically.
- **Custom config path** — pass a config path as the first argument: `./clounar /path/to/config.toml`

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports and feature requests go through [GitHub Issues](https://github.com/perinban/clounar/issues).

---

## License

Available for educational and research purposes only. See [LICENSE](LICENSE).