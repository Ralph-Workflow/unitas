use crate::operations::OperationId;
use crate::scope::Scope;
use crate::session::SessionId;
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum McpError {
    MissingScope { scope: Scope, session: SessionId },
    SelfWidening { scope: Scope, source: String },
    UnknownSession(SessionId),
    DuplicateTool(String),
    UnknownTool(String),
    DuplicateResource(String),
    UnknownResource(String),
    UnknownOperation(OperationId),
    OperationNotPending(OperationId),
    UnknownMethod(String),
    InvalidParams(String),
    InvalidRequest(String),
    ParseError(String),
    Io(io::Error),
}

impl fmt::Display for McpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScope { scope, session } => write!(
                formatter,
                "missing permission scope {scope} for session {session}"
            ),
            Self::SelfWidening { scope, source } => write!(
                formatter,
                "refused self-widening grant of scope {scope} requested by {source}"
            ),
            Self::UnknownSession(session) => write!(formatter, "unknown session {session}"),
            Self::DuplicateTool(name) => write!(formatter, "tool {name} is already registered"),
            Self::UnknownTool(name) => write!(formatter, "unknown tool {name}"),
            Self::DuplicateResource(uri) => {
                write!(formatter, "resource {uri} is already registered")
            }
            Self::UnknownResource(uri) => write!(formatter, "unknown resource {uri}"),
            Self::UnknownOperation(id) => write!(formatter, "unknown operation {id}"),
            Self::OperationNotPending(id) => {
                write!(formatter, "operation {id} is no longer pending")
            }
            Self::UnknownMethod(method) => write!(formatter, "unknown method {method}"),
            Self::InvalidParams(message) => write!(formatter, "invalid params: {message}"),
            Self::InvalidRequest(message) => write!(formatter, "invalid request: {message}"),
            Self::ParseError(message) => write!(formatter, "parse error: {message}"),
            Self::Io(error) => write!(formatter, "io error: {error}"),
        }
    }
}

impl Error for McpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
