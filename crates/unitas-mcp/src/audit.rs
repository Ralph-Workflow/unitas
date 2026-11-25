use crate::scope::Scope;
use crate::session::{SessionId, SessionIdentity};
use serde_json::{json, Value};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditTargetKind {
    Tool,
    Resource,
    Grant,
    Operation,
}

impl AuditTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Resource => "resource",
            Self::Grant => "grant",
            Self::Operation => "operation",
        }
    }
}

impl fmt::Display for AuditTargetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuditOutcome {
    Allowed,
    RefusedMissingScope { missing: Scope },
    RefusedSelfWidening { requested: Scope },
    Failed { reason: String },
}

impl AuditOutcome {
    pub fn status_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::RefusedMissingScope { .. } => "refused-missing-scope",
            Self::RefusedSelfWidening { .. } => "refused-self-widening",
            Self::Failed { .. } => "failed",
        }
    }
}

impl fmt::Display for AuditOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allowed => formatter.write_str("allowed"),
            Self::RefusedMissingScope { missing } => {
                write!(formatter, "refused: missing scope {missing}")
            }
            Self::RefusedSelfWidening { requested } => {
                write!(formatter, "refused: self-widening scope {requested}")
            }
            Self::Failed { reason } => write!(formatter, "failed: {reason}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    sequence: u64,
    timestamp_ms: u64,
    session_id: SessionId,
    identity: SessionIdentity,
    target_kind: AuditTargetKind,
    target: String,
    scope: Scope,
    outcome: AuditOutcome,
    affected_identities: Vec<String>,
}

impl AuditRecord {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn identity(&self) -> &SessionIdentity {
        &self.identity
    }

    pub fn target_kind(&self) -> AuditTargetKind {
        self.target_kind
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn scope(&self) -> Scope {
        self.scope
    }

    pub fn outcome(&self) -> &AuditOutcome {
        &self.outcome
    }

    pub fn affected_identities(&self) -> &[String] {
        &self.affected_identities
    }

    pub fn to_value(&self) -> Value {
        json!({
            "sequence": self.sequence,
            "timestamp_ms": self.timestamp_ms,
            "session_id": self.session_id.as_str(),
            "identity": {
                "agent": self.identity.agent(),
                "client": self.identity.client(),
                "authorized_by": self.identity.authorized_by(),
            },
            "target_kind": self.target_kind.as_str(),
            "target": self.target,
            "scope": self.scope.as_str(),
            "outcome": self.outcome.status_str(),
            "outcome_detail": self.outcome.to_string(),
            "affected_identities": self.affected_identities,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    pub timestamp_ms: u64,
    pub session_id: SessionId,
    pub identity: SessionIdentity,
    pub target_kind: AuditTargetKind,
    pub target: String,
    pub scope: Scope,
    pub outcome: AuditOutcome,
    pub affected_identities: Vec<String>,
}

pub struct AuditLog {
    records: Vec<AuditRecord>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, entry: AuditEntry) -> &AuditRecord {
        let sequence = (self.records.len() as u64) + 1;
        self.records.push(AuditRecord {
            sequence,
            timestamp_ms: entry.timestamp_ms,
            session_id: entry.session_id,
            identity: entry.identity,
            target_kind: entry.target_kind,
            target: entry.target,
            scope: entry.scope,
            outcome: entry.outcome,
            affected_identities: entry.affected_identities,
        });
        let last_index = self.records.len() - 1;
        &self.records[last_index]
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn export_json(&self) -> Value {
        Value::Array(self.records.iter().map(AuditRecord::to_value).collect())
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{AuditEntry, AuditLog, AuditOutcome, AuditTargetKind};
    use crate::scope::Scope;
    use crate::session::{SessionId, SessionIdentity};

    #[test]
    fn audit_log_is_append_only_and_exports_records() {
        let mut log = AuditLog::new();
        assert!(log.is_empty());
        let session_id = SessionId::new("session-1");
        let identity = SessionIdentity::new("agent-a", "stdio", "person-1");
        log.record(AuditEntry {
            timestamp_ms: 1_000,
            session_id: session_id.clone(),
            identity: identity.clone(),
            target_kind: AuditTargetKind::Resource,
            target: "unitas://project/scenes".to_string(),
            scope: Scope::ReadProject,
            outcome: AuditOutcome::Allowed,
            affected_identities: vec!["scene:Main".to_string()],
        });
        log.record(AuditEntry {
            timestamp_ms: 1_050,
            session_id,
            identity,
            target_kind: AuditTargetKind::Tool,
            target: "scene.edit".to_string(),
            scope: Scope::EditContent,
            outcome: AuditOutcome::RefusedMissingScope {
                missing: Scope::EditContent,
            },
            affected_identities: Vec::new(),
        });
        assert_eq!(log.len(), 2);
        let records = log.records();
        assert_eq!(records[0].sequence(), 1);
        assert_eq!(records[0].timestamp_ms(), 1_000);
        assert_eq!(records[0].target_kind(), AuditTargetKind::Resource);
        assert_eq!(records[0].target(), "unitas://project/scenes");
        assert_eq!(records[0].scope(), Scope::ReadProject);
        assert_eq!(records[0].outcome(), &AuditOutcome::Allowed);
        assert_eq!(records[0].affected_identities(), &["scene:Main".to_string()]);
        assert_eq!(records[1].sequence(), 2);
        assert_eq!(
            records[1].outcome(),
            &AuditOutcome::RefusedMissingScope {
                missing: Scope::EditContent
            }
        );
        let exported = log.export_json();
        assert!(exported.is_array());
    }
}
