use std::{fs, io::Read};

use serde_json::{json, Map, Value};

use crate::error::AppError;

pub fn read_json_arg(
    service: &'static str,
    operation: &'static str,
    json_arg: Option<&str>,
) -> Result<Value, AppError> {
    let Some(json_arg) = json_arg else {
        return Ok(Value::Object(Map::new()));
    };

    let text = if json_arg == "-" {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text)?;
        text
    } else if json_arg.trim_start().starts_with('{') || json_arg.trim_start().starts_with('[') {
        json_arg.to_string()
    } else {
        fs::read_to_string(json_arg).map_err(|err| {
            AppError::invalid_input(
                service,
                operation,
                format!("failed to read JSON file {json_arg}: {err}"),
            )
        })?
    };

    serde_json::from_str(&text).map_err(|err| {
        AppError::invalid_input(service, operation, format!("invalid JSON payload: {err}"))
    })
}

pub fn set_string(value: &mut Value, key: &str, field: &Option<String>) {
    if let Some(field) = field {
        ensure_object(value).insert(key.to_string(), Value::String(field.clone()));
    }
}

pub fn set_u64(value: &mut Value, key: &str, field: Option<u64>) {
    if let Some(field) = field {
        ensure_object(value).insert(key.to_string(), json!(field));
    }
}

pub fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .expect("value was just made an object")
}

/// Build an ADF document from plain text, keeping its line structure.
///
/// Blank lines separate paragraphs; single newlines inside a paragraph become
/// `hardBreak` nodes. ADF text nodes do not render embedded newlines, so a
/// multi-line `--body` would otherwise collapse into one run of text. An empty
/// input still yields one empty paragraph, as before.
pub fn minimal_adf(text: &str) -> Value {
    let normalized = text.replace("\r\n", "\n");
    let mut paragraphs: Vec<Value> = normalized
        .split("\n\n")
        .map(|block| block.trim_matches('\n'))
        .filter(|block| !block.is_empty())
        .map(|block| {
            let mut content = Vec::new();
            for (index, line) in block.split('\n').enumerate() {
                if index > 0 {
                    content.push(json!({ "type": "hardBreak" }));
                }
                if !line.is_empty() {
                    content.push(json!({ "type": "text", "text": line }));
                }
            }
            json!({ "type": "paragraph", "content": content })
        })
        .collect();
    if paragraphs.is_empty() {
        paragraphs.push(json!({
            "type": "paragraph",
            "content": [{ "type": "text", "text": text }]
        }));
    }
    json!({
        "type": "doc",
        "version": 1,
        "content": paragraphs
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_json() {
        let value = read_json_arg("test", "parse", Some(r#"{"a":1}"#)).unwrap();
        assert_eq!(value["a"], 1);
    }

    #[test]
    fn creates_minimal_adf_document() {
        let value = minimal_adf("hello");
        assert_eq!(value["type"], "doc");
        assert_eq!(value["content"].as_array().unwrap().len(), 1);
        assert_eq!(value["content"][0]["content"][0]["text"], "hello");
    }

    #[test]
    fn minimal_adf_keeps_paragraphs_and_line_breaks() {
        let text = "Prompt for the code reviewer — hand this over as-is:\n\n*Goal*\nShip it\n\n- [ ] one (AC #1)\n- [ ] two (AC #2)";
        let value = minimal_adf(text);
        let paragraphs = value["content"].as_array().unwrap();
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(
            paragraphs[0]["content"][0]["text"],
            "Prompt for the code reviewer — hand this over as-is:"
        );
        let second = paragraphs[1]["content"].as_array().unwrap();
        assert_eq!(second[0]["text"], "*Goal*");
        assert_eq!(second[1]["type"], "hardBreak");
        assert_eq!(second[2]["text"], "Ship it");
        let third = paragraphs[2]["content"].as_array().unwrap();
        assert_eq!(third.len(), 3);
        assert_eq!(third[2]["text"], "- [ ] two (AC #2)");
    }

    #[test]
    fn minimal_adf_handles_crlf_and_empty_input() {
        let value = minimal_adf("a\r\nb\r\n\r\nc");
        assert_eq!(value["content"].as_array().unwrap().len(), 2);
        assert_eq!(value["content"][0]["content"][1]["type"], "hardBreak");
        let empty = minimal_adf("");
        assert_eq!(empty["content"].as_array().unwrap().len(), 1);
        assert_eq!(empty["content"][0]["content"][0]["text"], "");
    }
}
