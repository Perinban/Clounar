use serde_json::Value;

use crate::bridge::{ArgField, ArgType, ToolKind};

use super::types::ArgValidationError;

type SemanticValidator = fn(&Value) -> Result<(), ArgValidationError>;

static TOOL_SEMANTIC_MAP: &[(ToolKind, SemanticValidator)] = &[
    (ToolKind::NotebookEdit, |args| {
        let cell_id = args["cell_id"].as_str().unwrap_or("").trim();
        if cell_id.is_empty() {
            return Err(ArgValidationError::SemanticViolation(
                "cell_id must not be empty — read the notebook first to get a valid cell ID".into(),
            ));
        }
        Ok(())
    }),
    (ToolKind::Edit, |args| {
        let old = args["old_string"].as_str().unwrap_or("");
        let new = args["new_string"].as_str().unwrap_or("");
        if old.is_empty() {
            return Err(ArgValidationError::SemanticViolation(
                "old_string is empty — Edit requires a non-empty search string".into(),
            ));
        }
        if old == new {
            return Err(ArgValidationError::SemanticViolation(
                "old_string and new_string are identical — edit would be a no-op".into(),
            ));
        }
        let trivial = old.chars().all(|c| "().[]{}\"' \t".contains(c));
        if trivial {
            return Err(ArgValidationError::SemanticViolation(
                "old_string contains only punctuation or whitespace — too vague to match safely"
                    .into(),
            ));
        }
        Ok(())
    }),
];

pub fn validate_args(
    kind: &ToolKind,
    fields: &[ArgField],
    args: &Value,
) -> Result<(), ArgValidationError> {
    let semantic = TOOL_SEMANTIC_MAP
        .iter()
        .find(|(k, _)| k == kind)
        .map(|(_, f)| *f)
        .unwrap_or(|_| Ok(()));
    validate_fields(args, fields, semantic)
}

pub fn validate_fields(
    args: &Value,
    fields: &[ArgField],
    semantic: impl Fn(&Value) -> Result<(), ArgValidationError>,
) -> Result<(), ArgValidationError> {
    for f in fields {
        let (name, kind, is_required) = match f {
            ArgField::Required(name, kind) => (name, kind, true),
            ArgField::Optional(name, kind) => (name, kind, false),
        };
        match args.get(name) {
            None if is_required => return Err(ArgValidationError::MissingRequired((*name).into())),
            None => continue,
            Some(v) => match kind {
                ArgType::String | ArgType::NonEmptyString => {
                    if !v.is_string() {
                        return Err(ArgValidationError::WrongType {
                            field: (*name).into(),
                            expected: "string",
                            got: json_type(v),
                        });
                    }
                    if matches!(kind, ArgType::NonEmptyString)
                        && v.as_str().unwrap_or("").trim().is_empty()
                    {
                        return Err(ArgValidationError::EmptyValue((*name).into()));
                    }
                }
                ArgType::Number => {
                    if !v.is_number() {
                        return Err(ArgValidationError::WrongType {
                            field: (*name).into(),
                            expected: "number",
                            got: json_type(v),
                        });
                    }
                }
                ArgType::Bool => {
                    if !v.is_boolean() {
                        return Err(ArgValidationError::WrongType {
                            field: (*name).into(),
                            expected: "boolean",
                            got: json_type(v),
                        });
                    }
                }
                ArgType::NonEmptyArray => match v.as_array() {
                    None => {
                        return Err(ArgValidationError::WrongType {
                            field: (*name).into(),
                            expected: "array",
                            got: json_type(v),
                        })
                    }
                    Some(arr) if arr.is_empty() => {
                        return Err(ArgValidationError::EmptyValue((*name).into()));
                    }
                    _ => {}
                },
            },
        }
    }
    semantic(args)
}

fn json_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
