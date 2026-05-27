use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct PromptsConfig {
    pub compress: String,
    pub args: String,
    pub tool_result: String,
    pub intent_classify: String,
    pub hash_select: String,
    pub web_search: String,
}

impl PromptsConfig {
    pub fn validate(&self) -> Result<()> {
        let checks: &[(&str, &str, &[&str])] = &[
            ("compress", &self.compress, &["{tool_name}", "{combined}"]),
            (
                "args",
                &self.args,
                &[
                    "{user_query}",
                    "{env_context}",
                    "{file_artifacts}",
                    "{resolved_args}",
                    "{rules}",
                    "{schema}",
                ],
            ),
            (
                "tool_result",
                &self.tool_result,
                &[
                    "{user_query}",
                    "{tool_name}",
                    "{tool_input}",
                    "{tool_result}",
                ],
            ),
            ("intent_classify", &self.intent_classify, &["{user_query}"]),
            (
                "hash_select",
                &self.hash_select,
                &["{user_query}", "{candidates}"],
            ),
            ("web_search", &self.web_search, &["{query}"]),
        ];
        for (name, template, placeholders) in checks {
            for placeholder in *placeholders {
                if !template.contains(placeholder) {
                    anyhow::bail!(
                        "prompts.{} is missing required placeholder {}",
                        name,
                        placeholder
                    );
                }
            }
        }

        Ok(())
    }
}
