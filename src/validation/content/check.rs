use serde_json::Value;

pub enum TransformCheck {
    Required { old: String, content: String },
    MissingOld,
    NotApplicable,
}

impl TransformCheck {
    pub fn from_args(input: &Value, artifact_refs: &[String], file_content: Option<&str>) -> Self {
        if !artifact_refs.contains(&"file_content".to_string()) {
            return Self::NotApplicable;
        }
        let old = input.get("old_string").and_then(|v| v.as_str());
        let new_exists = input.get("new_string").is_some();
        if old.is_none() && !new_exists {
            return Self::NotApplicable;
        }
        match old {
            Some(old) => Self::Required {
                old: old.to_string(),
                content: Self::strip(file_content.unwrap()),
            },
            None => Self::MissingOld,
        }
    }

    fn strip(s: &str) -> String {
        s.lines()
            .map(|line| {
                let mut parts = line.splitn(2, '\t');
                let prefix = parts.next().unwrap_or("");
                if prefix.chars().all(|c| c.is_ascii_digit()) {
                    parts.next().unwrap_or(line)
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
