use serde_json::Value;

use crate::anthropic::serialize_segments;

use super::{
    types::{PromptContext, PromptKind},
    utils::combined_description,
};

impl<'a> PromptContext<'a> {
    pub fn build(&self, kind: PromptKind<'a>) -> String {
        let user_query = serialize_segments(self.user_query);
        match kind {
            PromptKind::Compress => {
                let tool = self.tool.expect("PromptKind::Compress requires tool");
                let combined = combined_description(tool);
                let tool_name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("");

                self.prompts
                    .compress
                    .replace("{tool_name}", tool_name)
                    .replace("{combined}", &combined)
            }

            PromptKind::IntentClassify => self
                .prompts
                .intent_classify
                .replace("{user_query}", &user_query),

            PromptKind::Args => {
                let tool = self.tool.expect("PromptKind::Args requires tool");
                let compressed = self
                    .compressed
                    .expect("PromptKind::Args requires compressed");
                let env = self.env.expect("PromptKind::Args requires env");

                let schema = {
                    let mut s = tool
                        .get("input_schema")
                        .cloned()
                        .unwrap_or(Value::Object(Default::default()));
                    if let Some(hint) = self.args_hint {
                        if let Some(props) = s.get_mut("properties").and_then(|p| p.as_object_mut())
                        {
                            for key in hint
                                .as_object()
                                .map(|o| o.keys().cloned().collect::<Vec<_>>())
                                .unwrap_or_default()
                            {
                                props.remove(&key);
                            }
                        }
                    }
                    s
                };

                let env_context = if env.cwd.is_empty() {
                    String::new()
                } else {
                    format!(
                        "Environment:\n- cwd: {}\n- platform: {}\n- shell: {}\n",
                        env.cwd, env.platform, env.shell
                    )
                };

                let rules = {
                    let mut parts = compressed.capabilities.clone();
                    parts.extend(compressed.limits.iter().map(|l| format!("LIMIT: {}", l)));
                    parts.join("\n")
                };

                let resolved_args = match self.args_hint {
                    Some(hint) => format!("Known values (for context):\n{}\n", hint),
                    None => String::new(),
                };

                let mut file_parts: Vec<String> = Vec::new();
                for (ref_name, content) in self.artifact_refs.iter() {
                    if ref_name.contains("file_content") {
                        tracing::debug!(
                            "[prompt] injecting artifact ref={} content_preview={:?}",
                            ref_name,
                            &content[..content.len().min(120)]
                        );
                        file_parts.push(format!(
                            "<artifact ref=\"{}\">\n{}\n</artifact>",
                            ref_name, content
                        ));
                    } else {
                        tracing::debug!(
                            "[prompt] skipping artifact ref={} (passed via context uuid)",
                            ref_name,
                        );
                    }
                }
                let file_artifacts = file_parts.join("\n");

                self.prompts
                    .args
                    .replace("{user_query}", &user_query)
                    .replace("{env_context}", &env_context)
                    .replace("{file_artifacts}", &file_artifacts)
                    .replace("{resolved_args}", &resolved_args)
                    .replace("{rules}", &rules)
                    .replace("{schema}", &schema.to_string())
            }

            PromptKind::ToolResult {
                tool_name,
                tool_input,
                tool_result,
            } => {
                let tool_result_block = format!("<tool_result>\n{}\n</tool_result>", tool_result);
                self.prompts
                    .tool_result
                    .replace("{user_query}", &user_query)
                    .replace("{tool_name}", tool_name)
                    .replace("{tool_input}", &tool_input.to_string())
                    .replace("{tool_result}", &tool_result_block)
            }

            PromptKind::HashSelect { candidates } => self
                .prompts
                .hash_select
                .replace("{user_query}", &user_query)
                .replace("{candidates}", candidates),
        }
    }
}
