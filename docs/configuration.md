# Configuration

clounar stores all user-editable files under `~/.clounar/`:

| File | Created | Purpose |
|---|---|---|
| `config.toml` | First run (if absent) | All configuration: server, perplexity, and prompt templates |
| `.default_ignore` | First run (if absent) | `.gitignore`-format patterns to exclude from the file index when a project has no `.gitignore` |

Both files are written **only if they don't already exist** — clounar never overwrites your edits. Changes to either file take effect on the next restart; no recompile is needed.

### `.default_ignore`

This file controls which paths are excluded when clounar builds its file index of your working directory. It uses standard `.gitignore` syntax. It is only applied when the current project has **no `.gitignore`** of its own — if a `.gitignore` is present, `.default_ignore` is skipped entirely.

Edit `~/.clounar/.default_ignore` to add your own patterns (e.g. `node_modules/`, `*.log`, `dist/`).

---

## `config.toml`

Located at `~/.clounar/config.toml`.

## Full example

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
compress = """..."""
args = """..."""
tool_result = """..."""
intent_classify = """..."""
hash_select = """..."""
web_search = """..."""
```

---

## `[perplexity]`

| Key | Type | Default | Description |
|---|---|---|---|
| `default_mode` | string | `"concise"` | Perplexity query mode. Passed directly to the Perplexity API — run `/v1/models` or check your Perplexity account for supported values |
| `default_model` | string | `"experimental"` | Model to use. clounar logs all available models for your subscription tier on every startup — use that list to pick a value |
| `incognito` | bool | `true` | Whether to use incognito mode (no history saved on Perplexity) |

---

## `[server]`

| Key | Type | Default | Description |
|---|---|---|---|
| `host` | string | `"127.0.0.1"` | Host to bind to |
| `port` | u16 | `8081` | Port to bind to. If busy, clounar will prompt to use a free port automatically |
| `log_level` | string | `"info"` | Log level. Options: `error`, `warn`, `info`, `debug`, `trace` |

---

## `[prompts]`

Each key is a multiline string template embedded in `config.toml` used to instruct the model at a specific stage of the workflow. Templates support named placeholders — clounar substitutes them at runtime. You can reword instructions, change tone, or reorder placeholders freely.

**clounar validates all required placeholders are present at startup and will refuse to start if any are missing — it prints exactly which placeholder is absent.**

| Key | Required placeholders | Purpose |
|---|---|---|
| `compress` | `{tool_name}`, `{combined}` | Extracts structured capability/limit facts from a tool description |
| `args` | `{user_query}`, `{env_context}`, `{file_artifacts}`, `{resolved_args}`, `{rules}`, `{schema}` | Generates tool call arguments as JSON conforming to the tool's input schema |
| `tool_result` | `{user_query}`, `{tool_name}`, `{tool_input}`, `{tool_result}` | Formats the final response after a tool returns its output |
| `intent_classify` | `{user_query}` | Classifies the user's intent and extracts action type and target path |
| `hash_select` | `{user_query}`, `{candidates}` | Matches the current request to a prior task for context reuse |
| `web_search` | `{query}` | Drives web searches for error diagnosis and live lookups |

---

## Log level

`log_level` in `config.toml` sets the default. For targeted debugging without restarting, use `RUST_LOG`:

```bash
RUST_LOG=clounar=debug ./clounar
RUST_LOG=clounar::workflow::result=debug ./clounar
RUST_LOG=clounar::perplexity=debug ./clounar
```

`RUST_LOG` takes precedence over `config.toml` when set.