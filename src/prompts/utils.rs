use serde_json::Value;

pub(super) fn extract_schema_descriptions(val: &Value, out: &mut Vec<String>) {
    match val {
        Value::Object(map) => {
            if let Some(d) = map
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|d| !d.is_empty())
            {
                out.push(d.to_string());
            }
            for (_, v) in map {
                extract_schema_descriptions(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                extract_schema_descriptions(v, out);
            }
        }
        _ => {}
    }
}

pub(super) fn combined_description(tool: &Value) -> String {
    let description = tool
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut schema_descs = Vec::new();
    if let Some(schema) = tool.get("input_schema") {
        extract_schema_descriptions(schema, &mut schema_descs);
    }
    if schema_descs.is_empty() {
        description.to_string()
    } else {
        format!("{}\n{}", description, schema_descs.join("\n"))
    }
}
