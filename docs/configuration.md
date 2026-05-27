# Configuration

clounar is configured via `config.toml`, located at `~/.clounar/config.toml`.

## Full example

```toml
[perplexity]
default_mode = "copilot"
default_model = "experimental"
default_source = "default"
incognito = true

[server]
host = "127.0.0.1"
port = 8081
log_level = "info"

# -------- PROMPTS --------

[prompts]
compress = """..."""
agent_select = """..."""
args = """..."""
tool_result = """..."""
plan = """..."""
hash_select = """..."""
```

---

## `[perplexity]`

| Key | Type | Default | Description |
|---|---|---|---|
| `default_mode` | string | `"copilot"` | Perplexity query mode. Options: `copilot`, `concise`, `detailed` |
| `default_model` | string | `"experimental"` | Model to use. Run `/v1/models` to see available models for your tier |
| `default_source` | string | `"default"` | Source filter for search results |
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

Each key is a multiline string template used to instruct the model at a specific stage of the workflow. Templates support named placeholders — clounar substitutes them at runtime. You can reword instructions, change tone, or reorder placeholders freely.

**Clounar validates all required placeholders are present at startup and will refuse to start if any are missing.**

| Key | Required placeholders | Purpose |
|---|---|---|
| `compress` | `{user_request}`, `{combined}`, `{available_tools}` | Compresses a tool description into rules and preconditions |
| `agent_select` | `{tools_list}`, `{user_query}`, `{combined}` | Selects which subagent type to use for the request |
| `args` | `{user_query}`, `{env_context}`, `{prior_context}`, `{rules}`, `{schema}` | Generates tool call arguments as JSON |
| `tool_result` | `{user_query}`, `{tool_name}`, `{tool_input}`, `{tool_result}` | Formats the final response after a tool returns |
| `plan` | `{tools_section}`, `{user_query}` | Plans the sequence of tools needed to complete the request |
| `hash_select` | `{user_query}`, `{candidates}` | Matches the current request to a prior task for context reuse |

---

## Log level

`log_level` in `config.toml` sets the default. For targeted debugging without restarting, use `RUST_LOG`:

```bash
RUST_LOG=clounar=debug ./clounar
RUST_LOG=clounar::workflow::result=debug ./clounar
RUST_LOG=clounar::perplexity=debug ./clounar
```

`RUST_LOG` takes precedence over `config.toml` when set.