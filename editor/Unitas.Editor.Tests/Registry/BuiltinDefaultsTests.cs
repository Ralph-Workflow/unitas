using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class BuiltinDefaultsTests
    {
        public static void Run()
        {
            RegistersProjectStatusToolAndKnowledgeScopesResource();
            BuiltinDefaultsAreReadProjectScopedAndUntrusted();
            ProjectStatusSkipsGeneratedFolders();
            RegisteringBuiltinDefaultsTwiceFailsWithDuplicateTool();
            StartupHostUsesPersistentAuditStoreOutsideProject();
            StartupHostAuditRecordsSurviveHostRestart();
            StartupHostRefusesStateDirectoryInsideProject();
        }

        private static void RegistersProjectStatusToolAndKnowledgeScopesResource()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);

            IReadOnlyList<ToolDefinition> tools = host.ListTools();
            Check.Equal("builtin: one default tool registered", 1, tools.Count);
            Check.Equal("builtin: default tool name is project.status", "project.status", tools[0].Name);
            Check.Equal(
                "builtin: default tool description",
                "Read current unitas project status summary",
                tools[0].Description);
            Check.That(
                "builtin: default tool requires ReadProject",
                tools[0].RequiredScope == Scope.ReadProject);

            IReadOnlyList<ResourceDefinition> resources = host.ListResources();
            Check.Equal("builtin: three default knowledge resources registered", 3, resources.Count);
            Check.Equal(
                "builtin: resource 0 uri is unitas://knowledge/docs",
                "unitas://knowledge/docs",
                resources[0].Uri);
            Check.Equal(
                "builtin: resource 1 uri is unitas://knowledge/grants",
                "unitas://knowledge/grants",
                resources[1].Uri);
            Check.Equal(
                "builtin: resource 2 uri is unitas://knowledge/scopes",
                "unitas://knowledge/scopes",
                resources[2].Uri);
            Check.Equal(
                "builtin: scopes resource name",
                "AUT-007 Permission Scopes",
                resources[2].Name);
            Check.Equal(
                "builtin: scopes resource mimeType",
                "application/json",
                resources[2].MimeType);
            Check.That(
                "builtin: scopes resource requires ReadProject",
                resources[2].RequiredScope == Scope.ReadProject);
        }

        private static void BuiltinDefaultsAreReadProjectScopedAndUntrusted()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));

            JsonNode statusEnvelope = host.CallTool(session, "project.status", new JsonObject());
            Check.That(
                "builtin: project.status returns untrusted envelope",
                UntrustedContent.IsEnvelope(statusEnvelope));
            Check.Equal(
                "builtin: project.status origin",
                "unitas://project/status",
                statusEnvelope["origin"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "builtin: project.status state is ready",
                "ready",
                statusEnvelope["content"]?["state"]?.GetValue<string>() ?? string.Empty);

            JsonNode scopesEnvelope = host.ReadResource(session, "unitas://knowledge/scopes");
            Check.That(
                "builtin: knowledge/scopes returns untrusted envelope",
                UntrustedContent.IsEnvelope(scopesEnvelope));
            Check.Equal(
                "builtin: knowledge/scopes origin",
                "unitas://knowledge/scopes",
                scopesEnvelope["origin"]?.GetValue<string>() ?? string.Empty);
            JsonArray? scopeArray = scopesEnvelope["content"]?["scopes"] as JsonArray;
            Check.Equal(
                "builtin: knowledge/scopes lists all 11 fixed scopes",
                Scopes.All.Length,
                scopeArray?.Count ?? 0);
            Check.Equal(
                "builtin: knowledge/scopes first scope is read-project",
                "read-project",
                scopeArray?[0]?.GetValue<string>() ?? string.Empty);
        }

        // The scene walk must not descend into generated folders: a project
        // root that is a whole repository would otherwise report every scene
        // in the build output.
        private static void ProjectStatusSkipsGeneratedFolders()
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-builtin-scenes-" + Guid.NewGuid().ToString("N"));
            try
            {
                string scenes = Path.Combine(root, "Scenes");
                string generated = Path.Combine(root, "target", "Build");
                Directory.CreateDirectory(scenes);
                Directory.CreateDirectory(generated);
                File.WriteAllText(
                    Path.Combine(scenes, "Level.unity"),
                    "--- !u!1 &1\nGameObject:\n--- !u!1 &2\nGameObject:\n");
                File.WriteAllText(Path.Combine(generated, "Copy.unity"), "--- !u!1 &1\nGameObject:\n");

                using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
                BuiltinDefaults.Register(host);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));

                JsonNode envelope = host.CallTool(
                    session,
                    "project.status",
                    new JsonObject
                    {
                        ["projectPath"] = root,
                    });
                JsonArray? reported = envelope["content"]?["scenes"] as JsonArray;
                List<string> paths = new List<string>();
                foreach (JsonNode? node in reported ?? new JsonArray())
                {
                    paths.Add(node?["path"]?.GetValue<string>() ?? string.Empty);
                }

                Check.Equal("builtin: project.status reports the project scene", 1, paths.Count);
                Check.Equal("builtin: project.status names the project scene", "Scenes/Level.unity", paths[0]);
            }
            finally
            {
                try
                {
                    if (Directory.Exists(root))
                    {
                        Directory.Delete(root, true);
                    }
                }
                catch (IOException)
                {
                }
                catch (UnauthorizedAccessException)
                {
                }
            }
        }

        private static void RegisteringBuiltinDefaultsTwiceFailsWithDuplicateTool()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            bool threwDuplicate = false;
            try
            {
                BuiltinDefaults.Register(host);
            }
            catch (McpError.DuplicateTool error)
            {
                threwDuplicate = error.Name == "project.status";
            }

            Check.That("builtin: second registration throws DuplicateTool", threwDuplicate);
        }

        private static void StartupHostUsesPersistentAuditStoreOutsideProject()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas-startup-store-" + Guid.NewGuid().ToString("N"));
            string projectDir = Path.Combine(tempRoot, "project");
            string stateDir = Path.Combine(tempRoot, "state");
            Directory.CreateDirectory(projectDir);
            Directory.CreateDirectory(stateDir);
            string? previousStateDir = Environment.GetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable);
            try
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, stateDir);
                using McpHost host = BuiltinDefaults.CreateStartupHost(new[] { "-projectPath", projectDir });
                AuditStore? store = host.Audit.Store;
                Check.That(
                    "builtin_startup: the production startup host audits into a persistent store",
                    store != null);
                Check.That(
                    "builtin_startup: the production startup host reports no store failure",
                    host.Audit.StoreFailure == null);
                if (store == null)
                {
                    return;
                }

                Check.That(
                    "builtin_startup: the audit file lives outside the project directory",
                    !AuditStore.IsPathInsideDirectory(store.AuditFilePath, projectDir));
                Check.That(
                    "builtin_startup: the audit file lives under the unitas state directory",
                    store.AuditFilePath.StartsWith(
                        Path.GetFullPath(stateDir) + Path.DirectorySeparatorChar,
                        StringComparison.Ordinal));
                Check.Equal(
                    "builtin_startup: the store guards the resolved project path",
                    Path.GetFullPath(projectDir),
                    store.ProtectedProjectPath ?? string.Empty);
                Check.Equal(
                    "builtin_startup: the startup host wrote nothing into the project directory",
                    0,
                    Directory.GetFileSystemEntries(projectDir).Length);
            }
            finally
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, previousStateDir);
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void StartupHostAuditRecordsSurviveHostRestart()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas-startup-persist-" + Guid.NewGuid().ToString("N"));
            string projectDir = Path.Combine(tempRoot, "project");
            string stateDir = Path.Combine(tempRoot, "state");
            Directory.CreateDirectory(projectDir);
            Directory.CreateDirectory(stateDir);
            string? previousStateDir = Environment.GetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable);
            string? previousCredential = Environment.GetEnvironmentVariable(PersonCredential.EnvironmentVariableName);
            try
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, stateDir);
                Environment.SetEnvironmentVariable(PersonCredential.EnvironmentVariableName, "startup-person-token");
                using (McpHost firstHost = BuiltinDefaults.CreateStartupHost(new[] { "-projectPath", projectDir }))
                {
                    string firstSession = firstHost.CreateSession(new SessionIdentity("agent-a", "http", "person-1"));
                    firstHost.CallTool(firstSession, "project.status", new JsonObject());
                    Check.Equal(
                        "builtin_startup_persist: the first host audits the tool call",
                        1,
                        firstHost.Audit.Count);
                    Check.Equal(
                        "builtin_startup_persist: the first host audits project.status",
                        "project.status",
                        firstHost.Audit.Records[0].Target);
                }

                Check.Equal(
                    "builtin_startup_persist: no audit file was written into the project directory",
                    0,
                    Directory.GetFileSystemEntries(projectDir).Length);

                using (McpHost secondHost = BuiltinDefaults.CreateStartupHost(new[] { "-projectPath", projectDir }))
                {
                    Check.Equal(
                        "builtin_startup_persist: the second host reloads the first host's record",
                        1,
                        secondHost.Audit.Count);
                    Check.Equal(
                        "builtin_startup_persist: the second host sees the persisted target",
                        "project.status",
                        secondHost.Audit.Records[0].Target);

                    string secondSession = secondHost.CreateSession(new SessionIdentity("agent-a", "http", "person-1"));
                    JsonNode exportEnvelope = secondHost.CallTool(
                        secondSession,
                        "audit.export",
                        new JsonObject
                        {
                            ["personCredential"] = "startup-person-token",
                        });
                    JsonArray? exported = exportEnvelope["content"]?["records"] as JsonArray;
                    Check.Equal(
                        "builtin_startup_persist: audit.export exports the persisted record",
                        1,
                        exported?.Count ?? 0);
                }
            }
            finally
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, previousStateDir);
                Environment.SetEnvironmentVariable(PersonCredential.EnvironmentVariableName, previousCredential);
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void StartupHostRefusesStateDirectoryInsideProject()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas-startup-inside-" + Guid.NewGuid().ToString("N"));
            string projectDir = Path.Combine(tempRoot, "project");
            string stateDir = Path.Combine(projectDir, ".unitas-state");
            Directory.CreateDirectory(projectDir);
            string? previousStateDir = Environment.GetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable);
            try
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, stateDir);
                using McpHost host = BuiltinDefaults.CreateStartupHost(new[] { "-projectPath", projectDir });
                Check.That(
                    "builtin_startup_guard: no audit store is created inside the project directory",
                    host.Audit.Store == null);
                Check.That(
                    "builtin_startup_guard: the store failure is reported",
                    host.Audit.StoreFailure != null);
                Check.That(
                    "builtin_startup_guard: the store failure names the AUT-008 guard",
                    (host.Audit.StoreFailure ?? string.Empty).Contains("AUT-008", StringComparison.Ordinal));
                string session = host.CreateSession(new SessionIdentity("agent-a", "http", "person-1"));
                JsonNode statusEnvelope = host.CallTool(session, "project.status", new JsonObject());
                Check.That(
                    "builtin_startup_guard: the editor still serves tools after a store failure",
                    UntrustedContent.IsEnvelope(statusEnvelope));
                Check.Equal(
                    "builtin_startup_guard: no state directory was created inside the project",
                    0,
                    Directory.GetFileSystemEntries(projectDir).Length);
            }
            finally
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, previousStateDir);
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }
    }
}
