using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class NativeToolBridgeTests
    {
        public static void Run()
        {
            AbsentNativeLibraryYieldsNamedNonFatalOutcome();
            ManifestParserValidatesContractAndRejectsUnknownScope();
            SourceRegistrationScopeGatesEnvelopesAndAuditsNativeTools();
            NativeErrorPayloadsMapToStructuredMcpErrorsAndAuditFailure();
            LiveFfiLibraryDescribesRegistersAndInvokesRustNativeTool();
        }

        private static void AbsentNativeLibraryYieldsNamedNonFatalOutcome()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            string missingPath = "/nonexistent/unitas/path/libunitas_editor_abi_missing.dylib";

            NativeLoadOutcome outcome = host.TryRegisterNativeLibrary(missingPath);
            Check.That(
                "native-bridge: missing library returns LibraryNotFound",
                outcome.IsLibraryNotFound);
            Check.Equal(
                "native-bridge: missing library status is named non-fatal outcome",
                "native-library-not-found",
                outcome.Status);
            Check.Equal(
                "native-bridge: no tools registered when library is absent",
                0,
                host.ListTools().Count);
        }

        private static void ManifestParserValidatesContractAndRejectsUnknownScope()
        {
            string validManifest = "{\"tools\":[{"
                + "\"name\":\"scene.list\","
                + "\"description\":\"Inspect scenes in project\","
                + "\"inputSchema\":{\"type\":\"object\",\"properties\":{}},"
                + "\"requiredScope\":\"read-project\","
                + "\"origin\":\"unitas://native/scene\"}]}";

            IReadOnlyList<NativeToolDescriptor> descriptors = NativeToolBridge.ParseManifest(validManifest);
            Check.Equal("native-bridge: manifest parses one tool", 1, descriptors.Count);
            Check.Equal("native-bridge: parsed tool name", "scene.list", descriptors[0].Name);
            Check.That(
                "native-bridge: parsed tool requiredScope",
                descriptors[0].RequiredScope == Scope.ReadProject);
            Check.Equal("native-bridge: parsed tool origin", "unitas://native/scene", descriptors[0].Origin);

            string unknownScopeManifest = "{\"tools\":[{"
                + "\"name\":\"scene.nuke\","
                + "\"description\":\"Invalid scope tool\","
                + "\"inputSchema\":{},"
                + "\"requiredScope\":\"root-access\","
                + "\"origin\":\"unitas://native/scene\"}]}";

            McpError? rejected = Catch(() => NativeToolBridge.ParseManifest(unknownScopeManifest));
            Check.That(
                "native-bridge: unknown scope in manifest is rejected as ParseError",
                rejected is McpError.ParseError);
        }

        private static void SourceRegistrationScopeGatesEnvelopesAndAuditsNativeTools()
        {
            FakeNativeLibrarySource source = new FakeNativeLibrarySource(
                "fake://libunitas_editor_abi",
                "{\"tools\":["
                + "{\"name\":\"native.inspect\",\"description\":\"Inspect native state\","
                + "\"inputSchema\":{\"type\":\"object\"},\"requiredScope\":\"read-project\","
                + "\"origin\":\"unitas://native/inspect\"},"
                + "{\"name\":\"native.build\",\"description\":\"Build native player\","
                + "\"inputSchema\":{\"type\":\"object\"},\"requiredScope\":\"build\","
                + "\"origin\":\"unitas://native/build\"}]}",
                (name, argsJson) => "{\"tool\":\"" + name + "\",\"received\":" + argsJson + "}");

            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            NativeLoadOutcome outcome = host.RegisterNativeSource(source);
            Check.That(
                "native-bridge: fake source loads cleanly",
                outcome.IsLoaded);
            Check.Equal("native-bridge: two native tools registered", 2, host.ListTools().Count);

            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "person-1"));
            JsonNode inspectResult = host.CallTool(
                session,
                "native.inspect",
                new JsonObject { ["filter"] = "cameras" });

            Check.That(
                "native-bridge: native inspect output is wrapped in untrusted envelope",
                UntrustedContent.IsEnvelope(inspectResult));
            Check.Equal(
                "native-bridge: envelope carries descriptor origin",
                "unitas://native/inspect",
                inspectResult["origin"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "native-bridge: payload carries native tool return value",
                "cameras",
                inspectResult["content"]?["received"]?["filter"]?.GetValue<string>() ?? string.Empty);

            McpError? buildRefused = Catch(() => host.CallTool(session, "native.build", new JsonObject()));
            Check.That(
                "native-bridge: build tool refused without build scope",
                buildRefused is McpError.MissingScope);

            host.GrantScope(session, Scope.Build, GrantAuthority.Person);
            JsonNode buildAllowed = host.CallTool(session, "native.build", new JsonObject { ["profile"] = "dev" });
            Check.That(
                "native-bridge: build tool succeeds after scope grant",
                UntrustedContent.IsEnvelope(buildAllowed));

            IReadOnlyList<AuditRecord> records = host.Audit.Records;
            Check.Equal("native-bridge: four audit records recorded", 4, records.Count);
            Check.That("native-bridge: first call audited allowed", records[0].Outcome is AuditOutcome.Allowed);
            Check.That(
                "native-bridge: second call audited missing scope",
                records[1].Outcome is AuditOutcome.RefusedMissingScope);
            Check.That("native-bridge: fourth call audited allowed", records[3].Outcome is AuditOutcome.Allowed);
        }

        private static void NativeErrorPayloadsMapToStructuredMcpErrorsAndAuditFailure()
        {
            FakeNativeLibrarySource source = new FakeNativeLibrarySource(
                "fake://libunitas_editor_abi",
                "{\"tools\":["
                + "{\"name\":\"native.fail\",\"description\":\"Failing native tool\","
                + "\"inputSchema\":{\"type\":\"object\"},\"requiredScope\":\"read-project\","
                + "\"origin\":\"unitas://native/fail\"}]}",
                (name, argsJson) =>
                {
                    if (argsJson.Contains("bad-args", StringComparison.Ordinal))
                    {
                        return "{\"error\":{\"kind\":\"invalid-args\",\"message\":\"bad argument shape\"}}";
                    }

                    return "{\"error\":{\"kind\":\"tool-failed\",\"message\":\"native subsystem fault\"}}";
                });

            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            host.RegisterNativeSource(source);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "person-1"));

            McpError? invalidArgs = Catch(() => host.CallTool(
                session,
                "native.fail",
                new JsonObject { ["mode"] = "bad-args" }));
            Check.That(
                "native-bridge: invalid-args maps to McpError.InvalidParams",
                invalidArgs is McpError.InvalidParams);

            McpError? toolFailed = Catch(() => host.CallTool(
                session,
                "native.fail",
                new JsonObject { ["mode"] = "explode" }));
            Check.That(
                "native-bridge: tool-failed maps to McpError.InvalidRequest",
                toolFailed is McpError.InvalidRequest);

            IReadOnlyList<AuditRecord> records = host.Audit.Records;
            Check.Equal("native-bridge: both native failures audited", 2, records.Count);
            Check.That("native-bridge: first failure recorded", records[0].Outcome is AuditOutcome.Failed);
            Check.That("native-bridge: second failure recorded", records[1].Outcome is AuditOutcome.Failed);
        }

        private static void LiveFfiLibraryDescribesRegistersAndInvokesRustNativeTool()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            NativeLoadOutcome outcome = host.TryRegisterNativeLibrary();
            Check.That(
                "native-bridge: live libunitas_editor_abi loads from build output",
                outcome.IsLoaded);
            if (!outcome.IsLoaded)
            {
                return;
            }

            Check.That(
                "native-bridge: live library registers at least one tool",
                outcome.ToolCount >= 1);

            string session = host.CreateSession(new SessionIdentity("agent-live", "stdio", "person-live"));
            JsonNode envelope = host.CallTool(
                session,
                "engine.abi_status",
                new JsonObject { ["ping"] = "pong" });

            Check.That(
                "native-bridge: live FFI invocation returns untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            Check.Equal(
                "native-bridge: live FFI envelope origin matches Rust descriptor",
                "unitas://native/editor-abi",
                envelope["origin"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "native-bridge: live FFI payload reports unitas-editor-abi ready",
                "ready",
                envelope["content"]?["status"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "native-bridge: live FFI payload echoes JSON arguments across C ABI",
                "pong",
                envelope["content"]?["echo"]?["ping"]?.GetValue<string>() ?? string.Empty);

            IReadOnlyList<AuditRecord> records = host.Audit.Records;
            Check.Equal("native-bridge: live FFI call recorded in audit log", 1, records.Count);
            Check.Equal("native-bridge: live FFI audit target", "engine.abi_status", records[0].Target);
            Check.That(
                "native-bridge: live FFI audit outcome is Allowed",
                records[0].Outcome is AuditOutcome.Allowed);
        }

        private static McpError? Catch(Func<JsonNode> action)
        {
            try
            {
                action();
                return null;
            }
            catch (McpError error)
            {
                return error;
            }
        }

        private static McpError? Catch(Action action)
        {
            try
            {
                action();
                return null;
            }
            catch (McpError error)
            {
                return error;
            }
        }

        private sealed class FakeNativeLibrarySource : INativeLibrarySource
        {
            private readonly string manifestJson;
            private readonly Func<string, string, string> invoker;

            public FakeNativeLibrarySource(
                string libraryPath,
                string manifestJson,
                Func<string, string, string> invoker)
            {
                LibraryPath = libraryPath;
                this.manifestJson = manifestJson;
                this.invoker = invoker;
            }

            public string LibraryPath { get; }

            public string DescribeJson()
            {
                return manifestJson;
            }

            public string InvokeJson(string name, string argsJson)
            {
                return invoker(name, argsJson);
            }
        }
    }
}
