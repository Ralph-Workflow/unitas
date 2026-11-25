use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    ReadProject,
    EditContent,
    EditScripts,
    ExecuteProjectCode,
    ImportExternalFiles,
    ChangePackagesAndSettings,
    Build,
    Sign,
    Deploy,
    ExternalNetwork,
    Review,
}

impl Scope {
    pub const ALL: [Scope; 11] = [
        Scope::ReadProject,
        Scope::EditContent,
        Scope::EditScripts,
        Scope::ExecuteProjectCode,
        Scope::ImportExternalFiles,
        Scope::ChangePackagesAndSettings,
        Scope::Build,
        Scope::Sign,
        Scope::Deploy,
        Scope::ExternalNetwork,
        Scope::Review,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Scope::ReadProject => "read-project",
            Scope::EditContent => "edit-content",
            Scope::EditScripts => "edit-scripts",
            Scope::ExecuteProjectCode => "execute-project-code",
            Scope::ImportExternalFiles => "import-external-files",
            Scope::ChangePackagesAndSettings => "change-packages-and-settings",
            Scope::Build => "build",
            Scope::Sign => "sign",
            Scope::Deploy => "deploy",
            Scope::ExternalNetwork => "external-network",
            Scope::Review => "review",
        }
    }

    pub fn from_name(name: &str) -> Option<Scope> {
        Scope::ALL.into_iter().find(|scope| scope.as_str() == name)
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub required_scope: Scope,
    pub origin: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolManifest {
    pub tools: Vec<ToolDescriptor>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scope_serde_names_match_as_str_for_all_scopes() -> Result<(), Box<dyn std::error::Error>> {
        for scope in Scope::ALL {
            let encoded = serde_json::to_value(scope)?;
            assert_eq!(encoded, json!(scope.as_str()));
            assert_eq!(Scope::from_name(scope.as_str()), Some(scope));
        }
        assert_eq!(Scope::from_name("root-access"), None);
        Ok(())
    }

    #[test]
    fn descriptor_roundtrips_contract_shape() -> Result<(), Box<dyn std::error::Error>> {
        let wire = json!({
            "name": "scene.list",
            "description": "List scenes in the project",
            "inputSchema": {"type": "object", "properties": {}},
            "requiredScope": "read-project",
            "origin": "unitas://editor/scene",
        });
        let descriptor: ToolDescriptor = serde_json::from_value(wire.clone())?;
        assert_eq!(descriptor.required_scope, Scope::ReadProject);
        let encoded = serde_json::to_value(&descriptor)?;
        assert_eq!(encoded, wire);
        Ok(())
    }

    #[test]
    fn manifest_rejects_unknown_scope() {
        let wire = json!({"tools": [{
            "name": "x",
            "description": "x",
            "inputSchema": {},
            "requiredScope": "do-anything",
            "origin": "unitas://x",
        }]});
        assert!(serde_json::from_value::<ToolManifest>(wire).is_err());
    }
}
