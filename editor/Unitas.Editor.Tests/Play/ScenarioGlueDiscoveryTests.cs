using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Bridge.Scripting;
using Unitas.Editor.Mcp;
using UnityEngine;

namespace Unitas.Editor.Tests.Play
{
    public sealed class GlueProbeScript : MonoBehaviour
    {
    }

    public static class ScenarioGlueDiscoveryTests
    {
        private const string ProbeGuid = "0123456789abcdef0123456789abcdef";

        public static void Run()
        {
            RegistersProjectScriptGuidFromCompiledAssembly();
            ReportsMissingCompiledScriptAssembly();
            ReportsScriptGuidWithoutGlueType();
        }

        private static void RegistersProjectScriptGuidFromCompiledAssembly()
        {
            string project = CreateProject("register", out string scenePath);
            try
            {
                WriteScript(project, "GlueProbeScript", ProbeGuid);
                CopyCompiledScriptAssembly(project);

                using McpHost host = CreateScenarioHost(out string sessionId);
                RunScenario(host, sessionId, WriteScenario(project, scenePath));

                bool created = ScriptGlueRegistry.TryCreate(
                    ProbeGuid,
                    out UnityEngine.MonoBehaviour? instance,
                    out _);
                Check.That("glue-discovery: the discovered script guid creates its script type", created);
                Check.That(
                    "glue-discovery: the discovered script guid creates the project script type",
                    instance is GlueProbeScript);
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void ReportsMissingCompiledScriptAssembly()
        {
            string project = CreateProject("missing-assembly", out string scenePath);
            try
            {
                WriteScript(project, "GlueProbeScript", ProbeGuid);

                using McpHost host = CreateScenarioHost(out string sessionId);
                string? failure = CaptureFailure(host, sessionId, WriteScenario(project, scenePath));
                Check.That(
                    "glue-discovery: a missing compiled script assembly is a reported failure",
                    failure != null && failure.Contains("project script assembly not found", StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static void ReportsScriptGuidWithoutGlueType()
        {
            string project = CreateProject("unknown-guid", out string scenePath);
            try
            {
                WriteScript(project, "AbsentComponentScript", ProbeGuid);
                CopyCompiledScriptAssembly(project);

                using McpHost host = CreateScenarioHost(out string sessionId);
                string? failure = CaptureFailure(host, sessionId, WriteScenario(project, scenePath));
                Check.That(
                    "glue-discovery: a script guid without a glue type is a reported failure",
                    failure != null
                    && failure.Contains("does not contain the project's script types", StringComparison.Ordinal));
            }
            finally
            {
                Directory.Delete(project, recursive: true);
            }
        }

        private static string CreateProject(string name, out string scenePath)
        {
            string project = Path.Combine(
                Path.GetTempPath(),
                "unitas-glue-discovery-" + name + "-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path.Combine(project, "Assets", "Scripts"));
            Directory.CreateDirectory(Path.Combine(project, "Assets", "Scenes"));
            Directory.CreateDirectory(Path.Combine(project, "ProjectSettings"));
            scenePath = Path.Combine(project, "Assets", "Scenes", "Probe.unity");
            File.WriteAllText(scenePath, "%YAML 1.1\n");
            return project;
        }

        private static void WriteScript(string project, string scriptName, string guid)
        {
            string scriptPath = Path.Combine(project, "Assets", "Scripts", scriptName + ".cs");
            File.WriteAllText(
                scriptPath,
                "using UnityEngine;\n\npublic sealed class " + scriptName + " : MonoBehaviour\n{\n}\n");
            File.WriteAllText(
                scriptPath + ".meta",
                "fileFormatVersion: 2\nguid: " + guid + "\nMonoImporter:\n  externalObjects: {}\n");
        }

        private static void CopyCompiledScriptAssembly(string project)
        {
            string assemblyDirectory = Path.Combine(project, "Library", "UnitasScriptAssemblies");
            Directory.CreateDirectory(assemblyDirectory);
            File.Copy(
                typeof(ScenarioGlueDiscoveryTests).Assembly.Location,
                Path.Combine(assemblyDirectory, "Assembly-CSharp.dll"),
                overwrite: true);
        }

        private static string WriteScenario(string project, string scenePath)
        {
            string scenarioPath = Path.Combine(project, "probe.scenario.json");
            JsonObject scenario = new JsonObject
            {
                ["name"] = "glue-discovery",
                ["scene"] = scenePath,
                ["steps"] = new JsonArray(
                    new JsonObject
                    {
                        ["tick"] = new JsonObject
                        {
                            ["frames"] = 1,
                            ["delta_seconds"] = 0.02,
                        },
                    }),
                ["checkpoints"] = new JsonArray(),
            };
            File.WriteAllText(scenarioPath, scenario.ToJsonString());
            return scenarioPath;
        }

        private static McpHost CreateScenarioHost(out string sessionId)
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            ScenarioTools.Register(host);
            string created = host.CreateSession(new SessionIdentity("agent-a", "cli", "person-1"));
            host.GrantScope(created, Scope.ExecuteProjectCode, GrantAuthority.Person);
            sessionId = created;
            return host;
        }

        private static void RunScenario(McpHost host, string sessionId, string scenarioPath)
        {
            host.CallTool(
                sessionId,
                "scenario.run",
                new JsonObject { ["scenarioPath"] = scenarioPath });
        }

        private static string? CaptureFailure(McpHost host, string sessionId, string scenarioPath)
        {
            try
            {
                RunScenario(host, sessionId, scenarioPath);
                return null;
            }
            catch (McpError error)
            {
                return error.Message;
            }
        }
    }
}
