using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class AuditTests
    {
        public static void Run()
        {
            AuditLogIsAppendOnlyAndExportsRecords();
        }

        // audit.rs: audit_log_is_append_only_and_exports_records
        private static void AuditLogIsAppendOnlyAndExportsRecords()
        {
            AuditLog log = new AuditLog();
            Check.That("audit_log_is_append_only_and_exports_records: starts empty", log.IsEmpty);
            string sessionId = "session-1";
            SessionIdentity identity = new SessionIdentity("agent-a", "stdio", "person-1");
            log.Record(new AuditEntry(
                1_000,
                sessionId,
                identity,
                AuditTargetKind.Resource,
                "unitas://project/scenes",
                Scope.ReadProject,
                new AuditOutcome.Allowed(),
                new[] { "scene:Main" }));
            log.Record(new AuditEntry(
                1_050,
                sessionId,
                identity,
                AuditTargetKind.Tool,
                "scene.edit",
                Scope.EditContent,
                new AuditOutcome.RefusedMissingScope(Scope.EditContent),
                new string[0]));

            Check.Equal("audit_log_is_append_only_and_exports_records: count", 2, log.Count);
            AuditRecord first = log.Records[0];
            Check.That("audit_log_is_append_only_and_exports_records: first sequence is 1", first.Sequence == 1);
            Check.That("audit_log_is_append_only_and_exports_records: first timestamp", first.TimestampMs == 1_000);
            Check.That(
                "audit_log_is_append_only_and_exports_records: first target kind",
                first.TargetKind == AuditTargetKind.Resource);
            Check.Equal(
                "audit_log_is_append_only_and_exports_records: first target",
                "unitas://project/scenes",
                first.Target);
            Check.That(
                "audit_log_is_append_only_and_exports_records: first scope",
                first.Scope == Scope.ReadProject);
            Check.That(
                "audit_log_is_append_only_and_exports_records: first outcome",
                first.Outcome == new AuditOutcome.Allowed());
            Check.That(
                "audit_log_is_append_only_and_exports_records: first affected identities",
                first.AffectedIdentities.Count == 1 && first.AffectedIdentities[0] == "scene:Main");
            AuditRecord second = log.Records[1];
            Check.That("audit_log_is_append_only_and_exports_records: second sequence is 2", second.Sequence == 2);
            Check.That(
                "audit_log_is_append_only_and_exports_records: second outcome",
                second.Outcome == new AuditOutcome.RefusedMissingScope(Scope.EditContent));

            JsonArray exported = log.ExportJson();
            Check.Equal("audit_log_is_append_only_and_exports_records: export length", 2, exported.Count);
            JsonObject firstValue = (JsonObject)exported[0]!;
            Check.Equal(
                "audit_log_is_append_only_and_exports_records: export outcome detail",
                "allowed",
                firstValue["outcome_detail"]!.GetValue<string>());
        }
    }
}
