use crate::error::McpError;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(String);

impl OperationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OperationState {
    Pending {
        stage: Option<String>,
    },
    Completed {
        result: Value,
    },
    Failed {
        error: String,
    },
}

impl OperationState {
    pub fn status_str(&self) -> &'static str {
        match self {
            Self::Pending { .. } => "pending",
            Self::Completed { .. } => "completed",
            Self::Failed { .. } => "failed",
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationHandle {
    id: OperationId,
    name: String,
    state: OperationState,
}

impl OperationHandle {
    pub fn id(&self) -> &OperationId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state(&self) -> &OperationState {
        &self.state
    }

    pub fn to_value(&self) -> Value {
        match &self.state {
            OperationState::Pending { stage } => json!({
                "id": self.id.as_str(),
                "operation": self.name,
                "status": "pending",
                "stage": stage,
            }),
            OperationState::Completed { result } => json!({
                "id": self.id.as_str(),
                "operation": self.name,
                "status": "completed",
                "result": result,
            }),
            OperationState::Failed { error } => json!({
                "id": self.id.as_str(),
                "operation": self.name,
                "status": "failed",
                "error": error,
            }),
        }
    }
}

pub struct OperationStore {
    operations: BTreeMap<OperationId, OperationHandle>,
    next_id: u64,
}

impl OperationStore {
    pub fn new() -> Self {
        Self {
            operations: BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn start(&mut self, name: impl Into<String>) -> OperationId {
        self.start_with_stage(name, None)
    }

    pub fn start_with_stage(
        &mut self,
        name: impl Into<String>,
        stage: Option<String>,
    ) -> OperationId {
        self.next_id += 1;
        let id = OperationId::new(format!("op-{}", self.next_id));
        let handle = OperationHandle {
            id: id.clone(),
            name: name.into(),
            state: OperationState::Pending { stage },
        };
        self.operations.insert(id.clone(), handle);
        id
    }

    pub fn update_stage(
        &mut self,
        id: &OperationId,
        stage: impl Into<String>,
    ) -> Result<(), McpError> {
        let handle = self
            .operations
            .get_mut(id)
            .ok_or_else(|| McpError::UnknownOperation(id.clone()))?;
        if !handle.state.is_pending() {
            return Err(McpError::OperationNotPending(id.clone()));
        }
        handle.state = OperationState::Pending {
            stage: Some(stage.into()),
        };
        Ok(())
    }

    pub fn complete(&mut self, id: &OperationId, result: Value) -> Result<(), McpError> {
        let handle = self
            .operations
            .get_mut(id)
            .ok_or_else(|| McpError::UnknownOperation(id.clone()))?;
        if !handle.state.is_pending() {
            return Err(McpError::OperationNotPending(id.clone()));
        }
        handle.state = OperationState::Completed { result };
        Ok(())
    }

    pub fn fail(
        &mut self,
        id: &OperationId,
        error: impl Into<String>,
    ) -> Result<(), McpError> {
        let handle = self
            .operations
            .get_mut(id)
            .ok_or_else(|| McpError::UnknownOperation(id.clone()))?;
        if !handle.state.is_pending() {
            return Err(McpError::OperationNotPending(id.clone()));
        }
        handle.state = OperationState::Failed {
            error: error.into(),
        };
        Ok(())
    }

    pub fn get(&self, id: &OperationId) -> Result<&OperationHandle, McpError> {
        self.operations
            .get(id)
            .ok_or_else(|| McpError::UnknownOperation(id.clone()))
    }

    pub fn iter(&self) -> impl Iterator<Item = &OperationHandle> {
        self.operations.values()
    }
}

impl Default for OperationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationId, OperationState, OperationStore};
    use crate::error::McpError;
    use serde_json::json;

    #[test]
    fn operation_lifecycle_transitions_from_pending_to_completed() {
        let mut store = OperationStore::new();
        let id = store.start_with_stage("build-player", Some("compiling".to_string()));
        let initial = store.get(&id);
        assert!(initial.is_ok());
        assert_eq!(
            initial.map(|op| op.state().status_str()).unwrap_or(""),
            "pending"
        );
        assert!(store.update_stage(&id, "packaging").is_ok());
        assert!(store.complete(&id, json!({"artifact": "App.app"})).is_ok());
        let completed = store.get(&id);
        assert!(completed.is_ok());
        assert_eq!(
            completed.ok().map(super::OperationHandle::state),
            Some(&OperationState::Completed {
                result: json!({"artifact": "App.app"})
            })
        );
        let duplicate = store.fail(&id, "too late").err();
        assert!(matches!(duplicate, Some(McpError::OperationNotPending(_))));
    }

    #[test]
    fn operation_can_transition_to_failed_and_unknown_is_rejected() {
        let mut store = OperationStore::new();
        let id = store.start("import-assets");
        assert!(store.fail(&id, "corrupt asset").is_ok());
        let failed = store.get(&id);
        assert_eq!(
            failed.ok().map(super::OperationHandle::state),
            Some(&OperationState::Failed {
                error: "corrupt asset".to_string()
            })
        );
        let missing = store.get(&OperationId::new("op-999")).err();
        assert!(matches!(missing, Some(McpError::UnknownOperation(_))));
    }
}
