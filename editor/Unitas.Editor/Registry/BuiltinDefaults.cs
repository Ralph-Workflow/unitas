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
                arguments =>
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
                });

            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://knowledge/scopes",
                    "AUT-007 Permission Scopes",
                    "Fixed permission scope catalog for unitas MCP sessions",
                    "application/json",
                    Scope.ReadProject),
                () =>
                {
                    JsonArray scopes = new JsonArray();
                    foreach (Scope scope in Scopes.All)
                    {
                        scopes.Add(Scopes.AsString(scope));
                    }

                    return ResourceContent.Untrusted(
                        "unitas://knowledge/scopes",
                        new JsonObject
                        {
                            ["scopes"] = scopes,
                        });
                });
        }

        // Unity keeps scenes under Assets; anchoring there also avoids the
        // generated root folders (Library, obj, ...) that project.list filters.
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

        // The generated folders hold build output rather than project scenes,
        // so the walk skips them the way project.list does; without that a
        // project root that is a whole repository walks every build artefact.
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

        // Scene inspection the asset pipeline already trusts: a GameObject is a
        // class-1 YAML document, so counting document headers is the object count.
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

        // Trust state of the open project for the calling session: running project
        // code is the trusted verb, so probe the ExecuteProjectCode grant the same
        // way the headless CLI maps it to refused-untrusted-project.
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

        // The command line is a person-driven local invocation, so a test run
        // from it holds the scope that runs project code; the grant is bound to
        // the CLI agent of that invocation and to no other session.
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
