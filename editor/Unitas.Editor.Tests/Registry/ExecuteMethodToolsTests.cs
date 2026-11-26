using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Bridge.Scripting;
using Unitas.Editor.Mcp;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.Editor.Tests
{
    // Probe method the editor resolves through the compile-time registry, the
    // way the generated glue registers an [EditorCallableMethod] script method.
    public static class ExecuteMethodProbe
    {
        [EditorCallableMethod]
        public static string Ping() => "pong";
    }

    public static class ExecuteMethodToolsTests
    {
        private const string ProbeMethodName = "Unitas.Editor.Tests.ExecuteMethodProbe.Ping";

        public static void Run()
        {
            ToolIsRegisteredWithExecuteProjectCodeScopeAndMethodSchema();
            InvokesRegisteredProbeMethodAndReturnsUntrustedEnvelope();
            RefusesMethodThatIsNotRegistered();
            RefusesWhenScriptAssemblyIsMissing();
            RefusesCallerWithoutExecuteProjectCodeScope();
            RecordsAnAuditEntryForTheInvocation();
            RegistryDroppedMethodsAreNotInvoked();
            DiscoversEditorCallableMethodsAndEmitsGlueRegistration();
            TheLiveHostRegistersTheExecuteMethodTool();
        }

        private static void TheLiveHostRegistersTheExecuteMethodTool()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            using PlaySessionManager playSessions = new PlaySessionManager();
            BuiltinDefaults.RegisterAll(host, playSessions);

            ToolDefinition? definition = null;
            foreach (ToolDefinition candidate in host.ListTools())
            {
                if (candidate.Name == "editor.execute_method")
                {
                    definition = candidate;
                    break;
                }
            }

            Check.That("execute-method: the live host registers the tool", definition != null);
            Check.Equal(
                "execute-method: the live host gates the tool with execute-project-code",
                Scopes.AsString(Scope.ExecuteProjectCode),
                Scopes.AsString(definition!.RequiredScope));
        }

        private static void ToolIsRegisteredWithExecuteProjectCodeScopeAndMethodSchema()
        {
            using McpHost host = CreateHost(grantExecuteProjectCode: false, out _);
            ToolDefinition? definition = null;
            foreach (ToolDefinition candidate in host.ListTools())
            {
                if (candidate.Name == "editor.execute_method")
                {
                    definition = candidate;
                    break;
                }
            }

            Check.That("execute-method: tool is registered", definition != null);
            Check.Equal(
                "execute-method: scope gate is ExecuteProjectCode",
                Scopes.AsString(Scope.ExecuteProjectCode),
                Scopes.AsString(definition!.RequiredScope));
            JsonObject schema = (JsonObject)definition.InputSchema;
            Check.Equal("execute-method: schema requires method", "method", schema["required"]![0]!.GetValue<string>());
            Check.That(
                "execute-method: schema declares method as string",
                schema["properties"]?["method"]?["type"]?.GetValue<string>() == "string");
            Check.That(
                "execute-method: schema declares projectPath as string",
                schema["properties"]?["projectPath"]?["type"]?.GetValue<string>() == "string");
            Check.That(
                "execute-method: description names the EditorCallableMethod annotation",
                definition.Description.Contains("EditorCallableMethod", StringComparison.Ordinal));
        }

        private static void InvokesRegisteredProbeMethodAndReturnsUntrustedEnvelope()
        {
            string project = CreateProjectWithProbeAssembly("invoke");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                JsonObject envelope = CallExecuteMethod(host, sessionId, project, ProbeMethodName);

                Check.That("execute-method: result is an untrusted envelope", UntrustedContent.IsEnvelope(envelope));
                JsonObject content = (JsonObject)envelope["content"]!;
                Check.Equal("execute-method: ok is true", true, content["ok"]?.GetValue<bool>() ?? false);
                Check.Equal(
                    "execute-method: probe result is returned",
                    "pong",
                    content["result"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "execute-method: declaring type is reported",
                    "Unitas.Editor.Tests.ExecuteMethodProbe",
                    content["declaringType"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RefusesMethodThatIsNotRegistered()
        {
            string project = CreateProjectWithProbeAssembly("unregistered");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                string? failure = CaptureFailure(host, sessionId, project, "Unitas.Editor.Tests.ExecuteMethodProbe.Nope");
                Check.That(
                    "execute-method: unregistered method is a structured refusal",
                    failure != null && failure.Contains("EditorCallableMethod", StringComparison.Ordinal));

                failure = CaptureFailure(host, sessionId, project, "NoSuchType.Ping");
                Check.That(
                    "execute-method: unregistered type is a structured refusal",
                    failure != null && failure.Contains("EditorCallableMethod", StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RefusesWhenScriptAssemblyIsMissing()
        {
            string project = Path.Combine(
                Path.GetTempPath(),
                "unitas-execute-method-missing-assembly-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(project);
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                string? failure = CaptureFailure(host, sessionId, project, ProbeMethodName);
                Check.That(
                    "execute-method: missing script assembly is a structured refusal",
                    failure != null && failure.Contains("project script assembly not found", StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RefusesCallerWithoutExecuteProjectCodeScope()
        {
            string project = CreateProjectWithProbeAssembly("unscoped");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: false, out string sessionId);
                string? failure = CaptureFailure(host, sessionId, project, ProbeMethodName);
                Check.That(
                    "execute-method: caller without ExecuteProjectCode is refused",
                    failure != null && failure.Contains("scope", StringComparison.OrdinalIgnoreCase));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RecordsAnAuditEntryForTheInvocation()
        {
            string project = CreateProjectWithProbeAssembly("audit");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                CallExecuteMethod(host, sessionId, project, ProbeMethodName);

                AuditRecord? record = null;
                foreach (AuditRecord candidate in host.Audit.Records)
                {
                    if (candidate.TargetKind == AuditTargetKind.Tool
                        && candidate.Target == "editor.execute_method")
                    {
                        record = candidate;
                        break;
                    }
                }

                Check.That("execute-method: invocation is audited", record != null);
                Check.That(
                    "execute-method: audit outcome is allowed",
                    record!.Outcome is AuditOutcome.Allowed);
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RegistryDroppedMethodsAreNotInvoked()
        {
            EditorMethodRegistry.Clear();
            Check.That(
                "execute-method: a cleared registry holds no method",
                !EditorMethodRegistry.TryInvoke(ProbeMethodName, out object? _));
        }

        private static void DiscoversEditorCallableMethodsAndEmitsGlueRegistration()
        {
            string directory = Path.Combine(
                Path.GetTempPath(),
                "unitas-execute-method-discovery-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(directory);
            try
            {
                string sourcePath = Path.Combine(directory, "EditorScript.cs");
                File.WriteAllText(
                    sourcePath,
                    "namespace Sample.Editor\n"
                    + "{\n"
                    + "    public static class BuildScript\n"
                    + "    {\n"
                    + "        [EditorCallableMethod]\n"
                    + "        public static void PerformBuild() { }\n"
                    + "\n"
                    + "        [EditorCallableMethod]\n"
                    + "        public static string Describe() { return \"build\"; }\n"
                    + "    }\n"
                    + "}\n");

                List<EditorCallableMethodBinding> discovered =
                    ScriptAnalyzerAndGlueGenerator.DiscoverEditorCallableMethodsForAssembly(
                        new[] { sourcePath },
                        new HashSet<string>(StringComparer.Ordinal));
                Check.Equal("execute-method: two marked methods are discovered", 2, discovered.Count);
                Check.Equal(
                    "execute-method: the void method is discovered",
                    "Sample.Editor.BuildScript.PerformBuild",
                    discovered[0].FullTypeName + "." + discovered[0].MethodName);
                Check.That(
                    "execute-method: the void method is marked as void",
                    discovered[0].ReturnsVoid);
                Check.That(
                    "execute-method: the value method is marked as returning a value",
                    !discovered[1].ReturnsVoid);

                string glueSource = ScriptAnalyzerAndGlueGenerator.GenerateGlueSource(
                    Array.Empty<ScriptGuidBinding>(),
                    Array.Empty<SerializableTypeBinding>(),
                    Array.Empty<RuntimeInitializeMethodBinding>(),
                    discovered);
                Check.That(
                    "execute-method: the glue registers the marked method",
                    glueSource.Contains(
                        "EditorMethodRegistry.Register(\n                \"Sample.Editor.BuildScript.PerformBuild\"",
                        StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(directory, recursive: true);
            }
        }

        private static McpHost CreateHost(bool grantExecuteProjectCode, out string sessionId)
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            ExecuteMethodTools.Register(host);
            string created = host.CreateSession(new SessionIdentity("agent-a", "cli", "person-1"));
            if (grantExecuteProjectCode)
            {
                host.GrantScope(created, Scope.ExecuteProjectCode, GrantAuthority.Person);
            }

            EditorMethodRegistry.Clear();
            EditorMethodRegistry.Register(ProbeMethodName, () => ExecuteMethodProbe.Ping());
            sessionId = created;
            return host;
        }

        private static string CreateProjectWithProbeAssembly(string name)
        {
            string project = Path.Combine(
                Path.GetTempPath(),
                "unitas-execute-method-" + name + "-" + Guid.NewGuid().ToString("N"));
            string assemblyDirectory = Path.Combine(project, "Library", "UnitasScriptAssemblies");
            Directory.CreateDirectory(assemblyDirectory);
            File.Copy(
                typeof(ExecuteMethodProbe).Assembly.Location,
                Path.Combine(assemblyDirectory, "Assembly-CSharp.dll"),
                overwrite: true);
            return project;
        }

        private static JsonObject CallExecuteMethod(McpHost host, string sessionId, string project, string method)
        {
            return (JsonObject)host.CallTool(
                sessionId,
                "editor.execute_method",
                new JsonObject
                {
                    ["projectPath"] = project,
                    ["method"] = method,
                });
        }

        private static string? CaptureFailure(McpHost host, string sessionId, string project, string method)
        {
            try
            {
                CallExecuteMethod(host, sessionId, project, method);
                return null;
            }
            catch (McpError error)
            {
                return error.Message;
            }
        }
    }
}
