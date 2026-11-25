use crate::error::McpError;
use crate::scope::Scope;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionIdentity {
    agent: String,
    client: String,
    authorized_by: String,
}

impl SessionIdentity {
    pub fn new(
        agent: impl Into<String>,
        client: impl Into<String>,
        authorized_by: impl Into<String>,
    ) -> Self {
        Self {
            agent: agent.into(),
            client: client.into(),
            authorized_by: authorized_by.into(),
        }
    }

    pub fn agent(&self) -> &str {
        &self.agent
    }

    pub fn client(&self) -> &str {
        &self.client
    }

    pub fn authorized_by(&self) -> &str {
        &self.authorized_by
    }
}

impl fmt::Display for SessionIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} via {} authorized by {}",
            self.agent, self.client, self.authorized_by
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    id: SessionId,
    identity: SessionIdentity,
    scopes: BTreeSet<Scope>,
}

impl Session {
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn identity(&self) -> &SessionIdentity {
        &self.identity
    }

    pub fn scopes(&self) -> impl Iterator<Item = Scope> + '_ {
        self.scopes.iter().copied()
    }

    pub fn holds(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantAuthority {
    Person,
    SessionInherited,
}

pub struct Authorizer {
    sessions: BTreeMap<SessionId, Session>,
    standing: BTreeMap<String, BTreeSet<Scope>>,
    next_session: u64,
}

impl Authorizer {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            standing: BTreeMap::new(),
            next_session: 0,
        }
    }

    pub fn create_session(&mut self, identity: SessionIdentity) -> SessionId {
        self.next_session += 1;
        let id = SessionId::new(format!("session-{}", self.next_session));
        let mut scopes = BTreeSet::new();
        scopes.insert(Scope::ReadProject);
        let session = Session {
            id: id.clone(),
            identity,
            scopes,
        };
        self.sessions.insert(id.clone(), session);
        id
    }

    pub fn session(&self, id: &SessionId) -> Result<&Session, McpError> {
        self.sessions
            .get(id)
            .ok_or_else(|| McpError::UnknownSession(id.clone()))
    }

    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    pub fn grant_scope(
        &mut self,
        id: &SessionId,
        scope: Scope,
        authority: GrantAuthority,
    ) -> Result<(), McpError> {
        if authority == GrantAuthority::SessionInherited {
            return Err(McpError::SelfWidening {
                scope,
                source: format!("session {id}"),
            });
        }
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| McpError::UnknownSession(id.clone()))?;
        session.scopes.insert(scope);
        Ok(())
    }

    pub fn grant_standing(
        &mut self,
        agent: &str,
        scope: Scope,
        authority: GrantAuthority,
    ) -> Result<(), McpError> {
        if authority == GrantAuthority::SessionInherited {
            return Err(McpError::SelfWidening {
                scope,
                source: format!("standing grant for agent {agent}"),
            });
        }
        self.standing
            .entry(agent.to_string())
            .or_default()
            .insert(scope);
        Ok(())
    }

    pub fn authorize(&self, id: &SessionId, scope: Scope) -> Result<(), McpError> {
        let session = self.session(id)?;
        if session.holds(scope) {
            return Ok(());
        }
        let standing = self
            .standing
            .get(session.identity().agent())
            .is_some_and(|scopes| scopes.contains(&scope));
        if standing {
            return Ok(());
        }
        Err(McpError::MissingScope {
            scope,
            session: id.clone(),
        })
    }
}

impl Default for Authorizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Authorizer, GrantAuthority, SessionIdentity};
    use crate::error::McpError;
    use crate::scope::Scope;

    fn identity() -> SessionIdentity {
        SessionIdentity::new("agent-1", "stdio", "person-1")
    }

    #[test]
    fn new_session_holds_only_read_project() {
        let mut authorizer = Authorizer::new();
        let id = authorizer.create_session(identity());
        let session = authorizer.session(&id);
        assert!(session.is_ok());
        let scopes: Vec<Scope> = session.map(|session| session.scopes().collect()).unwrap_or_default();
        assert_eq!(scopes, vec![Scope::ReadProject]);
    }

    #[test]
    fn person_grant_adds_the_scope() {
        let mut authorizer = Authorizer::new();
        let id = authorizer.create_session(identity());
        assert!(authorizer
            .grant_scope(&id, Scope::EditContent, GrantAuthority::Person)
            .is_ok());
        assert!(authorizer.authorize(&id, Scope::EditContent).is_ok());
    }

    #[test]
    fn session_inherited_grant_is_refused() {
        let mut authorizer = Authorizer::new();
        let id = authorizer.create_session(identity());
        let error = authorizer
            .grant_scope(&id, Scope::Deploy, GrantAuthority::SessionInherited)
            .err();
        let message = error.as_ref().map(McpError::to_string).unwrap_or_default();
        assert!(
            message.contains("deploy"),
            "refusal must name the scope: {message}"
        );
        assert!(matches!(error, Some(McpError::SelfWidening { .. })));
        let holds = authorizer
            .session(&id)
            .map(|session| session.holds(Scope::Deploy))
            .unwrap_or(true);
        assert!(!holds, "a refused grant must not widen the session");
    }

    #[test]
    fn standing_grant_from_a_session_is_refused() {
        let mut authorizer = Authorizer::new();
        let error = authorizer
            .grant_standing("agent-1", Scope::Sign, GrantAuthority::SessionInherited)
            .err();
        assert!(matches!(error, Some(McpError::SelfWidening { .. })));
    }

    #[test]
    fn standing_grant_authorizes_a_matching_agent() {
        let mut authorizer = Authorizer::new();
        let id = authorizer.create_session(identity());
        assert!(authorizer
            .grant_standing("agent-1", Scope::Review, GrantAuthority::Person)
            .is_ok());
        assert!(authorizer.authorize(&id, Scope::Review).is_ok());
    }

    #[test]
    fn missing_scope_refusal_names_the_scope() {
        let mut authorizer = Authorizer::new();
        let id = authorizer.create_session(identity());
        let error = authorizer.authorize(&id, Scope::ExternalNetwork).err();
        let message = error.as_ref().map(McpError::to_string).unwrap_or_default();
        assert!(
            message.contains("external-network"),
            "refusal must name the missing scope: {message}"
        );
        assert!(matches!(
            error,
            Some(McpError::MissingScope {
                scope: Scope::ExternalNetwork,
                ..
            })
        ));
    }
}
