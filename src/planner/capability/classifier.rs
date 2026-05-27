use serde_json::Value;

use crate::{
    bridge::{ToolStatus, TOOL_MAP},
    planner::{CapabilityKind, ToolCapability},
};

struct CapabilityRule {
    includes: &'static [&'static str],
    excludes: &'static [&'static str],
    capability: ToolCapability,
}

impl CapabilityRule {
    fn matches(&self, has: &impl Fn(&str) -> bool) -> bool {
        self.includes.iter().all(|p| has(p)) && self.excludes.iter().all(|p| !has(p))
    }
}

impl TryFrom<&Value> for ToolCapability {
    type Error = ();

    fn try_from(tool: &Value) -> Result<Self, Self::Error> {
        if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
            if let Some(entry) = TOOL_MAP
                .iter()
                .find(|e| e.kind.as_ref().eq_ignore_ascii_case(name))
            {
                if entry.status == ToolStatus::Excluded {
                    return Err(());
                }
                if entry.status == ToolStatus::Override {
                    let produces: &'static [&'static str] = match entry.produces {
                        Some((ref_name, _)) => match ref_name {
                            "search_results" => &["search_results"],
                            "command_output" => &["command_output"],
                            "file_content" => &["file_content"],
                            _ => &[],
                        },
                        None => &[],
                    };
                    return Ok(ToolCapability {
                        kind: entry.capability.clone().unwrap(),
                        produces,
                        requires: &[],
                    });
                }
            }
        }

        let props = tool
            .get("input_schema")
            .and_then(|s| s.get("properties"))
            .and_then(|p| p.as_object());

        let has = |key: &str| props.is_some_and(|p| p.contains_key(key));

        const RULES: &[CapabilityRule] = &[
            CapabilityRule {
                includes: &["notebook_path", "new_source"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Edit,
                    produces: &[],
                    requires: &[],
                },
            },
            CapabilityRule {
                includes: &["old_string", "new_string"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Edit,
                    produces: &["file_content"],
                    requires: &["file_content"],
                },
            },
            CapabilityRule {
                includes: &["file_path", "content"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Write,
                    produces: &["file_content"],
                    requires: &[],
                },
            },
            CapabilityRule {
                includes: &["file_path"],
                excludes: &["old_string", "content"],
                capability: ToolCapability {
                    kind: CapabilityKind::Read,
                    produces: &["file_content"],
                    requires: &[],
                },
            },
            CapabilityRule {
                includes: &["command"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Execute,
                    produces: &["command_output"],
                    requires: &[],
                },
            },
            CapabilityRule {
                includes: &["url"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Read,
                    produces: &["fetched_content"],
                    requires: &[],
                },
            },
            CapabilityRule {
                includes: &["query"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Search,
                    produces: &["search_results"],
                    requires: &[],
                },
            },
            CapabilityRule {
                includes: &["questions"],
                excludes: &[],
                capability: ToolCapability {
                    kind: CapabilityKind::Interact,
                    produces: &["user_response"],
                    requires: &[],
                },
            },
        ];

        RULES
            .iter()
            .find(|r| r.matches(&has))
            .map(|r| r.capability.clone())
            .ok_or(())
    }
}
