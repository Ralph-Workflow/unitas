using System;

namespace Unitas.Editor.Mcp
{
    // C# records cannot inherit from Exception, so the variants are sealed
    // classes under an abstract base rather than the `abstract record` the
    // Rust enum would suggest.
    public abstract class McpError : Exception
    {
        protected McpError(string message)
            : base(message)
        {
        }

        protected McpError(string message, Exception innerException)
            : base(message, innerException)
        {
        }

        public sealed class MissingScope : McpError
        {
            public MissingScope(Scope scope, string session)
                : base("missing permission scope " + Scopes.AsString(scope) + " for session " + session)
            {
                Scope = scope;
                Session = session;
            }

            public Scope Scope { get; }

            public string Session { get; }
        }

        public sealed class SelfWidening : McpError
        {
            // Rust field `source`; renamed because Exception already has Source.
            public SelfWidening(Scope scope, string requester)
                : base("refused self-widening grant of scope " + Scopes.AsString(scope) + " requested by " + requester)
            {
                Scope = scope;
                Requester = requester;
            }

            public Scope Scope { get; }

            public string Requester { get; }
        }

        public sealed class UnknownSession : McpError
        {
            public UnknownSession(string session)
                : base("unknown session " + session)
            {
                Session = session;
            }

            public string Session { get; }
        }

        public sealed class DuplicateTool : McpError
        {
            public DuplicateTool(string name)
                : base("tool " + name + " is already registered")
            {
                Name = name;
            }

            public string Name { get; }
        }

        public sealed class UnknownTool : McpError
        {
            public UnknownTool(string name)
                : base("unknown tool " + name)
            {
                Name = name;
            }

            public string Name { get; }
        }

        public sealed class DuplicateResource : McpError
        {
            public DuplicateResource(string uri)
                : base("resource " + uri + " is already registered")
            {
                Uri = uri;
            }

            public string Uri { get; }
        }

        public sealed class UnknownResource : McpError
        {
            public UnknownResource(string uri)
                : base("unknown resource " + uri)
            {
                Uri = uri;
            }

            public string Uri { get; }
        }

        public sealed class UnknownOperation : McpError
        {
            public UnknownOperation(string operationId)
                : base("unknown operation " + operationId)
            {
                OperationId = operationId;
            }

            public string OperationId { get; }
        }

        public sealed class OperationNotPending : McpError
        {
            public OperationNotPending(string operationId)
                : base("operation " + operationId + " is no longer pending")
            {
                OperationId = operationId;
            }

            public string OperationId { get; }
        }

        public sealed class UnknownMethod : McpError
        {
            public UnknownMethod(string method)
                : base("unknown method " + method)
            {
                Method = method;
            }

            public string Method { get; }
        }

        public sealed class InvalidParams : McpError
        {
            public InvalidParams(string message)
                : base("invalid params: " + message)
            {
            }
        }

        public sealed class InvalidRequest : McpError
        {
            public InvalidRequest(string message)
                : base("invalid request: " + message)
            {
            }
        }

        public sealed class ParseError : McpError
        {
            public ParseError(string message)
                : base("parse error: " + message)
            {
            }
        }

        public sealed class Io : McpError
        {
            public Io(Exception error)
                : base("io error: " + error.Message, error)
            {
            }
        }

        public sealed class PersonCredentialRequired : McpError
        {
            public PersonCredentialRequired(string actionName, string reason)
                : base("person-only action " + actionName + " refused: " + reason)
            {
                ActionName = actionName;
                Reason = reason;
            }

            public string ActionName { get; }

            public string Reason { get; }
        }

        public sealed class NoPersonPresent : McpError
        {
            public NoPersonPresent(string actionName)
                : base("no person is present to authorize " + actionName)
            {
                ActionName = actionName;
            }

            public string ActionName { get; }
        }
    }
}
