using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;
using UnityEditor;

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
            TheCommandPaletteListsTheRegisteredCommands();
            TheCommandPaletteRunsACommand();
            PlayModeRestoresTheEditTimeSceneState();
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

        private static void TheCommandPaletteListsTheRegisteredCommands()
        {
            using McpHost host = NewHost();
            ChangeSetStore store = new ChangeSetStore(host);
            SceneTools.Register(host, store);
            CommandPaletteTools.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);

            MenuItem.ClearRegisteredCommands();
            try
            {
                MenuItem.RegisterCommand("Tools/Export Report", () => { });

                JsonNode palette = host.CallTool(session, "editor.commands", new JsonObject());
                JsonArray? commands = palette["content"]?["commands"] as JsonArray;
                Check.That("authoring: the palette lists the commands", (commands?.Count ?? 0) > 0);

                bool foundTool = false;
                bool foundMenuCommand = false;
                bool unavailable = false;
                string disabledReason = string.Empty;
                foreach (JsonNode? entry in commands ?? new JsonArray())
                {
                    string id = entry?["id"]?.GetValue<string>() ?? string.Empty;
                    if (string.Equals(id, "scene.open", StringComparison.Ordinal))
                    {
                        foundTool = true;
                    }

                    if (string.Equals(entry?["menuPath"]?.GetValue<string>(), "Tools/Export Report", StringComparison.Ordinal))
                    {
                        foundMenuCommand = true;
                    }

                    if (string.Equals(id, "scene.set_property", StringComparison.Ordinal))
                    {
                        unavailable = entry?["available"]?.GetValue<bool>() == false;
                        disabledReason = entry?["disabledReason"]?.GetValue<string>() ?? string.Empty;
                    }
                }

                Check.That("authoring: the palette lists the commands of the tool registry", foundTool);
                Check.That("authoring: the palette lists the menu commands of the project", foundMenuCommand);
                Check.That("authoring: the palette marks a command the session may not run as unavailable", unavailable);
                Check.That(
                    "authoring: the palette names the reason a command is unavailable",
                    disabledReason.Contains("edit-content", StringComparison.Ordinal));
            }
            finally
            {
                MenuItem.ClearRegisteredCommands();
            }
        }

        private static void TheCommandPaletteRunsACommand()
        {
            using McpHost host = NewHost();
            CommandPaletteTools.Register(host);
            host.RegisterTool(
                new ToolDefinition(
                    "test.echo",
                    "Echo the value it is given",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                arguments => ToolOutput.Untrusted(
                    "unitas://test/echo",
                    new JsonObject
                    {
                        ["echoed"] = arguments["value"]?.GetValue<string>() ?? string.Empty,
                    }));
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);
            host.GrantScope(session, Scope.ExecuteProjectCode, GrantAuthority.Person);

            JsonNode invoked = host.CallTool(
                session,
                "editor.invoke_command",
                new JsonObject
                {
                    ["command"] = "test.echo",
                    ["arguments"] = new JsonObject
                    {
                        ["value"] = "hello",
                    },
                });
            Check.Equal(
                "authoring: the palette runs the command it names",
                "hello",
                invoked["content"]?["result"]?["content"]?["echoed"]?.GetValue<string>() ?? string.Empty);

            MenuItem.ClearRegisteredCommands();
            try
            {
                bool ran = false;
                MenuItem.RegisterCommand("Tools/Export Report", () => { ran = true; });
                JsonNode menuInvoked = host.CallTool(
                    session,
                    "editor.invoke_command",
                    new JsonObject
                    {
                        ["command"] = "Tools/Export Report",
                    });
                Check.Equal(
                    "authoring: the palette reports the menu command it ran",
                    "Tools/Export Report",
                    menuInvoked["content"]?["menuPath"]?.GetValue<string>() ?? string.Empty);
                Check.That("authoring: the menu command ran", ran);
            }
            finally
            {
                MenuItem.ClearRegisteredCommands();
            }

            McpError? refused = null;
            try
            {
                host.CallTool(
                    session,
                    "editor.invoke_command",
                    new JsonObject
                    {
                        ["command"] = "test.does_not_exist",
                    });
            }
            catch (McpError failure)
            {
                refused = failure;
            }

            Check.That("authoring: the palette refuses an unknown command", refused != null);
        }

        private static void PlayModeRestoresTheEditTimeSceneState()
        {
            string project = CreateProjectWithSampleScene("playmode");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                using StubWorker worker = new StubWorker();
                using PlaySessionManager manager = new PlaySessionManager(_ => worker);
                PlayTools.Register(host, manager);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
                host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);
                host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);
                host.GrantScope(session, Scope.ExecuteProjectCode, GrantAuthority.Person);

                host.CallTool(
                    session,
                    "scene.open",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Scenes/RollABall.unity",
                    });
                host.CallTool(
                    session,
                    "scene.set_property",
                    new JsonObject
                    {
                        ["fileId"] = 1000000092L,
                        ["path"] = "m_LocalPosition.x",
                        ["value"] = "2.25",
                    });

                JsonNode entered = host.CallTool(session, "play.enter", new JsonObject());
                Check.Equal(
                    "authoring: entering play mode captures the edit-time state",
                    "captured",
                    entered["content"]?["editState"]?["status"]?.GetValue<string>() ?? string.Empty);

                host.CallTool(
                    session,
                    "play.set_runtime_property",
                    new JsonObject
                    {
                        ["fileId"] = 1000000092L,
                        ["path"] = "m_LocalPosition.x",
                        ["value"] = "99",
                    });
                Check.Equal(
                    "authoring: the runtime change is in the scene while play mode runs",
                    "{x: 99, y: 0.75, z: 3}",
                    PositionOf(host, session, 1000000091L));

                JsonNode exited = host.CallTool(session, "play.exit", new JsonObject());
                Check.Equal(
                    "authoring: leaving play mode restores the edit-time state",
                    "restored",
                    exited["content"]?["editState"]?["status"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "authoring: the runtime change is gone after play mode",
                    "{x: 2.25, y: 0.75, z: 3}",
                    PositionOf(host, session, 1000000091L));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static string PositionOf(McpHost host, string session, long fileId)
        {
            JsonNode inspected = host.CallTool(
                session,
                "scene.inspect_object",
                new JsonObject
                {
                    ["fileId"] = fileId,
                });
            return inspected["content"]?["position"]?.GetValue<string>() ?? string.Empty;
        }

        private sealed class StubWorker : IPlayWorker
        {
            public bool HasExited => false;

            public int? ExitCode => null;

            public int? ProcessId => null;

            public JsonObject SendCommand(JsonObject command)
            {
                return new JsonObject
                {
                    ["state"] = PlaySession.StatePlaying,
                    ["frames"] = 0,
                    ["delta_seconds"] = 1.0 / 60.0,
                    ["random_seed"] = 1,
                };
            }

            public void Kill()
            {
            }

            public void Dispose()
            {
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

        private static string CreateProjectWithSampleScene(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u3-" + label + "-" + Guid.NewGuid().ToString("N"));
            string scenes = Path.Combine(root, "Assets", "Scenes");
            Directory.CreateDirectory(scenes);
            File.Copy(
                Path.Combine(ResolveRepoRoot(), "samples", "roll-a-ball", "Assets", "Scenes", "RollABall.unity"),
                Path.Combine(scenes, "RollABall.unity"));
            return root;
        }

        private static string ResolveRepoRoot()
        {
            DirectoryInfo? directory = new DirectoryInfo(Directory.GetCurrentDirectory());
            for (int depth = 0; depth < 12 && directory != null; depth++)
            {
                if (Directory.Exists(Path.Combine(directory.FullName, "samples", "roll-a-ball")))
                {
                    return directory.FullName;
                }

                directory = directory.Parent;
            }

            throw new DirectoryNotFoundException("repo root not found from current directory");
        }
    }
}
