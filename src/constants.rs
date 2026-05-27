// Perplexity endpoints
pub const BASE_URL: &str = "https://www.perplexity.ai";
pub const SSE_URL: &str = "https://www.perplexity.ai/rest/sse/perplexity_ask";
pub const AUTH_SESSION_URL: &str = "https://www.perplexity.ai/api/auth/session";
pub const MODELS_CONFIG_URL: &str = "https://www.perplexity.ai/rest/models/config";

// Perplexity request defaults
pub const PERPLEXITY_VERSION: &str = "2.18";
pub const APP_CLIENT: &str = "mweb";

// Application directories
pub const CLOUNAR_DIR: &str = ".clounar";
pub const CLAUDE_DIR: &str = ".claude";

// Embedded files
pub const EXTRACT_COOKIES_PY: &str = include_str!("../extract_cookies.py");
pub const CLAUDE_SETTINGS: &str = include_str!("../settings.json.example");
pub const DEFAULT_CONFIG: &str = include_str!("../config.toml");
pub const DEFAULT_IGNORE: &str = include_str!("../.default_ignore");

// Workflow
pub const MAX_RETRIES: u32 = 3;
pub const SEARCH_DELAY_MS: u64 = 0;
pub const MAX_CLASSIFIER_RETRIES: u8 = 1;
pub const MAX_TOOL_RETRIES: u8 = 2;
pub const MAX_EDIT_RETRIES: u8 = 1;

// Truncation lengths
pub const TITLE_TRUNCATE_LEN: usize = 40;
pub const HISTORY_LOOKUP_LIMIT: usize = 5;
pub const ARTIFACT_NAME_SNIPPET_LEN: usize = 20;

// Context UUID keys
pub const CTX_KEY_PLANNER: &str = "planner";
pub const CTX_KEY_RESPOND: &str = "respond";
pub const CTX_KEY_WEB_SEARCH: &str = "websearch";

// SSE stream tokens
pub const SSE_EVENT_END: &str = "end_of_stream";
pub const SSE_STEP_FINAL: &str = "FINAL";
pub const SSE_USAGE_ASK_TEXT: &str = "ask_text";

// Search request values
pub const SEARCH_FOCUS_INTERNET: &str = "internet";
pub const SEARCH_FOCUS_WRITING: &str = "writing";
pub const SEARCH_SOURCE_WEB: &str = "web";
pub const SEARCH_SOURCE_NONE: &str = "none";
pub const SUPPORTED_BLOCK_USE_CASES: &[&str] = &[];
pub const SUPPORTED_FEATURES: &[&str] = &[];
pub const SEARCH_PROMPT_SOURCE: &str = "user";
pub const SEARCH_QUERY_SOURCE_HOME: &str = "home";
pub const SEARCH_QUERY_SOURCE_FOLLOWUP: &str = "followup";
pub const SEARCH_FOLLOWUP_SOURCE: &str = "link";

// Models config request
pub const MODELS_CONFIG_SCHEMA: &str = "v1";
pub const MODELS_CONFIG_SOURCE: &str = "default";
pub const MODELS_REQUEST_REASON: &str = "use-preferred-search-models";

// Python interpreter
pub const PYTHON_BIN: &str = "python3";

// File names
pub const CONFIG_FILE: &str = "config.toml";
pub const CLAUDE_SETTINGS_FILE: &str = "settings.json";
pub const CACHE_FILE: &str = "compressed_tools.json";
pub const DEFAULT_IGNORE_FILE: &str = ".default_ignore";

// API routes
pub const ROUTE_MESSAGES: &str = "/v1/messages";
pub const ROUTE_MODELS: &str = "/v1/models";

// Tool result caps
pub const TOOL_RESULT_PREVIEW_LEN: usize = 500;
pub const TOOL_RESULT_SNIPPET_LEN: usize = 300;
pub const RETRY_DELAY_MS: u64 = 3000;

// SSE channel buffer
pub const SSE_CHANNEL_BUF: usize = 64;

// System prompt prefixes
pub const SYS_PREFIX_CWD: &str = "Primary working directory:";
pub const SYS_PREFIX_PLATFORM: &str = "Platform:";
pub const SYS_PREFIX_SHELL: &str = "Shell:";
