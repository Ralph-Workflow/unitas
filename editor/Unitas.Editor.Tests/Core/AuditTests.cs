using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class AuditTests
    {
        private static readonly string[] SceneMainIdentities = { "scene:Main" };

        public static void Run()
        {
            AuditLogIsAppendOnlyAndExportsRecords();
            AuditLogPersistsAcrossHostRestartOutsideProjectFiles();
            AuditExportRequiresPersonCredentialAndRefusesStdioOrMissingToken();
            AgentAttemptsToDeleteOrAlterAuditRecordsAreRefusedAndAudited();
        }

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
                SceneMainIdentities));
            log.Record(new AuditEntry(
                1_050,
                sessionId,
                identity,
                AuditTargetKind.Tool,
                "scene.edit",
                Scope.EditContent,
                new AuditOutcome.RefusedMissingScope(Scope.EditContent),
                Array.Empty<string>()));

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
            JsonObject secondValue = (JsonObject)exported[1]!;
            Check.Equal(
                "audit_log_is_append_only_and_exports_records: second export outcome",
                "refused-missing-scope",
                secondValue["outcome"]!.GetValue<string>());
            Check.Equal(
                "audit_log_is_append_only_and_exports_records: second export outcome detail",
                "refused: missing scope edit-content",
                secondValue["outcome_detail"]!.GetValue<string>());
        }

        private static void AuditLogPersistsAcrossHostRestartOutsideProjectFiles()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas-audit-test-" + Guid.NewGuid().ToString("N"));
            string projectDir = Path.Combine(tempRoot, "project");
            string stateRoot = Path.Combine(tempRoot, "state");
            Directory.CreateDirectory(projectDir);
            Directory.CreateDirectory(stateRoot);
            try
            {
                AuditStore firstStore = AuditStore.ForProject(projectDir, stateRoot);
                FixedClock clock = new FixedClock(2_000);
                McpHost firstHost = new McpHost(clock, PersonCredential.Absent, new AuditLog(firstStore));
                BuiltinDefaults.Register(firstHost);
                firstHost.RegisterTool(
                    new ToolDefinition(
                        "scene.touch",
                        "Touch scene",
                        new JsonObject
                        {
                            ["type"] = "object",
                            ["properties"] = new JsonObject(),
                        },
                        Scope.EditContent),
                    _ => ToolOutput.Untrusted("unitas://scene/main", new JsonObject()).WithAffectedIdentities("scene:Main"));

                string firstSession = firstHost.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                firstHost.CallTool(firstSession, "project.status", new JsonObject());

                clock.Advance(25);
                bool missingScopeRefused = false;
                try
                {
                    firstHost.CallTool(firstSession, "scene.touch", new JsonObject());
                }
                catch (McpError.MissingScope)
                {
                    missingScopeRefused = true;
                }

                Check.That("audit_persistence: missing scope refused", missingScopeRefused);
                Check.Equal(
                    "audit_persistence: zero provenance files written inside project dir",
                    0,
                    Directory.GetFileSystemEntries(projectDir).Length);
                Check.That("audit_persistence: store file exists in state dir", File.Exists(firstStore.AuditFilePath));

                AuditStore secondStore = AuditStore.ForProject(projectDir, stateRoot);
                clock.Advance(50);
                McpHost secondHost = new McpHost(clock, PersonCredential.Absent, new AuditLog(secondStore));
                BuiltinDefaults.Register(secondHost);

                Check.Equal("audit_persistence: reloaded count after restart", 2, secondHost.Audit.Count);
                AuditRecord loadedFirst = secondHost.Audit.Records[0];
                Check.That("audit_persistence: first sequence preserved", loadedFirst.Sequence == 1);
                Check.That("audit_persistence: first timestamp preserved", loadedFirst.TimestampMs == 2_000);
                Check.Equal("audit_persistence: first agent preserved", "agent-a", loadedFirst.Identity.Agent);
                Check.Equal("audit_persistence: first target preserved", "project.status", loadedFirst.Target);
                Check.That("audit_persistence: first scope preserved", loadedFirst.Scope == Scope.ReadProject);
                Check.That("audit_persistence: first outcome preserved", loadedFirst.Outcome == new AuditOutcome.Allowed());

                AuditRecord loadedSecond = secondHost.Audit.Records[1];
                Check.That("audit_persistence: second sequence preserved", loadedSecond.Sequence == 2);
                Check.That("audit_persistence: second timestamp preserved", loadedSecond.TimestampMs == 2_025);
                Check.Equal("audit_persistence: second target preserved", "scene.touch", loadedSecond.Target);
                Check.That(
                    "audit_persistence: second outcome preserved",
                    loadedSecond.Outcome == new AuditOutcome.RefusedMissingScope(Scope.EditContent));

                string secondSession = secondHost.CreateSession(new SessionIdentity("agent-b", "http", "person-1"));
                secondHost.CallTool(secondSession, "project.status", new JsonObject());
                Check.Equal("audit_persistence: count after post-restart call", 3, secondHost.Audit.Count);
                Check.That(
                    "audit_persistence: sequence continues from 3",
                    secondHost.Audit.Records[2].Sequence == 3);
            }
            finally
            {
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void AuditExportRequiresPersonCredentialAndRefusesStdioOrMissingToken()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas-audit-export-" + Guid.NewGuid().ToString("N"));
            string projectDir = Path.Combine(tempRoot, "project");
            string stateRoot = Path.Combine(tempRoot, "state");
            Directory.CreateDirectory(projectDir);
            Directory.CreateDirectory(stateRoot);
            try
            {
                AuditStore store = AuditStore.ForProject(projectDir, stateRoot);
                FixedClock clock = new FixedClock(5_000);
                McpHost host = new McpHost(
                    clock,
                    PersonCredential.FromToken("secret-person-token"),
                    new AuditLog(store));
                BuiltinDefaults.Register(host);
                AuditTools.Register(host);

                string httpSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));
                string stdioSession = host.CreateSession(new SessionIdentity("agent-cli", "stdio", "person-1"));

                host.CallTool(stdioSession, "project.status", new JsonObject());

                bool missingTokenRefused = false;
                try
                {
                    host.CallTool(httpSession, "audit.export", new JsonObject());
                }
                catch (McpError.PersonCredentialRequired)
                {
                    missingTokenRefused = true;
                }

                Check.That("audit_export: http without token is refused", missingTokenRefused);

                bool wrongTokenRefused = false;
                try
                {
                    host.CallTool(
                        httpSession,
                        "audit.export",
                        new JsonObject
                        {
                            ["personCredential"] = "wrong-token",
                        });
                }
                catch (McpError.PersonCredentialRequired)
                {
                    wrongTokenRefused = true;
                }

                Check.That("audit_export: http with wrong token is refused", wrongTokenRefused);

                McpError.PersonCredentialRequired? stdioError = null;
                try
                {
                    host.CallTool(
                        stdioSession,
                        "audit.export",
                        new JsonObject
                        {
                            ["personCredential"] = "secret-person-token",
                        });
                }
                catch (McpError.PersonCredentialRequired error)
                {
                    stdioError = error;
                }

                Check.That(
                    "audit_export: stdio session even with right token is refused",
                    stdioError != null
                    && stdioError.Message.Contains(
                        "requests over stdio are never treated as a person's",
                        StringComparison.Ordinal));

                bool insideProjectExportRefused = false;
                try
                {
                    host.CallTool(
                        httpSession,
                        "audit.export",
                        new JsonObject
                        {
                            ["personCredential"] = "secret-person-token",
                            ["outputPath"] = Path.Combine(projectDir, "audit-export.json"),
                        });
                }
                catch (McpError.InvalidParams)
                {
                    insideProjectExportRefused = true;
                }

                Check.That("audit_export: exporting into project directory is refused", insideProjectExportRefused);

                string exportFile = Path.Combine(tempRoot, "exported-audit.json");
                JsonNode envelope = host.CallTool(
                    httpSession,
                    "audit.export",
                    new JsonObject
                    {
                        ["personCredential"] = "secret-person-token",
                        ["outputPath"] = exportFile,
                    });

                JsonObject content = (JsonObject)((JsonObject)envelope)["content"]!;
                JsonArray records = (JsonArray)content["records"]!;
                Check.That("audit_export: exported records non-empty", records.Count >= 4);
                JsonObject firstRecord = (JsonObject)records[0]!;
                Check.That("audit_export: record has timestamp_ms", firstRecord["timestamp_ms"] != null);
                Check.That("audit_export: record has identity", firstRecord["identity"] is JsonObject);
                Check.Equal(
                    "audit_export: first record target",
                    "project.status",
                    firstRecord["target"]!.GetValue<string>());
                Check.Equal(
                    "audit_export: first record scope",
                    "read-project",
                    firstRecord["scope"]!.GetValue<string>());
                Check.Equal(
                    "audit_export: first record outcome",
                    "allowed",
                    firstRecord["outcome"]!.GetValue<string>());
                Check.That("audit_export: output file written outside project", File.Exists(exportFile));
            }
            finally
            {
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void AgentAttemptsToDeleteOrAlterAuditRecordsAreRefusedAndAudited()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas-audit-tamper-" + Guid.NewGuid().ToString("N"));
            string projectDir = Path.Combine(tempRoot, "project");
            string stateRoot = Path.Combine(tempRoot, "state");
            Directory.CreateDirectory(projectDir);
            Directory.CreateDirectory(stateRoot);
            try
            {
                bool insideProjectStoreRefused = false;
                try
                {
                    AuditStore.ForProject(projectDir, Path.Combine(projectDir, ".unitas-state"));
                }
                catch (McpError.InvalidParams)
                {
                    insideProjectStoreRefused = true;
                }

                Check.That("audit_tamper: placing AuditStore inside project dir is refused", insideProjectStoreRefused);

                AuditStore store = AuditStore.ForProject(projectDir, stateRoot);
                FixedClock clock = new FixedClock(9_000);
                McpHost host = new McpHost(clock, PersonCredential.Absent, new AuditLog(store));
                BuiltinDefaults.Register(host);
                AuditTools.Register(host);

                string session = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.CallTool(session, "project.status", new JsonObject());
                Check.Equal("audit_tamper: initial record count", 1, host.Audit.Count);

                bool deleteRefused = false;
                try
                {
                    host.CallTool(session, "audit.delete", new JsonObject());
                }
                catch (McpError.InvalidRequest)
                {
                    deleteRefused = true;
                }

                Check.That("audit_tamper: audit.delete is refused with InvalidRequest", deleteRefused);
                Check.Equal("audit_tamper: audit.delete appends failed record", 2, host.Audit.Count);
                Check.Equal(
                    "audit_tamper: audit.delete target recorded",
                    "audit.delete",
                    host.Audit.Records[1].Target);
                Check.Equal(
                    "audit_tamper: audit.delete outcome is failed",
                    "failed",
                    host.Audit.Records[1].Outcome.StatusString);

                bool clearRefused = false;
                try
                {
                    host.CallTool(session, "audit.clear", new JsonObject());
                }
                catch (McpError.InvalidRequest)
                {
                    clearRefused = true;
                }

                Check.That("audit_tamper: audit.clear is refused with InvalidRequest", clearRefused);
                Check.Equal("audit_tamper: audit.clear appends failed record", 3, host.Audit.Count);
                Check.Equal(
                    "audit_tamper: audit.clear outcome is failed",
                    "failed",
                    host.Audit.Records[2].Outcome.StatusString);
                Check.Equal(
                    "audit_tamper: original first record remains intact",
                    "project.status",
                    host.Audit.Records[0].Target);
            }
            finally
            {
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }
    }
}
