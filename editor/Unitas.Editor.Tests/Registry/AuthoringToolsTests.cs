using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class AuthoringToolsTests
    {
        public static void Run()
        {
            RegistersAuthoringToolsWithLeastPrivilegeScopes();
            ListReadWriteRoundTripRecordsAChangeSet();
            WriteOutsideTheProjectIsRefused();
            WriteRequiresTheEditScopes();
            CompileReportsScriptDiagnostics();
            TestRunReturnsAPassFailSummary();
            HeadlessCliRunTestsIsSupported();
        }

        private static void RegistersAuthoringToolsWithLeastPrivilegeScopes()
        {
            using McpHost host = NewHost();
            ChangeSetStore store = new ChangeSetStore(host);
            ProjectTools.Register(host, store);
            ScriptTools.Register(host);
            TestTools.Register(host);

            Check.Equal(
                "authoring: three project tools and two pipeline tools are registered",
                5,
                host.ListTools().Count);
            Check.That(
                "authoring: project.list requires read-project",
                Scope.ReadProject == ScopeOf(host, "project.list"));
            Check.That(
                "authoring: project.read_file requires read-project",
                Scope.ReadProject == ScopeOf(host, "project.read_file"));
            Check.That(
                "authoring: project.write_file requires edit-content",
                Scope.EditContent == ScopeOf(host, "project.write_file"));
            Check.That(
                "authoring: scripts.compile requires edit-scripts",
                Scope.EditScripts == ScopeOf(host, "scripts.compile"));
            Check.That(
                "authoring: tests.run requires execute-project-code",
                Scope.ExecuteProjectCode == ScopeOf(host, "tests.run"));
        }

        private static void ListReadWriteRoundTripRecordsAChangeSet()
        {
            string project = CreateProject("roundtrip", false);
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
                host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);

                JsonNode listEnvelope = host.CallTool(
                    session,
                    "project.list",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                    });
                JsonArray? entries = listEnvelope["content"]?["entries"] as JsonArray;
                Check.That("authoring: project.list returns entries", (entries?.Count ?? 0) > 0);

                JsonNode writeEnvelope = host.CallTool(
                    session,
                    "project.write_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Notes.md",
                        ["content"] = "first draft\n",
                    });
                Check.Equal(
                    "authoring: the write reports applied",
                    "applied",
                    writeEnvelope["content"]?["status"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "authoring: the write reaches the project file",
                    "first draft\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Notes.md")));

                JsonNode readEnvelope = host.CallTool(
                    session,
                    "project.read_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Notes.md",
                    });
                Check.Equal(
                    "authoring: the read returns the written text",
                    "first draft\n",
                    readEnvelope["content"]?["content"]?.GetValue<string>() ?? string.Empty);

                Check.Equal(
                    "authoring: the write recorded one change set",
                    1,
                    store.All.Count);
                JsonObject? operation = store.All[0].Operations[0];
                Check.Equal(
                    "authoring: the change set names the written path",
                    "Assets/Notes.md",
                    operation?["path"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "authoring: the change set keeps the text before the write",
                    string.Empty,
                    operation?["before"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "authoring: the change set keeps the text after the write",
                    "first draft\n",
                    operation?["after"]?.GetValue<string>() ?? string.Empty);

                host.CallTool(
                    session,
                    "project.write_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Notes.md",
                        ["content"] = "second draft\n",
                    });
                Check.Equal(
                    "authoring: the second write records the previous text",
                    "first draft\n",
                    store.All[1].Operations[0]?["before"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void WriteOutsideTheProjectIsRefused()
        {
            string project = CreateProject("escape", false);
            string outside = Path.Combine(Path.GetDirectoryName(project) ?? project, "escaped.txt");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
                host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);
                host.GrantScope(session, Scope.EditScripts, GrantAuthority.Person);

                McpError? error = null;
                try
                {
                    host.CallTool(
                        session,
                        "project.write_file",
                        new JsonObject
                        {
                            ["projectPath"] = project,
                            ["path"] = "../escaped.txt",
                            ["content"] = "escaped\n",
                        });
                }
                catch (McpError failure)
                {
                    error = failure;
                }

                Check.That(
                    "authoring: a write outside the project is refused",
                    error is McpError.InvalidRequest);
                Check.That(
                    "authoring: the refused write created no file outside the project",
                    !File.Exists(outside));
                Check.Equal(
                    "authoring: the refused write recorded no change set",
                    0,
                    store.All.Count);
            }
            finally
            {
                DeleteDirectory(project);
                if (File.Exists(outside))
                {
                    File.Delete(outside);
                }
            }
        }

        private static void WriteRequiresTheEditScopes()
        {
            string project = CreateProject("scopes", false);
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));

                McpError? readOnly = null;
                try
                {
                    host.CallTool(
                        session,
                        "project.write_file",
                        new JsonObject
                        {
                            ["projectPath"] = project,
                            ["path"] = "Assets/Notes.md",
                            ["content"] = "text\n",
                        });
                }
                catch (McpError failure)
                {
                    readOnly = failure;
                }

                Check.That(
                    "authoring: a write without edit-content is refused",
                    readOnly is McpError.MissingScope);
                Check.That(
                    "authoring: the refusal names the edit-content scope",
                    readOnly != null && readOnly.Message.Contains("edit-content", StringComparison.Ordinal));

                host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);
                McpError? scriptOnly = null;
                try
                {
                    host.CallTool(
                        session,
                        "project.write_file",
                        new JsonObject
                        {
                            ["projectPath"] = project,
                            ["path"] = "Assets/Scripts/Extra.cs",
                            ["content"] = "public class Extra { }\n",
                        });
                }
                catch (McpError failure)
                {
                    scriptOnly = failure;
                }

                Check.That(
                    "authoring: a script write without edit-scripts is refused",
                    scriptOnly is McpError.MissingScope);
                Check.That(
                    "authoring: the script refusal names the edit-scripts scope",
                    scriptOnly != null && scriptOnly.Message.Contains("edit-scripts", StringComparison.Ordinal));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void CompileReportsScriptDiagnostics()
        {
            string project = CreateProject("compile", true);
            try
            {
                using McpHost host = NewHost();
                ScriptTools.Register(host);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
                host.GrantScope(session, Scope.EditScripts, GrantAuthority.Person);

                JsonNode envelope = host.CallTool(
                    session,
                    "scripts.compile",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                    });
                Check.Equal(
                    "authoring: the compile reports failure",
                    false,
                    envelope["content"]?["ok"]?.GetValue<bool>() ?? true);
                JsonArray? diagnostics = envelope["content"]?["diagnostics"] as JsonArray;
                Check.That(
                    "authoring: the compile reports a diagnostic",
                    (diagnostics?.Count ?? 0) > 0);
                string identifier = diagnostics?[0]?["identifier"]?.GetValue<string>() ?? string.Empty;
                Check.That(
                    "authoring: the diagnostic carries a compiler identifier",
                    identifier.StartsWith("CS", StringComparison.Ordinal));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void TestRunReturnsAPassFailSummary()
        {
            string project = CreateProject("tests", false);
            try
            {
                using McpHost host = NewHost();
                TestTools.Register(host);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
                host.GrantScope(session, Scope.ExecuteProjectCode, GrantAuthority.Person);

                JsonNode envelope = host.CallTool(
                    session,
                    "tests.run",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["testPlatform"] = "EditMode",
                    });
                Check.Equal(
                    "authoring: the test run passes",
                    true,
                    envelope["content"]?["ok"]?.GetValue<bool>() ?? false);
                Check.Equal(
                    "authoring: the test run counts the passing test",
                    1,
                    envelope["content"]?["passed"]?.GetValue<int>() ?? 0);
                Check.Equal(
                    "authoring: the test run counts no failure",
                    0,
                    envelope["content"]?["failed"]?.GetValue<int>() ?? -1);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void HeadlessCliRunTestsIsSupported()
        {
            string project = CreateProject("cli", false);
            string stateRoot = Path.Combine(Path.GetTempPath(), "unitas-u1-state-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(stateRoot);
            string? previousStateDir = Environment.GetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable);
            try
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, stateRoot);
                string[] args =
                {
                    "-batchmode",
                    "-nographics",
                    "-quit",
                    "-projectPath",
                    project,
                    "-runTests",
                    "-testPlatform",
                    "EditMode",
                };
                using McpHost host = BuiltinDefaults.CreateStartupHost(args);
                CliResult result = HeadlessCli.Execute(host, args);
                Check.Equal(
                    "authoring: the command line runs the project tests",
                    CliExitCode.Success,
                    result.ExitCode);
                Check.Equal(
                    "authoring: the command line reports the tests operation as supported",
                    "succeeded",
                    result.Operations.Count > 1 ? result.Operations[1].Outcome : string.Empty);
            }
            finally
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, previousStateDir);
                DeleteDirectory(project);
                DeleteDirectory(stateRoot);
            }
        }

        private static McpHost NewHost()
        {
            return new McpHost(new FixedClock(1_700_000_000_000L));
        }

        private static Scope ScopeOf(McpHost host, string name)
        {
            foreach (ToolDefinition definition in host.ListTools())
            {
                if (string.Equals(definition.Name, name, StringComparison.Ordinal))
                {
                    return definition.RequiredScope;
                }
            }

            throw new InvalidOperationException("tool is not registered: " + name);
        }

        private static string CreateProject(string label, bool broken)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u1-" + label + "-" + Guid.NewGuid().ToString("N"));
            string runtimeDirectory = Path.Combine(root, "Assets", "Scripts");
            string testDirectory = Path.Combine(root, "Assets", "Tests", "EditMode");
            Directory.CreateDirectory(runtimeDirectory);
            Directory.CreateDirectory(testDirectory);

            File.WriteAllText(
                Path.Combine(runtimeDirectory, "Fixture.Runtime.asmdef"),
                "{\n  \"name\": \"Fixture.Runtime\"\n}\n");
            File.WriteAllText(
                Path.Combine(runtimeDirectory, "Score.cs"),
                "namespace Fixture\n{\n    public static class Score\n    {\n        public static int Add(int a, int b) => a + b;\n    }\n}\n");
            File.WriteAllText(
                Path.Combine(testDirectory, "Fixture.Tests.EditMode.asmdef"),
                "{\n  \"name\": \"Fixture.Tests.EditMode\",\n  \"references\": [ \"Fixture.Runtime\" ],\n  \"includePlatforms\": [ \"Editor\" ],\n  \"defineConstraints\": [ \"UNITY_INCLUDE_TESTS\" ]\n}\n");
            File.WriteAllText(
                Path.Combine(testDirectory, "ScoreTests.cs"),
                "using NUnit.Framework;\nusing Fixture;\nnamespace Fixture.Tests\n{\n    public class ScoreTests\n    {\n        [Test]\n        public void Adds()\n        {\n            Assert.AreEqual(3, Score.Add(1, 2));\n        }\n    }\n}\n");

            if (broken)
            {
                File.WriteAllText(
                    Path.Combine(runtimeDirectory, "Broken.cs"),
                    "namespace Fixture\n{\n    public class Broken\n    {\n        public int Value = \"not an int\";\n    }\n}\n");
            }

            return root;
        }

        private static void DeleteDirectory(string directory)
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, true);
            }
        }
    }
}
