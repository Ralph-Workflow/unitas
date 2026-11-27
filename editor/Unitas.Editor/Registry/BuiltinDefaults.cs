using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class BuiltinDefaults
    {
        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            host.RegisterTool(
                new ToolDefinition(
                    "project.status",
                    "Read current unitas project status summary",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                arguments => HandleProjectStatus(host, arguments));

            RegisterKnowledgeResources(host);
        }

        private static ToolOutput HandleProjectStatus(McpHost host, JsonNode arguments)
        {
            string? projectPath = ProjectAccess.OptionalString(arguments, "projectPath");
            if (projectPath != null)
            {
                host.OpenProject(projectPath);
            }

            string root = ProjectAccess.ResolveRoot(host, arguments, "project.status");
            JsonArray scenes = new JsonArray();
            int objectsTotal = 0;
            foreach (JsonObject scene in ScanScenes(root))
            {
                objectsTotal += scene["objects"]?.GetValue<int>() ?? 0;
                scenes.Add(scene);
            }

            return ToolOutput.Untrusted(
                "unitas://project/status",
                new JsonObject
                {
                    ["state"] = "ready",
                    ["projectPath"] = root,
                    ["scenes"] = scenes,
                    ["objectsTotal"] = objectsTotal,
                    ["trust"] = TrustState(host),
                });
        }

        private static void RegisterKnowledgeResources(McpHost host)
        {
            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://knowledge/scopes",
                    "AUT-007 Permission Scopes",
                    "Fixed permission scope catalog for unitas MCP sessions",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted("unitas://knowledge/scopes", BuildScopesPayload()));

            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://knowledge/grants",
                    "AUT-007 Grant Prompt and Security Boundaries",
                    "Mandatory grant-prompt disclosures, person-only actions, and security defaults for unitas MCP sessions",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted("unitas://knowledge/grants", BuildGrantsPayload()));

            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://knowledge/docs",
                    "AUT-006 Unitas Product Knowledge",
                    "Product documentation, API reference compatibility tiers, diagnostic explanations, and agent workflow guides",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted("unitas://knowledge/docs", BuildDocsPayload()));
        }

        private static JsonObject BuildScopesPayload()
        {
            JsonArray scopes = new JsonArray();
            foreach (Scope scope in Scopes.All)
            {
                scopes.Add(Scopes.AsString(scope));
            }

            return new JsonObject
            {
                ["version"] = "0.1.0",
                ["defaultScope"] = Scopes.AsString(Scope.ReadProject),
                ["scopes"] = scopes,
                ["descriptions"] = BuildScopeDescriptions(),
            };
        }

        private static JsonObject BuildScopeDescriptions()
        {
            return new JsonObject
            {
                ["read-project"] = "Inspect project content, runtime state, logs, and evidence",
                ["edit-content"] = "Change scenes, prefabs, assets, and their serialized values",
                ["edit-scripts"] = "Create, change, and delete scripts and assembly definitions",
                ["execute-project-code"] = "Enter play mode, run tests and scenarios, invoke editor methods, and run project-contributed commands",
                ["import-external-files"] = "Bring files from outside the project folder into the project",
                ["change-packages-and-settings"] = "Change package manifests and project settings",
                ["build"] = "Create player and content builds",
                ["sign"] = "Apply signing identities to builds",
                ["deploy"] = "Install and run builds on connected devices",
                ["external-network"] = "Have unitas make requests outside the local machine on the agent session's behalf",
                ["review"] = "Accept, approve, reject, and revert change sets of other agent sessions",
                ["edit-scenes"] = "Change scenes and their serialized hierarchy and component values",
                ["edit-assets"] = "Create, import, move, duplicate, and delete project assets",
                ["run-play-and-tests"] = "Run play sessions, project tests, and declared scenarios",
                ["build-player"] = "Package scenes and assets into player bundles",
                ["manage-packages-and-settings"] = "Update project settings and package manifests",
            };
        }

        private static JsonObject BuildGrantsPayload()
        {
            return new JsonObject
            {
                ["version"] = "0.1.0",
                ["grantDisclosures"] = new JsonArray(
                    "Holding both the edit-scripts and execute-project-code scopes amounts to running arbitrary code with the privileges of the operating-system account running unitas.",
                    "The execute-project-code scope allows project code that the agent session runs to read files marked secret.",
                    "The MCP permission model constrains only what agents do through MCP; an agent that also has shell or file access as the same operating-system account can act outside it."),
                ["personOnlyActions"] = new JsonArray(
                    "permissions.person_grant",
                    "audit.delete",
                    "audit.clear",
                    "audit.alter",
                    "trust.approve",
                    "credentials.write"),
                ["defaults"] = new JsonObject
                {
                    ["reviewPolicy"] = "require-approval",
                    ["httpOrigins"] = new JsonArray("loopback"),
                    ["standingGrants"] = new JsonArray(),
                    ["crashReportDestinations"] = new JsonArray(),
                },
            };
        }

        private static JsonObject BuildDocsPayload()
        {
            return new JsonObject
            {
                ["version"] = "0.1.0",
                ["unityBaseline"] = "2022.3.62f1",
                ["documents"] = BuildProductDocuments(),
                ["apiReference"] = BuildApiReference(),
                ["diagnostics"] = BuildDiagnosticGuides(),
                ["guides"] = BuildWorkflowGuides(),
            };
        }

        private static JsonArray BuildProductDocuments()
        {
            return new JsonArray(
                DocEntry("automation-and-mcp", "MCP capability parity, discovery, structured inspection/editing, product knowledge, and permissions", "AUT-001..AUT-008"),
                DocEntry("editor-and-authoring", "Workspace shell, scene authoring, undo history, and change-set review policy", "EDT-001..EDT-008"),
                DocEntry("compatibility-contract", "Unity 2022.3.62f1 baseline, compatibility tiers, and schema versioning", "CMP-001..CMP-005"),
                DocEntry("scripting-runtime", "C# compilation, MonoBehaviour lifecycle, hot reload, and deterministic frame stepping", "RUN-001..RUN-007"),
                DocEntry("diagnostics-profiling-playtesting", "Structured diagnostics, debug adapter protocol, scenarios, tests, and captures", "DIA-001..DIA-003, TST-001..TST-003, PRF-001..PRF-002"),
                DocEntry("build-and-distribution", "Player bundle packaging, headless CLI batch invocation, and licensing records", "BLD-001..BLD-002, LIC-001"));
        }

        private static JsonObject DocEntry(string id, string summary, string requirements)
        {
            return new JsonObject
            {
                ["id"] = id,
                ["summary"] = summary,
                ["requirements"] = requirements,
            };
        }

        private static JsonObject BuildApiReference()
        {
            return new JsonObject
            {
                ["tiers"] = new JsonArray("Exact", "Semantic", "Familiar", "Native"),
                ["namespaces"] = new JsonArray(
                    "UnityEngine",
                    "UnityEngine.SceneManagement",
                    "UnityEngine.UI",
                    "UnityEngine.InputSystem",
                    "TMPro"),
            };
        }

        private static JsonObject BuildDiagnosticGuides()
        {
            return new JsonObject
            {
                ["CS0103"] = DiagEntry("The name does not exist in the current context", "Compilation fails and the project continues running the last valid compilation", "Declare the missing symbol or add the required using directive and re-run scripts.compile"),
                ["CS0029"] = DiagEntry("Cannot implicitly convert type", "Compilation fails and no new assembly is loaded", "Match the target type or add an explicit conversion and re-run scripts.compile"),
                ["permission-denied"] = DiagEntry("The session lacks the required permission scope for the tool or resource", "The request is refused with -32001 and changes nothing", "Call permissions.request_grant or have a person grant the missing scope via permissions.person_grant"),
                ["person-only-action"] = DiagEntry("The requested action requires an interactive person credential", "The request is refused and recorded in the audit log", "Present the request to a person to execute with a valid personCredential over a non-stdio transport"),
                ["untrusted-project"] = DiagEntry("The project has not been trusted by a person (PRJ-008)", "Execution of project code is blocked while read-only inspection remains available", "Ask a person to grant project trust in the visual editor or interactive terminal"),
            };
        }

        private static JsonObject DiagEntry(string meaning, string consequence, string remediation)
        {
            return new JsonObject
            {
                ["meaning"] = meaning,
                ["consequence"] = consequence,
                ["remediation"] = remediation,
            };
        }

        private static JsonObject BuildWorkflowGuides()
        {
            return new JsonObject
            {
                ["scene-authoring-and-review"] = new JsonArray(
                    "Open or inspect the scene with scene.open, scene.hierarchy, and scene.inspect_object.",
                    "Apply reversible edits with scene.set_property, scene.create_object, or scene.duplicate_object.",
                    "Inspect pending change sets with changes.list and changes.show; a person approves or rejects under require-approval."),
                ["play-and-capture-verification"] = new JsonArray(
                    "Start an isolated play session with play.enter after holding execute-project-code.",
                    "Advance deterministically with play.step or inject input with play.inject_input.",
                    "Capture visual and object-ID evidence with capture, then stop the worker with play.exit."),
                ["script-edit-and-hot-reload"] = new JsonArray(
                    "Write or update C# scripts with project.write_file or assets.import under edit-scripts.",
                    "Run scripts.compile or scripts.diagnose and inspect structured diagnostics.",
                    "Verify behavior with tests.run or a play session; note that shell/file access outside MCP is not constrained by MCP scopes (AUT-007)."),
            };
        }

        private static IEnumerable<JsonObject> ScanScenes(string root)
        {
            string assets = Path.Combine(root, "Assets");
            string start = Directory.Exists(assets) ? assets : root;
            List<string> sceneFiles = new List<string>();
            CollectSceneFiles(start, sceneFiles);
            sceneFiles.Sort(StringComparer.Ordinal);
            foreach (string sceneFile in sceneFiles)
            {
                yield return new JsonObject
                {
                    ["path"] = sceneFile.Substring(root.Length).TrimStart('/', '\\').Replace('\\', '/'),
                    ["name"] = Path.GetFileNameWithoutExtension(sceneFile),
                    ["objects"] = CountGameObjects(sceneFile),
                };
            }
        }

        private static void CollectSceneFiles(string directory, List<string> sceneFiles)
        {
            string[] entries;
            try
            {
                entries = Directory.GetFiles(directory, "*.unity");
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                return;
            }

            sceneFiles.AddRange(entries);

            string[] children;
            try
            {
                children = Directory.GetDirectories(directory);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                return;
            }

            foreach (string child in children)
            {
                if (ProjectTools.IsGeneratedDirectory(Path.GetFileName(child)))
                {
                    continue;
                }

                CollectSceneFiles(child, sceneFiles);
            }
        }

        private static int CountGameObjects(string sceneFile)
        {
            int count = 0;
            try
            {
                foreach (string line in File.ReadLines(sceneFile))
                {
                    if (line.StartsWith("--- !u!1 ", StringComparison.Ordinal))
                    {
                        count++;
                    }
                }
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                return 0;
            }

            return count;
        }

        private static JsonObject TrustState(McpHost host)
        {
            bool mayRunProjectCode = false;
            string? sessionId = host.CurrentSessionId;
            if (sessionId != null)
            {
                try
                {
                    host.RequireScope(sessionId, Scope.ExecuteProjectCode);
                    mayRunProjectCode = true;
                }
                catch (McpError)
                {
                }
            }

            return new JsonObject
            {
                ["mayRunProjectCode"] = mayRunProjectCode,
            };
        }

        public static void RegisterAll(McpHost host, PlaySessionManager playSessions)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(playSessions);
            Register(host);
            AuditTools.Register(host);
            PlayTools.Register(host, playSessions);
            DebugTools.Register(host, playSessions);
            CaptureTool.Register(host, playSessions);
            ScenarioTools.Register(host);
            ChangeSetStore changeSets = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, changeSets);
            ProjectTools.Register(host, changeSets);
            AssetTools.Register(host, changeSets);
            SceneTools.Register(host, changeSets);
            ScriptTools.Register(host);
            TestTools.Register(host);
            BuildTools.Register(host);
            OperationTools.Register(host);
            ExecuteMethodTools.Register(host);
            CommandPaletteTools.Register(host);
            host.TryRegisterNativeLibrary();
        }

        public static McpHost CreateHostWithDefaults()
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(null),
                new AuditLog());
            Register(host);
            return host;
        }

        public static McpHost CreateHostWithDefaults(IClock clock)
        {
            McpHost host = new McpHost(clock);
            Register(host);
            return host;
        }

        public static EditorRuntime CreateStartupRuntime(string[]? args = null)
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(args),
                CreateStartupAuditLog(args));
            PlaySessionManager playSessions = new PlaySessionManager();
            RegisterAll(host, playSessions);
            GrantCliTestExecution(host, args);
            return new EditorRuntime(host, playSessions);
        }

        public static McpHost CreateStartupHost(string[]? args = null)
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(args),
                CreateStartupAuditLog(args));
            PlaySessionManager playSessions = new PlaySessionManager();
            host.OwnResource(playSessions);
            RegisterAll(host, playSessions);
            GrantCliTestExecution(host, args);
            return host;
        }

        private static void GrantCliTestExecution(McpHost host, string[]? args)
        {
            if (args == null || !HeadlessCli.IsCliInvocation(args))
            {
                return;
            }

            CliArgs parsed = CliArgs.Parse(args);
            if (!parsed.RequestsTestExecution)
            {
                return;
            }

            host.GrantStanding(HeadlessCli.CliAgentName, Scope.ExecuteProjectCode, GrantAuthority.Person);
        }

        private static AuditLog CreateStartupAuditLog(string[]? args)
        {
            string projectPath = ResolveStartupProjectPath(args);
            try
            {
                return new AuditLog(AuditStore.ForProject(projectPath));
            }
            catch (Exception error) when (error is McpError
                || error is IOException
                || error is UnauthorizedAccessException
                || error is ArgumentException)
            {
                string reason = "audit store unavailable for project " + projectPath + ": " + error.Message;
                Console.Error.WriteLine("[unitas] " + reason);
                return new AuditLog(null, reason);
            }
        }

        private static string ResolveStartupProjectPath(string[]? args)
        {
            string? requested = null;
            if (args != null)
            {
                for (int index = 0; index < args.Length - 1; index++)
                {
                    if (string.Equals(args[index], CliArgs.ProjectPathOption, StringComparison.Ordinal))
                    {
                        requested = args[index + 1];
                        break;
                    }
                }
            }

            if (string.IsNullOrWhiteSpace(requested))
            {
                return Directory.GetCurrentDirectory();
            }

            return Path.GetFullPath(requested);
        }
    }
}
