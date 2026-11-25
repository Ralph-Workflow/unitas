using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public enum AuditTargetKind
    {
        Tool,
        Resource,
        Grant,
        Operation,
    }

    public static class AuditTargetKinds
    {
        public static string AsString(AuditTargetKind kind)
        {
            switch (kind)
            {
                case AuditTargetKind.Tool: return "tool";
                case AuditTargetKind.Resource: return "resource";
                case AuditTargetKind.Grant: return "grant";
                case AuditTargetKind.Operation: return "operation";
                default: return kind.ToString();
            }
        }

        public static bool TryParse(string name, out AuditTargetKind kind)
        {
            switch (name)
            {
                case "tool":
                    kind = AuditTargetKind.Tool;
                    return true;
                case "resource":
                    kind = AuditTargetKind.Resource;
                    return true;
                case "grant":
                    kind = AuditTargetKind.Grant;
                    return true;
                case "operation":
                    kind = AuditTargetKind.Operation;
                    return true;
                default:
                    kind = default;
                    return false;
            }
        }
    }

    public abstract record AuditOutcome
    {
        private AuditOutcome()
        {
        }

        public abstract string StatusString { get; }

        public static AuditOutcome FromJson(string status, string detail)
        {
            ArgumentNullException.ThrowIfNull(status);
            ArgumentNullException.ThrowIfNull(detail);
            switch (status)
            {
                case "allowed":
                    return new Allowed();
                case "refused-missing-scope":
                    return new RefusedMissingScope(ParseScopeFromDetail(detail, "refused: missing scope "));
                case "refused-self-widening":
                    return new RefusedSelfWidening(ParseScopeFromDetail(detail, "refused: self-widening scope "));
                case "failed":
                    const string failedPrefix = "failed: ";
                    string reason = detail.StartsWith(failedPrefix, StringComparison.Ordinal)
                        ? detail.Substring(failedPrefix.Length)
                        : detail;
                    return new Failed(reason);
                default:
                    throw new McpError.ParseError("unknown audit outcome " + status);
            }
        }

        private static Scope ParseScopeFromDetail(string detail, string prefix)
        {
            string scopeName = detail.StartsWith(prefix, StringComparison.Ordinal)
                ? detail.Substring(prefix.Length)
                : detail;
            if (Scopes.TryParse(scopeName, out Scope parsed))
            {
                return parsed;
            }

            return Scope.ReadProject;
        }

        public sealed record Allowed : AuditOutcome
        {
            public override string StatusString => "allowed";

            public override string ToString() => "allowed";
        }

        public sealed record RefusedMissingScope(Scope Missing) : AuditOutcome
        {
            public override string StatusString => "refused-missing-scope";

            public override string ToString() => "refused: missing scope " + Scopes.AsString(Missing);
        }

        public sealed record RefusedSelfWidening(Scope Requested) : AuditOutcome
        {
            public override string StatusString => "refused-self-widening";

            public override string ToString() => "refused: self-widening scope " + Scopes.AsString(Requested);
        }

        public sealed record Failed(string Reason) : AuditOutcome
        {
            public override string StatusString => "failed";

            public override string ToString() => "failed: " + Reason;
        }
    }

    public sealed record AuditEntry(
        long TimestampMs,
        string SessionId,
        SessionIdentity Identity,
        AuditTargetKind TargetKind,
        string Target,
        Scope Scope,
        AuditOutcome Outcome,
        IReadOnlyList<string> AffectedIdentities);

    public sealed record AuditRecord(
        long Sequence,
        long TimestampMs,
        string SessionId,
        SessionIdentity Identity,
        AuditTargetKind TargetKind,
        string Target,
        Scope Scope,
        AuditOutcome Outcome,
        IReadOnlyList<string> AffectedIdentities)
    {
        public static AuditRecord FromJson(JsonObject obj)
        {
            ArgumentNullException.ThrowIfNull(obj);
            long sequence = obj["sequence"]?.GetValue<long>()
                ?? throw new McpError.ParseError("audit record missing sequence");
            long timestampMs = obj["timestamp_ms"]?.GetValue<long>()
                ?? throw new McpError.ParseError("audit record missing timestamp_ms");
            string sessionId = obj["session_id"]?.GetValue<string>()
                ?? throw new McpError.ParseError("audit record missing session_id");

            if (!(obj["identity"] is JsonObject identityObj))
            {
                throw new McpError.ParseError("audit record missing identity");
            }

            string agent = identityObj["agent"]?.GetValue<string>() ?? string.Empty;
            string client = identityObj["client"]?.GetValue<string>() ?? string.Empty;
            string authorizedBy = identityObj["authorized_by"]?.GetValue<string>() ?? string.Empty;
            SessionIdentity identity = new SessionIdentity(agent, client, authorizedBy);

            string targetKindStr = obj["target_kind"]?.GetValue<string>()
                ?? throw new McpError.ParseError("audit record missing target_kind");
            if (!AuditTargetKinds.TryParse(targetKindStr, out AuditTargetKind targetKind))
            {
                throw new McpError.ParseError("unknown audit target_kind " + targetKindStr);
            }

            string target = obj["target"]?.GetValue<string>()
                ?? throw new McpError.ParseError("audit record missing target");
            string scopeStr = obj["scope"]?.GetValue<string>()
                ?? throw new McpError.ParseError("audit record missing scope");
            if (!Scopes.TryParse(scopeStr, out Scope scope))
            {
                throw new McpError.ParseError("unknown audit scope " + scopeStr);
            }

            string outcomeStr = obj["outcome"]?.GetValue<string>()
                ?? throw new McpError.ParseError("audit record missing outcome");
            string outcomeDetail = obj["outcome_detail"]?.GetValue<string>() ?? outcomeStr;
            AuditOutcome outcome = AuditOutcome.FromJson(outcomeStr, outcomeDetail);

            List<string> affected = new List<string>();
            if (obj["affected_identities"] is JsonArray affectedArray)
            {
                foreach (JsonNode? item in affectedArray)
                {
                    string? value = item?.GetValue<string>();
                    if (value != null)
                    {
                        affected.Add(value);
                    }
                }
            }

            return new AuditRecord(
                sequence,
                timestampMs,
                sessionId,
                identity,
                targetKind,
                target,
                scope,
                outcome,
                affected);
        }

        public JsonObject ToValue()
        {
            JsonArray affected = new JsonArray();
            foreach (string identity in AffectedIdentities)
            {
                affected.Add(identity);
            }

            return new JsonObject
            {
                ["sequence"] = Sequence,
                ["timestamp_ms"] = TimestampMs,
                ["session_id"] = SessionId,
                ["identity"] = new JsonObject
                {
                    ["agent"] = Identity.Agent,
                    ["client"] = Identity.Client,
                    ["authorized_by"] = Identity.AuthorizedBy,
                },
                ["target_kind"] = AuditTargetKinds.AsString(TargetKind),
                ["target"] = Target,
                ["scope"] = Scopes.AsString(Scope),
                ["outcome"] = Outcome.StatusString,
                ["outcome_detail"] = Outcome.ToString(),
                ["affected_identities"] = affected,
            };
        }
    }

    public sealed class AuditLog
    {
        private readonly List<AuditRecord> _records = new List<AuditRecord>();
        private readonly AuditStore? _store;
        private readonly string? _storeFailure;

        public AuditLog()
            : this(null, null)
        {
        }

        public AuditLog(AuditStore? store, string? storeFailure = null)
        {
            _store = store;
            _storeFailure = storeFailure;
            if (_store != null)
            {
                foreach (AuditRecord existing in _store.LoadAll())
                {
                    _records.Add(existing);
                }
            }
        }

        public AuditStore? Store => _store;

        public string? StoreFailure => _storeFailure;

        public AuditRecord Record(AuditEntry entry)
        {
            ArgumentNullException.ThrowIfNull(entry);
            AuditRecord record = new AuditRecord(
                _records.Count + 1,
                entry.TimestampMs,
                entry.SessionId,
                entry.Identity,
                entry.TargetKind,
                entry.Target,
                entry.Scope,
                entry.Outcome,
                entry.AffectedIdentities);
            _records.Add(record);
            _store?.Append(record);
            return record;
        }

        public IReadOnlyList<AuditRecord> Records => _records;

        public int Count => _records.Count;

        public bool IsEmpty => _records.Count == 0;

        public JsonArray ExportJson()
        {
            JsonArray exported = new JsonArray();
            foreach (AuditRecord record in _records)
            {
                exported.Add(record.ToValue());
            }

            return exported;
        }
    }
}
