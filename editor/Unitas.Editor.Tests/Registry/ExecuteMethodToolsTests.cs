using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    // Probe methods resolved inside the copied test assembly that stands in as
    // the temp project's compiled script assembly.
    public static class ExecuteMethodProbe
    {
        public static string Ping() => "pong";
    }

    public sealed class ExecuteMethodProbeInstance
    {
        public int NotStatic() => 42;

        public static int StaticWithArgument(int value) => value;
    }

    public static class ExecuteMethodToolsTests
    {
        public static void Run()
        {
            ToolIsRegisteredWithExecuteProjectCodeScopeAndMethodSchema();
            InvokesProbeStaticMethodAndReturnsUntrustedEnvelope();
            ResolvesMethodByFullyQualifiedTypeName();
            RefusesUnknownMethod();
            RefusesNonStaticMethod();
            RefusesWrongSignature();
            RefusesWhenScriptAssemblyIsMissing();
            RefusesCallerWithoutExecuteProjectCodeScope();
            RecordsAnAuditEntryForTheInvocation();
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
        }

        private static void InvokesProbeStaticMethodAndReturnsUntrustedEnvelope()
        {
            string project = CreateProjectWithProbeAssembly("invoke");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                JsonObject envelope = CallExecuteMethod(host, sessionId, project, "ExecuteMethodProbe.Ping");

                Check.That("execute-method: result is an untrusted envelope", UntrustedContent.IsEnvelope(envelope));
                JsonObject content = (JsonObject)envelope["content"]!;
                Check.Equal("execute-method: ok is true", true, content["ok"]?.GetValue<bool>() ?? false);
                Check.Equal(
                    "execute-method: probe result is returned",
                    "pong",
                    content["result"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "execute-method: declaring type is reported",
                    typeof(ExecuteMethodProbe).FullName!,
                    content["declaringType"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void ResolvesMethodByFullyQualifiedTypeName()
        {
            string project = CreateProjectWithProbeAssembly("full-name");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                JsonObject envelope = CallExecuteMethod(
                    host,
                    sessionId,
                    project,
                    typeof(ExecuteMethodProbe).FullName + ".Ping");

                JsonObject content = (JsonObject)envelope["content"]!;
                Check.Equal("execute-method: fully-qualified name resolves", true, content["ok"]?.GetValue<bool>() ?? false);
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RefusesUnknownMethod()
        {
            string project = CreateProjectWithProbeAssembly("unknown");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                string? failure = CaptureFailure(host, sessionId, project, "ExecuteMethodProbe.Nope");
                Check.That(
                    "execute-method: unknown method is a structured refusal",
                    failure != null && failure.Contains("unknown method", StringComparison.Ordinal));

                failure = CaptureFailure(host, sessionId, project, "NoSuchType.Ping");
                Check.That(
                    "execute-method: unknown type is a structured refusal",
                    failure != null && failure.Contains("unknown method", StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RefusesNonStaticMethod()
        {
            string project = CreateProjectWithProbeAssembly("non-static");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                string? failure = CaptureFailure(host, sessionId, project, "ExecuteMethodProbeInstance.NotStatic");
                Check.That(
                    "execute-method: non-static method is a structured refusal",
                    failure != null && failure.Contains("not static", StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void RefusesWrongSignature()
        {
            string project = CreateProjectWithProbeAssembly("wrong-signature");
            try
            {
                using McpHost host = CreateHost(grantExecuteProjectCode: true, out string sessionId);
                string? failure = CaptureFailure(host, sessionId, project, "ExecuteMethodProbeInstance.StaticWithArgument");
                Check.That(
                    "execute-method: parameterized method is a structured refusal",
                    failure != null && failure.Contains("parameterless", StringComparison.Ordinal));
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
                string? failure = CaptureFailure(host, sessionId, project, "ExecuteMethodProbe.Ping");
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
                string? failure = CaptureFailure(host, sessionId, project, "ExecuteMethodProbe.Ping");
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
                CallExecuteMethod(host, sessionId, project, "ExecuteMethodProbe.Ping");

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

        private static McpHost CreateHost(bool grantExecuteProjectCode, out string sessionId)
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            ExecuteMethodTools.Register(host);
            string created = host.CreateSession(new SessionIdentity("agent-a", "cli", "person-1"));
            if (grantExecuteProjectCode)
            {
                host.GrantScope(created, Scope.ExecuteProjectCode, GrantAuthority.Person);
            }

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
