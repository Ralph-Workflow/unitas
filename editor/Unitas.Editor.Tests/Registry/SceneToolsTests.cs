using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class SceneToolsTests
    {
        private const string SampleScene = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";
        private const string ScenePath = "Assets/Scenes/RollABall.unity";
        private const long PickupFileId = 1000000091;
        private const long ArenaFileId = 1000000021;

        public static void Run()
        {
            RegistersTheSceneToolsWithLeastPrivilegeScopes();
            TheLiveHostRegistersTheSceneTools();
            MutatingCallWithoutTheEditScopeIsRefused();
            SetPropertyUndoAndRedoRoundTrip();
            MutatingCallRecordsAChangeSet();
            CreateAndDestroyObjectRoundTrip();
            UndoWithAnEmptyHistoryReturnsAStructuredError();
        }

        private static void RegistersTheSceneToolsWithLeastPrivilegeScopes()
        {
            using McpHost host = NewHost();
            ChangeSetStore store = new ChangeSetStore(host);
            SceneTools.Register(host, store);

            Check.Equal("scene: eight scene tools are registered", 8, host.ListTools().Count);
            Check.That("scene: scene.open requires read-project", Scope.ReadProject == ScopeOf(host, "scene.open"));
            Check.That("scene: scene.hierarchy requires read-project", Scope.ReadProject == ScopeOf(host, "scene.hierarchy"));
            Check.That("scene: scene.set_property requires edit-content", Scope.EditContent == ScopeOf(host, "scene.set_property"));
            Check.That("scene: scene.create_object requires edit-content", Scope.EditContent == ScopeOf(host, "scene.create_object"));
            Check.That("scene: scene.destroy_object requires edit-content", Scope.EditContent == ScopeOf(host, "scene.destroy_object"));
            Check.That("scene: scene.save requires edit-content", Scope.EditContent == ScopeOf(host, "scene.save"));
            Check.That("scene: editor.undo requires edit-content", Scope.EditContent == ScopeOf(host, "editor.undo"));
            Check.That("scene: editor.redo requires edit-content", Scope.EditContent == ScopeOf(host, "editor.redo"));

            Check.Equal("scene: the hierarchy resource is registered", 1, host.ListResources().Count);
            Check.Equal(
                "scene: the hierarchy resource uri",
                "unitas://scene/hierarchy",
                host.ListResources()[0].Uri);
            Check.That(
                "scene: the hierarchy resource requires read-project",
                Scope.ReadProject == host.ListResources()[0].RequiredScope);
        }

        private static void TheLiveHostRegistersTheSceneTools()
        {
            using McpHost host = NewHost();
            using PlaySessionManager playSessions = new PlaySessionManager();
            BuiltinDefaults.RegisterAll(host, playSessions);

            foreach (string name in new[]
            {
                "scene.open",
                "scene.hierarchy",
                "scene.set_property",
                "scene.create_object",
                "scene.destroy_object",
                "scene.save",
                "editor.undo",
                "editor.redo",
            })
            {
                Check.That("scene: the live host registers " + name, FindTool(host, name) != null);
            }

            bool hierarchyResource = false;
            foreach (ResourceDefinition resource in host.ListResources())
            {
                if (string.Equals(resource.Uri, SceneTools.HierarchyOrigin, StringComparison.Ordinal))
                {
                    hierarchyResource = true;
                }
            }

            Check.That("scene: the live host registers the hierarchy resource", hierarchyResource);
        }

        private static void MutatingCallWithoutTheEditScopeIsRefused()
        {
            using McpHost host = NewHost();
            ChangeSetStore store = new ChangeSetStore(host);
            SceneTools.Register(host, store);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);

            McpError? refused = null;
            try
            {
                host.CallTool(
                    session,
                    "scene.set_property",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                        ["path"] = "m_Name",
                        ["value"] = "Refused",
                    });
            }
            catch (McpError failure)
            {
                refused = failure;
            }

            Check.That(
                "scene: a scene edit without edit-content is refused",
                refused is McpError.MissingScope);
            Check.That(
                "scene: the refusal names the edit-content scope",
                refused != null && refused.Message.Contains("edit-content", StringComparison.Ordinal));
        }

        private static void SetPropertyUndoAndRedoRoundTrip()
        {
            string project = CreateProjectWithSampleScene("roundtrip");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);

                JsonNode opened = host.CallTool(
                    session,
                    "scene.open",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = ScenePath,
                    });
                Check.Equal(
                    "scene: the open reports the scene path",
                    ScenePath,
                    opened["content"]?["path"]?.GetValue<string>() ?? string.Empty);

                JsonNode edit = host.CallTool(
                    session,
                    "scene.set_property",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                        ["path"] = "m_Name",
                        ["value"] = "Pickup_1 Renamed",
                    });
                Check.Equal(
                    "scene: the edit reports the value before the edit",
                    "Pickup_1",
                    edit["content"]?["before"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "scene: the edit reports the value after the edit",
                    "Pickup_1 Renamed",
                    edit["content"]?["after"]?.GetValue<string>() ?? string.Empty);

                JsonNode hierarchy = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the hierarchy reports the new value",
                    "Pickup_1 Renamed",
                    NameOf(hierarchy, PickupFileId));

                JsonNode undone = host.CallTool(session, "editor.undo", new JsonObject());
                Check.That(
                    "scene: the undo reports success",
                    undone["content"]?["ok"]?.GetValue<bool>() ?? false);
                JsonNode afterUndo = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the undo restores the previous value",
                    "Pickup_1",
                    NameOf(afterUndo, PickupFileId));

                JsonNode redone = host.CallTool(session, "editor.redo", new JsonObject());
                Check.That(
                    "scene: the redo reports success",
                    redone["content"]?["ok"]?.GetValue<bool>() ?? false);
                JsonNode afterRedo = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the redo reapplies the value",
                    "Pickup_1 Renamed",
                    NameOf(afterRedo, PickupFileId));

                host.CallTool(session, "scene.save", new JsonObject());
                string saved = File.ReadAllText(Path.Combine(project, "Assets", "Scenes", "RollABall.unity"));
                Check.That(
                    "scene: the saved scene holds the new value",
                    saved.Contains("m_Name: Pickup_1 Renamed", StringComparison.Ordinal));

                string sample = File.ReadAllText(ResolveRepoPath(SampleScene));
                Check.That(
                    "scene: the sample scene is untouched",
                    !sample.Contains("Pickup_1 Renamed", StringComparison.Ordinal));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void MutatingCallRecordsAChangeSet()
        {
            string project = CreateProjectWithSampleScene("changeset");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string session = NewEditingSession(host);

                host.CallTool(
                    session,
                    "scene.open",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = ScenePath,
                    });
                JsonNode edit = host.CallTool(
                    session,
                    "scene.set_property",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                        ["path"] = "m_Name",
                        ["value"] = "Pickup_1 Recorded",
                    });
                string changeId = edit["content"]?["changeId"]?.GetValue<string>() ?? string.Empty;
                Check.That("scene: the edit records a change id", changeId.Length > 0);
                Check.Equal(
                    "scene: the change set names the scene edit",
                    "scene.set_property",
                    store.All[0].Operations[0]?["tool"]?.GetValue<string>() ?? string.Empty);

                JsonNode list = host.CallTool(session, "changes.list", new JsonObject());
                JsonArray? changeSets = list["content"]?["changeSets"] as JsonArray;
                Check.Equal("scene: changes.list finds the scene edit", 1, changeSets?.Count ?? 0);
                Check.Equal(
                    "scene: changes.list names the change set",
                    changeId,
                    changeSets?[0]?["id"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void CreateAndDestroyObjectRoundTrip()
        {
            string project = CreateProjectWithSampleScene("create");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);

                host.CallTool(
                    session,
                    "scene.open",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = ScenePath,
                    });

                JsonNode created = host.CallTool(
                    session,
                    "scene.create_object",
                    new JsonObject
                    {
                        ["name"] = "U2B Cube",
                        ["parentFileId"] = ArenaFileId,
                    });
                long createdFileId = created["content"]?["fileId"]?.GetValue<long>() ?? 0;
                Check.That("scene: the create reports the new file id", createdFileId != 0);
                JsonNode withCube = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the hierarchy reports the created object",
                    "U2B Cube",
                    NameOf(withCube, createdFileId));
                Check.Equal(
                    "scene: the created object is a child of the arena",
                    "1000000021",
                    ParentOf(withCube, createdFileId));

                host.CallTool(session, "editor.undo", new JsonObject());
                JsonNode withoutCube = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the undo removes the created object",
                    string.Empty,
                    NameOf(withoutCube, createdFileId));

                host.CallTool(
                    session,
                    "scene.destroy_object",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                    });
                JsonNode withoutPickup = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the hierarchy no longer reports the destroyed object",
                    string.Empty,
                    NameOf(withoutPickup, PickupFileId));

                host.CallTool(session, "editor.undo", new JsonObject());
                JsonNode withPickup = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the undo restores the destroyed object",
                    "Pickup_1",
                    NameOf(withPickup, PickupFileId));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void UndoWithAnEmptyHistoryReturnsAStructuredError()
        {
            string project = CreateProjectWithSampleScene("emptyundo");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);

                host.CallTool(
                    session,
                    "scene.open",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = ScenePath,
                    });

                JsonNode? undone = null;
                McpError? thrown = null;
                try
                {
                    undone = host.CallTool(session, "editor.undo", new JsonObject());
                }
                catch (McpError failure)
                {
                    thrown = failure;
                }

                Check.That(
                    "scene: an empty undo is a structured error, not a throw",
                    thrown == null && undone != null);
                Check.Equal(
                    "scene: an empty undo reports failure",
                    false,
                    undone?["content"]?["ok"]?.GetValue<bool>() ?? true);
                Check.Equal(
                    "scene: an empty undo names the problem",
                    "nothing to undo",
                    undone?["content"]?["error"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static string NewEditingSession(McpHost host)
        {
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);
            host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);
            return session;
        }

        private static string NameOf(JsonNode envelope, long fileId)
        {
            JsonArray? objects = envelope["content"]?["objects"] as JsonArray;
            if (objects != null)
            {
                foreach (JsonNode? entry in objects)
                {
                    if (entry?["fileId"]?.GetValue<long>() == fileId)
                    {
                        return entry?["name"]?.GetValue<string>() ?? string.Empty;
                    }
                }
            }

            return string.Empty;
        }

        private static string ParentOf(JsonNode envelope, long fileId)
        {
            JsonArray? objects = envelope["content"]?["objects"] as JsonArray;
            if (objects != null)
            {
                foreach (JsonNode? entry in objects)
                {
                    if (entry?["fileId"]?.GetValue<long>() == fileId)
                    {
                        return entry?["parentFileId"]?.GetValue<long>().ToString(System.Globalization.CultureInfo.InvariantCulture) ?? string.Empty;
                    }
                }
            }

            return string.Empty;
        }

        private static McpHost NewHost()
        {
            return new McpHost(new FixedClock(1_700_000_000_000L));
        }

        private static ToolDefinition? FindTool(McpHost host, string name)
        {
            foreach (ToolDefinition candidate in host.ListTools())
            {
                if (string.Equals(candidate.Name, name, StringComparison.Ordinal))
                {
                    return candidate;
                }
            }

            return null;
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

        private static string CreateProjectWithSampleScene(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u2b-" + label + "-" + Guid.NewGuid().ToString("N"));
            string scenes = Path.Combine(root, "Assets", "Scenes");
            Directory.CreateDirectory(scenes);
            File.Copy(ResolveRepoPath(SampleScene), Path.Combine(scenes, "RollABall.unity"));
            return root;
        }

        private static void DeleteDirectory(string directory)
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, true);
            }
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

        private static string ResolveRepoPath(string relativePath)
        {
            return Path.Combine(ResolveRepoRoot(), relativePath);
        }
    }
}
