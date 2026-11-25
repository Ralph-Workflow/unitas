using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed partial class McpHost
    {
        private readonly IClock clock;
        private readonly Authorizer authorizer;
        private readonly ToolRegistry tools;
        private readonly ResourceRegistry resources;
        private readonly OperationStore operations;
        private readonly AuditLog auditLog;

        // shortcut: the Contract's Authorizer exposes no session lookup, so the host
        // mirrors each session's identity at creation time for audit records; revisit
        // if the Authorizer grows a session getter.
        private readonly Dictionary<string, SessionIdentity> identities =
            new Dictionary<string, SessionIdentity>(StringComparer.Ordinal);

        public McpHost()
            : this(new SystemClock())
        {
        }

        public McpHost(IClock clock)
        {
            ArgumentNullException.ThrowIfNull(clock);
            this.clock = clock;
            authorizer = new Authorizer();
            tools = new ToolRegistry();
            resources = new ResourceRegistry();
            operations = new OperationStore();
            auditLog = new AuditLog();
        }

        public OperationStore Operations => operations;

        public AuditLog Audit => auditLog;

        public string CreateSession(SessionIdentity identity)
        {
            string sessionId = authorizer.CreateSession(identity);
            identities[sessionId] = identity;
            return sessionId;
        }

        public void GrantScope(string sessionId, Scope scope, GrantAuthority authority)
        {
            SessionIdentity identity = IdentityOf(sessionId);
            long timestamp = clock.NowMs();
            try
            {
                authorizer.GrantScope(sessionId, scope, authority);
            }
            catch (McpError)
            {
                auditLog.Record(new AuditEntry(
                    timestamp,
                    sessionId,
                    identity,
                    AuditTargetKind.Grant,
                    "grant:" + Scopes.AsString(scope),
                    scope,
                    new AuditOutcome.RefusedSelfWidening(scope),
                    Array.Empty<string>()));
                throw;
            }

            auditLog.Record(new AuditEntry(
                timestamp,
                sessionId,
                identity,
                AuditTargetKind.Grant,
                "grant:" + Scopes.AsString(scope),
                scope,
                new AuditOutcome.Allowed(),
                new[] { sessionId }));
        }

        public void GrantStanding(string agent, Scope scope, GrantAuthority authority)
        {
            authorizer.GrantStanding(agent, scope, authority);
        }

        public void RegisterTool(ToolDefinition definition, ToolHandler handler)
        {
            tools.Register(definition, handler);
        }

        public void RegisterResource(ResourceDefinition definition, ResourceReader reader)
        {
            resources.Register(definition, reader);
        }

        public IReadOnlyList<ToolDefinition> ListTools()
        {
            return tools.Definitions();
        }

        public IReadOnlyList<ResourceDefinition> ListResources()
        {
            return resources.Definitions();
        }

        public JsonNode CallTool(string sessionId, string name, JsonNode arguments)
        {
            SessionIdentity identity = IdentityOf(sessionId);
            if (name == "permissions/grant" || name == "scopes/grant")
            {
                Scope requested = RequestedScope(arguments);
                long refusedAt = clock.NowMs();
                auditLog.Record(new AuditEntry(
                    refusedAt,
                    sessionId,
                    identity,
                    AuditTargetKind.Grant,
                    name,
                    requested,
                    new AuditOutcome.RefusedSelfWidening(requested),
                    Array.Empty<string>()));
                throw new McpError.SelfWidening(requested, "tool call " + name + " by session " + sessionId);
            }

            RegisteredTool registered = tools.GetRegistered(name);
            Scope requiredScope = registered.Definition.RequiredScope;
            long timestamp = clock.NowMs();
            try
            {
                authorizer.Authorize(sessionId, requiredScope);
            }
            catch (McpError)
            {
                auditLog.Record(new AuditEntry(
                    timestamp,
                    sessionId,
                    identity,
                    AuditTargetKind.Tool,
                    name,
                    requiredScope,
                    new AuditOutcome.RefusedMissingScope(requiredScope),
                    Array.Empty<string>()));
                throw;
            }

            ToolOutput output;
            try
            {
                output = registered.Handler(arguments);
            }
            catch (McpError error)
            {
                auditLog.Record(new AuditEntry(
                    timestamp,
                    sessionId,
                    identity,
                    AuditTargetKind.Tool,
                    name,
                    requiredScope,
                    new AuditOutcome.Failed(error.Message),
                    Array.Empty<string>()));
                throw;
            }

            JsonNode envelope = output.ToEnvelopedValue();
            auditLog.Record(new AuditEntry(
                timestamp,
                sessionId,
                identity,
                AuditTargetKind.Tool,
                name,
                requiredScope,
                new AuditOutcome.Allowed(),
                output.AffectedIdentities));
            return envelope;
        }

        public JsonNode ReadResource(string sessionId, string uri)
        {
            SessionIdentity identity = IdentityOf(sessionId);
            RegisteredResource registered = resources.GetRegistered(uri);
            Scope requiredScope = registered.Definition.RequiredScope;
            long timestamp = clock.NowMs();
            try
            {
                authorizer.Authorize(sessionId, requiredScope);
            }
            catch (McpError)
            {
                auditLog.Record(new AuditEntry(
                    timestamp,
                    sessionId,
                    identity,
                    AuditTargetKind.Resource,
                    uri,
                    requiredScope,
                    new AuditOutcome.RefusedMissingScope(requiredScope),
                    Array.Empty<string>()));
                throw;
            }

            ResourceContent content;
            try
            {
                content = registered.Reader();
            }
            catch (McpError error)
            {
                auditLog.Record(new AuditEntry(
                    timestamp,
                    sessionId,
                    identity,
                    AuditTargetKind.Resource,
                    uri,
                    requiredScope,
                    new AuditOutcome.Failed(error.Message),
                    Array.Empty<string>()));
                throw;
            }

            JsonNode envelope = content.ToEnvelopedValue();
            auditLog.Record(new AuditEntry(
                timestamp,
                sessionId,
                identity,
                AuditTargetKind.Resource,
                uri,
                requiredScope,
                new AuditOutcome.Allowed(),
                new[] { uri }));
            return envelope;
        }

        public string StartOperation(string sessionId, string name, Scope requiredScope)
        {
            SessionIdentity identity = IdentityOf(sessionId);
            long timestamp = clock.NowMs();
            try
            {
                authorizer.Authorize(sessionId, requiredScope);
            }
            catch (McpError)
            {
                auditLog.Record(new AuditEntry(
                    timestamp,
                    sessionId,
                    identity,
                    AuditTargetKind.Operation,
                    name,
                    requiredScope,
                    new AuditOutcome.RefusedMissingScope(requiredScope),
                    Array.Empty<string>()));
                throw;
            }

            string operationId = operations.Start(name);
            auditLog.Record(new AuditEntry(
                timestamp,
                sessionId,
                identity,
                AuditTargetKind.Operation,
                name,
                requiredScope,
                new AuditOutcome.Allowed(),
                new[] { operationId }));
            return operationId;
        }

        public void CompleteOperation(string operationId, string origin, JsonNode result)
        {
            JsonNode wrapped = UntrustedContent.Ensure(origin, result);
            operations.Complete(operationId, wrapped);
        }

        public void FailOperation(string operationId, string error)
        {
            operations.Fail(operationId, error);
        }

        public OperationHandle GetOperation(string operationId)
        {
            return operations.Get(operationId);
        }

        private SessionIdentity IdentityOf(string sessionId)
        {
            if (!identities.TryGetValue(sessionId, out SessionIdentity? identity))
            {
                throw new McpError.UnknownSession(sessionId);
            }

            return identity;
        }

        private static Scope RequestedScope(JsonNode arguments)
        {
            JsonNode? scopeNode = (arguments as JsonObject)?["scope"];
            if (scopeNode is JsonValue value
                && value.TryGetValue(out string? name)
                && Scopes.TryParse(name, out Scope parsed))
            {
                return parsed;
            }

            return Scope.ReadProject;
        }
    }
}
