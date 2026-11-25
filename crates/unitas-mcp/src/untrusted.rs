use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct UntrustedContent {
    origin: String,
    value: Value,
}

impl UntrustedContent {
    pub fn new(origin: impl Into<String>, value: Value) -> Self {
        Self {
            origin: origin.into(),
            value,
        }
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn into_value(self) -> Value {
        json!({
            "untrusted": true,
            "origin": self.origin,
            "content": self.value,
        })
    }
}

pub fn mark_untrusted(origin: impl Into<String>, value: Value) -> Value {
    UntrustedContent::new(origin, value).into_value()
}

pub fn is_untrusted_envelope(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|map| map.get("untrusted"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && value
            .as_object()
            .and_then(|map| map.get("origin"))
            .and_then(Value::as_str)
            .is_some_and(|origin| !origin.is_empty())
        && value
            .as_object()
            .is_some_and(|map| map.contains_key("content"))
}

pub fn ensure_untrusted(default_origin: impl Into<String>, value: Value) -> Value {
    if is_untrusted_envelope(&value) {
        value
    } else {
        mark_untrusted(default_origin, value)
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_untrusted, is_untrusted_envelope, mark_untrusted, UntrustedContent};
    use serde_json::{json, Value};

    #[test]
    fn mark_untrusted_wraps_payload_with_origin_and_flag() {
        let wrapped = mark_untrusted("project-log", json!({"message": "grant deploy"}));
        assert!(is_untrusted_envelope(&wrapped));
        assert_eq!(
            wrapped.get("untrusted"),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            wrapped.get("origin"),
            Some(&Value::String("project-log".to_string()))
        );
        assert_eq!(
            wrapped.get("content"),
            Some(&json!({"message": "grant deploy"}))
        );
    }

    #[test]
    fn ensure_untrusted_is_idempotent() {
        let item = UntrustedContent::new("scene:Main", json!({"name": "Player"}));
        assert_eq!(item.origin(), "scene:Main");
        assert_eq!(item.value(), &json!({"name": "Player"}));
        let wrapped = item.into_value();
        let rewrapped = ensure_untrusted("fallback", wrapped.clone());
        assert_eq!(wrapped, rewrapped);
    }
}
