use crate::descriptor::{Scope, ToolDescriptor, ToolManifest};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub type ToolHandler = Box<dyn Fn(&Value) -> Result<Value, String> + Send + Sync>;

#[derive(Debug, PartialEq, Eq)]
pub enum AbiError {
    DuplicateTool(String),
    Poisoned,
}

impl fmt::Display for AbiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AbiError::DuplicateTool(name) => write!(formatter, "tool {name} is already registered"),
            AbiError::Poisoned => write!(formatter, "dispatch table lock poisoned"),
        }
    }
}

impl std::error::Error for AbiError {}

pub fn error_json(kind: &str, tool: Option<&str>, message: &str) -> Value {
    let mut error = json!({"kind": kind, "message": message});
    if let Some(name) = tool {
        error["tool"] = json!(name);
    }
    json!({"error": error})
}

struct DispatchEntry {
    descriptor: ToolDescriptor,
    handler: ToolHandler,
}

#[derive(Default)]
pub struct DispatchTableBuilder {
    entries: BTreeMap<String, DispatchEntry>,
    granted: BTreeSet<Scope>,
}

impl DispatchTableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant(mut self, scope: Scope) -> Self {
        self.granted.insert(scope);
        self
    }

    pub fn tool(
        mut self,
        descriptor: ToolDescriptor,
        handler: ToolHandler,
    ) -> Result<Self, AbiError> {
        if self.entries.contains_key(&descriptor.name) {
            return Err(AbiError::DuplicateTool(descriptor.name));
        }
        self.entries.insert(
            descriptor.name.clone(),
            DispatchEntry {
                descriptor,
                handler,
            },
        );
        Ok(self)
    }

    pub fn build(self) -> DispatchTable {
        DispatchTable {
            entries: self.entries,
            granted: self.granted,
        }
    }
}

#[derive(Default)]
pub struct DispatchTable {
    entries: BTreeMap<String, DispatchEntry>,
    granted: BTreeSet<Scope>,
}

impl DispatchTable {
    pub fn builder() -> DispatchTableBuilder {
        DispatchTableBuilder::new()
    }

    pub fn manifest(&self) -> ToolManifest {
        ToolManifest {
            tools: self
                .entries
                .values()
                .map(|entry| entry.descriptor.clone())
                .collect(),
        }
    }

    pub fn describe_json(&self) -> String {
        serde_json::to_string(&self.manifest()).unwrap_or_else(|_| "{\"tools\":[]}".to_string())
    }

    pub fn invoke(&self, name: &str, args: &Value) -> Value {
        let Some(entry) = self.entries.get(name) else {
            return error_json("unknown-tool", Some(name), &format!("unknown tool {name}"));
        };
        let required = entry.descriptor.required_scope;
        if !self.granted.contains(&required) {
            return json!({"error": {
                "kind": "missing-scope",
                "tool": name,
                "scope": required.as_str(),
                "message": format!("tool {name} requires scope {required}"),
            }});
        }
        match (entry.handler)(args) {
            Ok(payload) => payload,
            Err(message) => error_json("tool-failed", Some(name), &message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(name: &str, scope: Scope) -> ToolDescriptor {
        ToolDescriptor {
            name: name.to_string(),
            description: format!("tool {name}"),
            input_schema: json!({"type": "object"}),
            required_scope: scope,
            origin: "unitas://test".to_string(),
        }
    }

    fn echo_table() -> Result<DispatchTable, AbiError> {
        Ok(DispatchTable::builder()
            .grant(Scope::ReadProject)
            .tool(
                descriptor("test.echo", Scope::ReadProject),
                Box::new(|args| Ok(args.clone())),
            )?
            .tool(
                descriptor("test.build", Scope::Build),
                Box::new(|_| Ok(json!({}))),
            )?
            .build())
    }

    #[test]
    fn describe_lists_tools_in_name_order() -> Result<(), Box<dyn std::error::Error>> {
        let table = echo_table()?;
        let manifest = table.manifest();
        let names: Vec<&str> = manifest
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect();
        assert_eq!(names, ["test.build", "test.echo"]);
        let parsed: Value = serde_json::from_str(&table.describe_json())?;
        assert_eq!(parsed["tools"][1]["requiredScope"], json!("read-project"));
        assert_eq!(parsed["tools"][1]["origin"], json!("unitas://test"));
        Ok(())
    }

    #[test]
    fn invoke_dispatches_to_handler() -> Result<(), AbiError> {
        let table = echo_table()?;
        let result = table.invoke("test.echo", &json!({"x": 1}));
        assert_eq!(result, json!({"x": 1}));
        Ok(())
    }

    #[test]
    fn invoke_unknown_name_returns_structured_error() -> Result<(), AbiError> {
        let table = echo_table()?;
        let result = table.invoke("nope", &json!({}));
        assert_eq!(result["error"]["kind"], json!("unknown-tool"));
        assert_eq!(result["error"]["tool"], json!("nope"));
        Ok(())
    }

    #[test]
    fn invoke_missing_scope_returns_structured_error() -> Result<(), AbiError> {
        let table = echo_table()?;
        let result = table.invoke("test.build", &json!({}));
        assert_eq!(result["error"]["kind"], json!("missing-scope"));
        assert_eq!(result["error"]["scope"], json!("build"));
        Ok(())
    }

    #[test]
    fn invoke_handler_failure_returns_structured_error() -> Result<(), AbiError> {
        let table = DispatchTable::builder()
            .grant(Scope::ReadProject)
            .tool(
                descriptor("test.fail", Scope::ReadProject),
                Box::new(|_| Err("boom".to_string())),
            )?
            .build();
        let result = table.invoke("test.fail", &json!({}));
        assert_eq!(result["error"]["kind"], json!("tool-failed"));
        assert_eq!(result["error"]["message"], json!("boom"));
        Ok(())
    }

    #[test]
    fn duplicate_tool_name_is_rejected() -> Result<(), AbiError> {
        let result = DispatchTable::builder()
            .tool(
                descriptor("dup", Scope::ReadProject),
                Box::new(|_| Ok(json!({}))),
            )?
            .tool(descriptor("dup", Scope::Build), Box::new(|_| Ok(json!({}))));
        assert_eq!(
            result.err(),
            Some(AbiError::DuplicateTool("dup".to_string()))
        );
        Ok(())
    }
}
