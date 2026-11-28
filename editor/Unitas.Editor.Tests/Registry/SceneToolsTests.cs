using System;
using System.Globalization;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class SceneToolsTests
    {
        private const string SampleScene = "samples/marble-rush/Assets/Scenes/MarbleRush.unity";
        private const string ScenePath = "Assets/Scenes/MarbleRush.unity";
        private const long PickupFileId = 1000000091;
        private const long PickupTransformFileId = 1000000092;
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
            InspectObjectReportsTheComponentsAndProperties();
            ReparentMovesTheObjectAndRefusesACycle();
            SetActiveTogglesTheObject();
            DuplicateObjectCopiesTheObject();
            SnapshotAndRestoreBringTheEditTimeSceneBack();
        }

        private static void RegistersTheSceneToolsWithLeastPrivilegeScopes()
        {
            using McpHost host = NewHost();
            ChangeSetStore store = new ChangeSetStore(host);
            SceneTools.Register(host, store);

            Check.Equal("scene: fourteen scene tools are registered", 14, host.ListTools().Count);
            Check.That("scene: scene.open requires read-project", Scope.ReadProject == ScopeOf(host, "scene.open"));
            Check.That("scene: scene.hierarchy requires read-project", Scope.ReadProject == ScopeOf(host, "scene.hierarchy"));
            Check.That("scene: scene.set_property requires edit-content", Scope.EditContent == ScopeOf(host, "scene.set_property"));
            Check.That("scene: scene.create_object requires edit-content", Scope.EditContent == ScopeOf(host, "scene.create_object"));
            Check.That("scene: scene.destroy_object requires edit-content", Scope.EditContent == ScopeOf(host, "scene.destroy_object"));
            Check.That("scene: scene.save requires edit-content", Scope.EditContent == ScopeOf(host, "scene.save"));
            Check.That("scene: editor.undo requires edit-content", Scope.EditContent == ScopeOf(host, "editor.undo"));
            Check.That("scene: editor.redo requires edit-content", Scope.EditContent == ScopeOf(host, "editor.redo"));
            Check.That("scene: scene.inspect_object requires read-project", Scope.ReadProject == ScopeOf(host, "scene.inspect_object"));
            Check.That("scene: scene.reparent_object requires edit-content", Scope.EditContent == ScopeOf(host, "scene.reparent_object"));
            Check.That("scene: scene.set_active requires edit-content", Scope.EditContent == ScopeOf(host, "scene.set_active"));
            Check.That("scene: scene.duplicate_object requires edit-content", Scope.EditContent == ScopeOf(host, "scene.duplicate_object"));
            Check.That("scene: scene.snapshot_edit_state requires read-project", Scope.ReadProject == ScopeOf(host, "scene.snapshot_edit_state"));
            Check.That("scene: scene.restore_edit_state requires edit-content", Scope.EditContent == ScopeOf(host, "scene.restore_edit_state"));

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
                "scene.inspect_object",
                "scene.reparent_object",
                "scene.set_active",
                "scene.duplicate_object",
                "scene.snapshot_edit_state",
                "scene.restore_edit_state",
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
                string saved = File.ReadAllText(Path.Combine(project, "Assets", "Scenes", "MarbleRush.unity"));
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

        private static void InspectObjectReportsTheComponentsAndProperties()
        {
            string project = CreateProjectWithSampleScene("inspect");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);
                OpenSampleScene(host, session, project);

                JsonNode inspected = host.CallTool(
                    session,
                    "scene.inspect_object",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                    });
                Check.Equal(
                    "scene: the inspection names the object",
                    "Pickup_1",
                    inspected["content"]?["name"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "scene: the inspection names the parent",
                    "1000000081",
                    inspected["content"]?["parentFileId"]?.GetValue<long>().ToString(CultureInfo.InvariantCulture) ?? string.Empty);
                Check.Equal(
                    "scene: the inspection reports the position",
                    "{x: 3, y: 0.75, z: 3}",
                    inspected["content"]?["position"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "scene: the inspection reports the object as active",
                    true,
                    inspected["content"]?["active"]?.GetValue<bool>() ?? false);

                JsonArray? components = inspected["content"]?["components"] as JsonArray;
                Check.Equal("scene: the inspection lists the components", 5, components?.Count ?? 0);
                Check.Equal(
                    "scene: the first component of the inspection is the transform",
                    "Transform",
                    components?[0]?["type"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void ReparentMovesTheObjectAndRefusesACycle()
        {
            string project = CreateProjectWithSampleScene("reparent");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string session = NewEditingSession(host);
                OpenSampleScene(host, session, project);

                JsonNode moved = host.CallTool(
                    session,
                    "scene.reparent_object",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                        ["parentFileId"] = ArenaFileId,
                    });
                Check.Equal(
                    "scene: the reparent reports the new parent",
                    "1000000021",
                    moved["content"]?["parentFileId"]?.GetValue<long>().ToString(CultureInfo.InvariantCulture) ?? string.Empty);
                JsonNode hierarchy = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the hierarchy reports the new parent",
                    "1000000021",
                    ParentOf(hierarchy, PickupFileId));

                McpError? refused = null;
                try
                {
                    host.CallTool(
                        session,
                        "scene.reparent_object",
                        new JsonObject
                        {
                            ["fileId"] = ArenaFileId,
                            ["parentFileId"] = PickupFileId,
                        });
                }
                catch (McpError failure)
                {
                    refused = failure;
                }

                Check.That("scene: a reparent that would create a cycle is refused", refused != null);
                JsonNode afterRefusal = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the refused reparent leaves the parent alone",
                    "1000000021",
                    ParentOf(afterRefusal, PickupFileId));

                host.CallTool(session, "editor.undo", new JsonObject());
                JsonNode afterUndo = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the undo restores the previous parent",
                    "1000000081",
                    ParentOf(afterUndo, PickupFileId));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void SetActiveTogglesTheObject()
        {
            string project = CreateProjectWithSampleScene("setactive");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);
                OpenSampleScene(host, session, project);

                host.CallTool(
                    session,
                    "scene.set_active",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                        ["active"] = false,
                    });
                JsonNode deactivated = host.CallTool(
                    session,
                    "scene.inspect_object",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                    });
                Check.Equal(
                    "scene: the deactivated object is not active",
                    false,
                    deactivated["content"]?["active"]?.GetValue<bool>() ?? true);

                host.CallTool(session, "editor.undo", new JsonObject());
                JsonNode activated = host.CallTool(
                    session,
                    "scene.inspect_object",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                    });
                Check.Equal(
                    "scene: the undo activates the object again",
                    true,
                    activated["content"]?["active"]?.GetValue<bool>() ?? false);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void DuplicateObjectCopiesTheObject()
        {
            string project = CreateProjectWithSampleScene("duplicate");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);
                OpenSampleScene(host, session, project);

                JsonNode duplicated = host.CallTool(
                    session,
                    "scene.duplicate_object",
                    new JsonObject
                    {
                        ["fileId"] = PickupFileId,
                    });
                long copyFileId = duplicated["content"]?["fileId"]?.GetValue<long>() ?? 0;
                Check.That("scene: the duplicate reports the new file id", copyFileId != 0);
                Check.Equal(
                    "scene: the duplicate names the copy",
                    "Pickup_1 Copy",
                    duplicated["content"]?["name"]?.GetValue<string>() ?? string.Empty);

                JsonNode hierarchy = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the hierarchy reports the copy",
                    "Pickup_1 Copy",
                    NameOf(hierarchy, copyFileId));
                Check.Equal(
                    "scene: the copy keeps the parent of the object it copies",
                    "1000000081",
                    ParentOf(hierarchy, copyFileId));

                host.CallTool(session, "editor.undo", new JsonObject());
                JsonNode afterUndo = host.CallTool(session, "scene.hierarchy", new JsonObject());
                Check.Equal(
                    "scene: the undo removes the copy",
                    string.Empty,
                    NameOf(afterUndo, copyFileId));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void SnapshotAndRestoreBringTheEditTimeSceneBack()
        {
            string project = CreateProjectWithSampleScene("editstate");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                SceneTools.Register(host, store);
                string session = NewEditingSession(host);
                OpenSampleScene(host, session, project);

                host.CallTool(
                    session,
                    "scene.set_property",
                    new JsonObject
                    {
                        ["fileId"] = PickupTransformFileId,
                        ["path"] = "m_LocalPosition.x",
                        ["value"] = "2.25",
                    });

                JsonNode captured = host.CallTool(session, "scene.snapshot_edit_state", new JsonObject());
                Check.Equal(
                    "scene: the snapshot reports the captured state",
                    "captured",
                    captured["content"]?["status"]?.GetValue<string>() ?? string.Empty);

                host.CallTool(
                    session,
                    "scene.set_property",
                    new JsonObject
                    {
                        ["fileId"] = PickupTransformFileId,
                        ["path"] = "m_LocalPosition.x",
                        ["value"] = "99",
                    });
                Check.Equal(
                    "scene: the change after the snapshot is in the scene",
                    "{x: 99, y: 0.75, z: 3}",
                    PositionOf(host, session, PickupFileId));

                JsonNode restored = host.CallTool(session, "scene.restore_edit_state", new JsonObject());
                Check.Equal(
                    "scene: the restore reports the restored state",
                    "restored",
                    restored["content"]?["status"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "scene: the restore brings the edit-time scene back",
                    "{x: 2.25, y: 0.75, z: 3}",
                    PositionOf(host, session, PickupFileId));

                McpError? refused = null;
                try
                {
                    host.CallTool(session, "scene.restore_edit_state", new JsonObject());
                }
                catch (McpError failure)
                {
                    refused = failure;
                }

                Check.That("scene: a restore without a captured state is refused", refused != null);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void OpenSampleScene(McpHost host, string session, string project)
        {
            host.CallTool(
                session,
                "scene.open",
                new JsonObject
                {
                    ["projectPath"] = project,
                    ["path"] = ScenePath,
                });
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
            File.Copy(ResolveRepoPath(SampleScene), Path.Combine(scenes, "MarbleRush.unity"));
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
                if (Directory.Exists(Path.Combine(directory.FullName, "samples", "marble-rush")))
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
