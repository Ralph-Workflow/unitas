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
    }

    public abstract record AuditOutcome
    {
        private AuditOutcome()
        {
        }

        public abstract string StatusString { get; }

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

        public AuditRecord Record(AuditEntry entry)
        {
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
