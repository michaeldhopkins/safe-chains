//! A call's arguments checked against the tool's schema before anything runs.
//!
//! An undeclared argument is refused with the ones the tool takes, never dropped: a model that
//! passed one meant something by it. A wrong type is called a wrong type, never "missing", since a
//! model told "missing" sends the same value again. Blank text counts as absent, because small
//! models fill optional text with "".

use serde_json::{Map, Value};

pub const NOTHING_DONE: &str = "Nothing was done.";

/// The arguments as text, by name, or the refusal to return instead.
pub fn check(tool: &str, schema: &Value, args: &Value) -> Result<Map<String, Value>, String> {
    let empty = Map::new();
    let props = schema.get("properties").and_then(Value::as_object).unwrap_or(&empty);
    let given = match args {
        Value::Object(map) => map.clone(),
        Value::Null => Map::new(),
        _ => return Err(format!("{tool} takes named arguments, as a JSON object. {NOTHING_DONE}")),
    };
    let takes = || if props.is_empty() { "no arguments".to_string() } else { props.keys().cloned().collect::<Vec<_>>().join(", ") };
    let unknown: Vec<&str> = given.keys().filter(|k| !props.contains_key(*k)).map(String::as_str).collect();
    if !unknown.is_empty() {
        return Err(format!("{tool} does not take {}. It takes: {}. {NOTHING_DONE}", unknown.join(", "), takes()));
    }
    let mut out = Map::new();
    for (name, value) in given {
        let text = match &value {
            Value::Null => continue,
            Value::String(s) if s.trim().is_empty() => continue,
            Value::String(s) => s.trim().to_string(),
            other => {
                return Err(format!(
                    "{name} has the wrong type: it must be text, not {}. {NOTHING_DONE}",
                    if other.is_number() {
                        "a number"
                    } else if other.is_boolean() {
                        "true or false"
                    } else {
                        "a list or an object"
                    }
                ));
            }
        };
        if let Some(allowed) = props[&name].get("enum").and_then(Value::as_array) {
            let allowed: Vec<&str> = allowed.iter().filter_map(Value::as_str).collect();
            let squash = |s: &str| s.to_ascii_lowercase().replace([' ', '-', '_'], "");
            let Some(found) = allowed.iter().find(|a| squash(a) == squash(&text)) else {
                return Err(format!("{name} must be one of {}, not {text:?}. {NOTHING_DONE}", allowed.join(", ")));
            };
            out.insert(name, Value::String((*found).to_string()));
            continue;
        }
        out.insert(name, Value::String(text));
    }
    for name in schema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
        if !out.contains_key(name) {
            return Err(format!("{tool} needs {name}. {NOTHING_DONE}"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string" }, "access": { "type": "string", "enum": ["read", "write", "read_write"] } },
            "required": ["path", "access"],
            "additionalProperties": false
        })
    }

    #[test]
    fn an_undeclared_argument_is_refused_with_the_real_ones() {
        let err = check("grant_path", &schema(), &json!({"path": "~/x", "access": "read", "recursive": true})).unwrap_err();
        assert!(err.contains("does not take recursive"), "{err}");
        assert!(err.contains("path") && err.contains("access") && err.contains(NOTHING_DONE), "{err}");
    }

    #[test]
    fn a_wrong_type_says_type_not_missing() {
        let err = check("grant_path", &schema(), &json!({"path": 12, "access": "read"})).unwrap_err();
        assert!(err.contains("wrong type") && !err.contains("needs"), "{err}");
    }

    #[test]
    fn blank_text_counts_as_absent() {
        let err = check("grant_path", &schema(), &json!({"path": "  ", "access": "read"})).unwrap_err();
        assert!(err.contains("needs path"), "{err}");
    }

    #[test]
    fn an_enum_is_lenient_about_spelling_and_strict_about_values() {
        let ok = check("grant_path", &schema(), &json!({"path": "~/x", "access": "Read-Write"})).unwrap();
        assert_eq!(ok["access"], "read_write");
        let err = check("grant_path", &schema(), &json!({"path": "~/x", "access": "everything"})).unwrap_err();
        assert!(err.contains("read, write, read_write"), "{err}");
    }

    #[test]
    fn arguments_must_be_an_object() {
        assert!(check("grant_path", &schema(), &json!(["~/x"])).unwrap_err().contains("named arguments"));
    }
}

#[cfg(test)]
mod properties {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    proptest! {
        /// An argument the schema does not declare is refused by name, whatever else is passed.
        #[test]
        fn an_undeclared_argument_is_always_refused(name in "[a-z_]{1,12}", value in "[ -~]{0,12}") {
            prop_assume!(name != "path" && name != "access");
            let schema = json!({"type": "object", "properties": {"path": {"type": "string"}, "access": {"type": "string"}}, "additionalProperties": false});
            let err = check("grant_path", &schema, &json!({"path": "~/x", "access": "read", name.clone(): value})).unwrap_err();
            prop_assert!(err.contains(&name) && err.contains(NOTHING_DONE), "{}", err);
        }
    }
}
