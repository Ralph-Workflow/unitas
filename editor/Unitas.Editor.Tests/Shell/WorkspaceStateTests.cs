using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;
using Unitas.Editor.Shell;

namespace Unitas.Editor.Tests
{
    public static class WorkspaceStateTests
    {
        private const string SampleSceneRelative = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";
        private const string ProjectSceneRelative = "Assets/Scenes/RollABall.unity";
        private const long PickupFileId = 1000000091L;
        private const long ArenaFileId = 1000000021L;

        public static void Run()
        {
            InitializeProjectPopulatesHierarchyAndInspectsFirstObject();
            SelectByIndexByIdAndNextUpdateInspectorComponentsAndProperties();
            ShellHostDispatchesEventsAndUpdatesWorkspaceDto();
            InspectPayloadIncludesPropertyPathAndWorkspaceRendersRows();
            ScriptedSelectStepPropertyAndUndoRestoreValue();
            TogglePropertyAndToggleActiveFlipBoolAndActiveState();
            CreateDuplicateAndDestroyUpdateHierarchyAndKeepSelectionValid();
            OffscreenShellRunRendersWorkspaceCanvasAndCapturesBmpWhenAbiPresent();
        }

        private static void InitializeProjectPopulatesHierarchyAndInspectsFirstObject()
        {
            string project = CreateSampleProjectCopy("init");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                string session = NewShellSession(host);
                WorkspaceState state = new WorkspaceState();

                state.InitializeProject(host, session, project);

                Check.Equal("workspace-init: active scene path is RollABall.unity", ProjectSceneRelative, state.ActiveScenePath);
                Check.That("workspace-init: hierarchy entries populated from scene.hierarchy", state.HierarchyEntries.Count > 5);
                Check.That(
                    "workspace-init: first object is selected by default",
                    !string.IsNullOrWhiteSpace(state.SelectedObjectId) && !string.IsNullOrWhiteSpace(state.SelectedObjectName));
                Check.That("workspace-init: title text names active scene", state.TitleText.Contains("RollABall.unity", StringComparison.Ordinal));
                Check.That(
                    "workspace-init: hierarchy rows capped at 12 and first row has selected marker",
                    state.HierarchyRows.Count <= WorkspaceState.MaxHierarchyRows
                        && state.HierarchyRows.Count > 0
                        && state.HierarchyRows[0].StartsWith("> " + state.SelectedObjectName, StringComparison.Ordinal));
                Check.That("workspace-init: inspector components populated for selected object", state.InspectorComponents.Count >= 1);
                Check.That(
                    "workspace-init: property rows capped at 8 and include Transform header and properties",
                    state.PropertyRows.Count <= WorkspaceState.MaxPropertyRows
                        && state.PropertyRows.Count > 1
                        && state.PropertyRows[0] == "[Transform]");
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void SelectByIndexByIdAndNextUpdateInspectorComponentsAndProperties()
        {
            string project = CreateSampleProjectCopy("select");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                string session = NewShellSession(host);
                WorkspaceState state = new WorkspaceState();
                state.InitializeProject(host, session, project);

                state.SelectById(host, session, "1000000091");
                Check.Equal("workspace-select: SelectById selects Pickup_1 id", "1000000091", state.SelectedObjectId);
                Check.Equal("workspace-select: SelectById updates SelectedObjectName", "Pickup_1", state.SelectedObjectName);
                Check.Equal("workspace-select: Pickup_1 has 5 components in inspector", 5, state.InspectorComponents.Count);
                Check.That(
                    "workspace-select: InspectorSummary reports active state and position",
                    state.InspectorSummary.Contains("Pickup_1 [Active]", StringComparison.Ordinal)
                        && state.InspectorSummary.Contains("Pos={x: 3, y: 0.75, z: 3}", StringComparison.Ordinal));
                Check.That("workspace-select: Transform component has properties populated", state.InspectorComponents[0].Properties.Count > 0);

                state.SelectByIndex(host, session, 0);
                string firstId = state.SelectedObjectId;
                string secondExpectedId = state.HierarchyEntries[1].FileId.ToString(System.Globalization.CultureInfo.InvariantCulture);

                state.SelectNext(host, session);
                Check.Equal("workspace-select: SelectNext advances from index 0 to index 1", secondExpectedId, state.SelectedObjectId);

                state.SelectByIndex(host, session, state.HierarchyEntries.Count - 1);
                state.SelectNext(host, session);
                Check.Equal("workspace-select: SelectNext wraps around to index 0", firstId, state.SelectedObjectId);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void ShellHostDispatchesEventsAndUpdatesWorkspaceDto()
        {
            string project = CreateSampleProjectCopy("events");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                ShellHost shell = new ShellHost(host);

                string initJson = shell.HandleNativeEvent(ShellEvent.Init(project).ToJsonString());
                JsonObject? initDto = JsonNode.Parse(initJson) as JsonObject;
                Check.That("shell-host: init event returns JSON DTO", initDto != null);
                Check.Equal(
                    "shell-host: init DTO activeScenePath",
                    ProjectSceneRelative,
                    initDto?["activeScenePath"]?.GetValue<string>() ?? string.Empty);

                string selectJson = shell.HandleNativeEvent(ShellEvent.SelectById("1000000021").ToJsonString());
                JsonObject? selectDto = JsonNode.Parse(selectJson) as JsonObject;
                Check.Equal(
                    "shell-host: select by id updates selectedObjectId",
                    "1000000021",
                    selectDto?["selectedObjectId"]?.GetValue<string>() ?? string.Empty);

                int countBefore = shell.State.HierarchyEntries.Count;
                shell.ApplyEvent(ShellEvent.InvokeCommand(
                    "scene.create_object",
                    new JsonObject { ["name"] = "ShellCreatedObject", ["parentFileId"] = ArenaFileId }));
                Check.Equal(
                    "shell-host: command event invokes tool and refreshes hierarchy count",
                    countBefore + 1,
                    shell.State.HierarchyEntries.Count);

                shell.ApplyEvent(ShellEvent.InvokeCommand(
                    "scene.set_property",
                    new JsonObject { ["fileId"] = PickupFileId, ["path"] = "m_Name", ["value"] = "Pickup_Renamed_By_Shell" }));
                shell.ApplyEvent(ShellEvent.SelectById("1000000091"));
                Check.Equal(
                    "shell-host: property mutation reflected in workspace selection name",
                    "Pickup_Renamed_By_Shell",
                    shell.State.SelectedObjectName);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void InspectPayloadIncludesPropertyPathAndWorkspaceRendersRows()
        {
            string project = CreateSampleProjectCopy("inspect-rows");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                string session = NewShellSession(host);
                WorkspaceState state = new WorkspaceState();
                state.InitializeProject(host, session, project);

                JsonNode inspected = host.CallTool(
                    session,
                    "scene.inspect_object",
                    new JsonObject { ["fileId"] = PickupFileId });
                JsonArray? components = inspected["content"]?["components"] as JsonArray;
                JsonObject? firstProp = (components?[0]?["properties"] as JsonArray)?[0] as JsonObject;
                Check.That(
                    "inspect-path: scene.inspect_object property entry carries non-empty path equal to name",
                    firstProp != null
                        && !string.IsNullOrWhiteSpace(firstProp["path"]?.GetValue<string>())
                        && string.Equals(
                            firstProp["name"]?.GetValue<string>(),
                            firstProp["path"]?.GetValue<string>(),
                            StringComparison.Ordinal));

                state.SelectByIndex(host, session, 2);
                Check.That(
                    "hierarchy-rows: select index 2 marks row 2 with >",
                    state.HierarchyRows.Count > 2 && state.HierarchyRows[2].StartsWith("> ", StringComparison.Ordinal));
                Check.Equal(
                    "hierarchy-rows: truncated list ends with ellipsis row at index 11",
                    "...",
                    state.HierarchyRows[WorkspaceState.MaxHierarchyRows - 1]);

                state.SelectById(host, session, "1000000001");
                Check.That(
                    "property-rows: includes m_LocalPosition value for selected object",
                    FindPropertyRowIndex(state, "m_LocalPosition = ") >= 0);

                JsonObject dto = state.ToJsonObject();
                Check.That(
                    "workspace-dto: carries hierarchyRows and propertyRows arrays instead of monolithic texts",
                    dto["hierarchyRows"] is JsonArray
                        && dto["propertyRows"] is JsonArray
                        && dto["hierarchyText"] == null
                        && dto["inspectorText"] == null);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void ScriptedSelectStepPropertyAndUndoRestoreValue()
        {
            string project = CreateSampleProjectCopy("step-undo");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                ShellHost shell = new ShellHost(host);

                shell.ApplyEvent(ShellEvent.Init(project));
                shell.ApplyEvent(ShellEvent.SelectByIndex(1));

                int typeRowIndex = FindPropertyRowIndex(shell.State, "m_Type = 1");
                Check.That("step-property: Directional Light exposes m_Type = 1 in propertyRows", typeRowIndex >= 0);

                shell.ApplyEvent(ShellEvent.SelectProperty(typeRowIndex));
                Check.That(
                    "step-property: select_property marks row with >",
                    shell.State.PropertyRows[typeRowIndex].StartsWith("> m_Type = 1", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.StepProperty(1));
                Check.That(
                    "step-property: stepping integral property +1 increases value by 1 to 2",
                    shell.State.PropertyRows[typeRowIndex].StartsWith("> m_Type = 2", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.InvokeCommand("editor.undo"));
                Check.That(
                    "step-property: editor.undo restores m_Type = 1",
                    shell.State.PropertyRows[typeRowIndex].StartsWith("> m_Type = 1", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.SelectById("1000000201"));
                int durationRowIndex = FindPropertyRowIndex(shell.State, "m_Duration = 0.6");
                Check.That("step-property: CollectBurst exposes fractional m_Duration = 0.6 in propertyRows", durationRowIndex >= 0);
                shell.ApplyEvent(ShellEvent.SelectProperty(durationRowIndex));
                shell.ApplyEvent(ShellEvent.StepProperty(1));
                Check.That(
                    "step-property: stepping fractional property +1 increases value by 0.1 to 0.7",
                    shell.State.PropertyRows[durationRowIndex].StartsWith("> m_Duration = 0.7", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.InvokeCommand("editor.undo"));
                Check.That(
                    "step-property: editor.undo restores m_Duration = 0.6",
                    shell.State.PropertyRows[durationRowIndex].StartsWith("> m_Duration = 0.6", StringComparison.Ordinal));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void TogglePropertyAndToggleActiveFlipBoolAndActiveState()
        {
            string project = CreateSampleProjectCopy("toggle");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                ShellHost shell = new ShellHost(host);

                shell.ApplyEvent(ShellEvent.Init(project));
                shell.ApplyEvent(ShellEvent.SelectByIndex(1));

                int enabledRowIndex = FindPropertyRowIndex(shell.State, "m_Enabled = 1");
                Check.That("toggle-property: Directional Light exposes m_Enabled = 1 in propertyRows", enabledRowIndex >= 0);

                shell.ApplyEvent(ShellEvent.SelectProperty(enabledRowIndex));
                shell.ApplyEvent(ShellEvent.ToggleProperty());
                Check.That(
                    "toggle-property: toggle_property flips m_Enabled from 1 to 0",
                    shell.State.PropertyRows[enabledRowIndex].StartsWith("> m_Enabled = 0", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.ToggleProperty());
                Check.That(
                    "toggle-property: second toggle_property flips m_Enabled back to 1",
                    shell.State.PropertyRows[enabledRowIndex].StartsWith("> m_Enabled = 1", StringComparison.Ordinal));

                Check.Equal("toggle-active: initially active", true, shell.State.SelectedObjectActive);
                Check.That(
                    "toggle-active: InspectorSummary initially contains [Active]",
                    shell.State.InspectorSummary.Contains("[Active]", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.ToggleActive());
                Check.Equal("toggle-active: toggle_active flips SelectedObjectActive to false", false, shell.State.SelectedObjectActive);
                Check.That(
                    "toggle-active: InspectorSummary flips to [Inactive]",
                    shell.State.InspectorSummary.Contains("[Inactive]", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.InvokeCommand("editor.undo"));
                Check.Equal("toggle-active: undo restores SelectedObjectActive to true", true, shell.State.SelectedObjectActive);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void CreateDuplicateAndDestroyUpdateHierarchyAndKeepSelectionValid()
        {
            string project = CreateSampleProjectCopy("crud");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                ShellHost shell = new ShellHost(host);

                shell.ApplyEvent(ShellEvent.Init(project));
                int initialCount = shell.State.HierarchyEntries.Count;

                shell.ApplyEvent(ShellEvent.InvokeCommand("scene.create_object"));
                Check.Equal(
                    "shell-crud: create_object without explicit args increments hierarchy count",
                    initialCount + 1,
                    shell.State.HierarchyEntries.Count);
                Check.That("shell-crud: selection stays valid after create_object", !string.IsNullOrWhiteSpace(shell.State.SelectedObjectId));

                shell.ApplyEvent(ShellEvent.SelectById("1000000091"));
                shell.ApplyEvent(ShellEvent.InvokeCommand("scene.duplicate_object"));
                Check.Equal(
                    "shell-crud: duplicate_object without explicit args duplicates selected object",
                    initialCount + 2,
                    shell.State.HierarchyEntries.Count);

                shell.ApplyEvent(ShellEvent.InvokeCommand("scene.destroy_object"));
                Check.Equal(
                    "shell-crud: destroy_object without explicit args deletes selected object",
                    initialCount + 1,
                    shell.State.HierarchyEntries.Count);
                Check.That(
                    "shell-crud: selection falls back to a valid object after destroy_object",
                    !string.IsNullOrWhiteSpace(shell.State.SelectedObjectId)
                        && !string.Equals(shell.State.SelectedObjectId, "1000000091", StringComparison.Ordinal));

                shell.ApplyEvent(ShellEvent.InvokeCommand("editor.undo"));
                Check.Equal("shell-crud: undo restores destroyed object", initialCount + 2, shell.State.HierarchyEntries.Count);

                shell.ApplyEvent(ShellEvent.InvokeCommand("editor.redo"));
                Check.Equal("shell-crud: redo re-applies destroy_object", initialCount + 1, shell.State.HierarchyEntries.Count);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static int FindPropertyRowIndex(WorkspaceState state, string substring)
        {
            for (int i = 0; i < state.PropertyRows.Count; i++)
            {
                if (state.PropertyRows[i].Contains(substring, StringComparison.Ordinal))
                {
                    return i;
                }
            }

            return -1;
        }

        private static void OffscreenShellRunRendersWorkspaceCanvasAndCapturesBmpWhenAbiPresent()
        {
            if (!NativeLibraryLoader.TryOpen(null, out NativeLibraryLoader? loader, out _)
                || loader == null)
            {
                return;
            }

            string project = CreateSampleProjectCopy("offscreen");
            string capturePath = Path.Combine(project, "shell-capture.bmp");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                ShellHost shell = new ShellHost(host);
                ShellRunOptions stepOptions = new ShellRunOptions
                {
                    TargetProjectPath = project,
                    Headless = true,
                    MaxFrames = 5,
                    Width = 320,
                    Height = 180,
                };
                stepOptions.Events.Add(ShellEvent.Init(project));
                stepOptions.Events.Add(ShellEvent.SelectByIndex(1));
                stepOptions.Events.Add(ShellEvent.SelectProperty(6));
                stepOptions.Events.Add(ShellEvent.StepProperty(1));

                JsonObject stepResult;
                try
                {
                    stepResult = shell.Run(stepOptions);
                }
                catch (McpError.InvalidRequest error)
                    when (error.Message.Contains("unitas_editor_run_shell export is not available", StringComparison.Ordinal))
                {
                    return;
                }

                Check.Equal("shell-offscreen: step run reports ok=true", true, stepResult["ok"]?.GetValue<bool>() ?? false);
                Check.Equal(
                    "shell-offscreen: select index 1 marks row 1 with >",
                    true,
                    (stepResult["lastWorkspaceState"]?["hierarchyRows"] as JsonArray)?[1]?.GetValue<string>().StartsWith("> Directional Light", StringComparison.Ordinal) ?? false);
                Check.Equal(
                    "shell-offscreen: step_property +1 steps m_Type to 2 in lastWorkspaceState.propertyRows",
                    "> m_Type = 2",
                    (stepResult["lastWorkspaceState"]?["propertyRows"] as JsonArray)?[6]?.GetValue<string>() ?? string.Empty);

                ShellRunOptions undoOptions = new ShellRunOptions
                {
                    TargetProjectPath = project,
                    Headless = true,
                    MaxFrames = 3,
                    Width = 320,
                    Height = 180,
                    CaptureOut = capturePath,
                };
                undoOptions.Events.Add(ShellEvent.InvokeCommand("editor.undo"));

                JsonObject result = shell.Run(undoOptions);

                Check.Equal("shell-offscreen: native run reports ok=true", true, result["ok"]?.GetValue<bool>() ?? false);
                Check.That("shell-offscreen: renders non-background pixels", (result["nonBackgroundPixels"]?.GetValue<long>() ?? 0L) > 0L);
                Check.Equal(
                    "shell-offscreen: undo restores m_Type to 1 in lastWorkspaceState.propertyRows",
                    "> m_Type = 1",
                    (result["lastWorkspaceState"]?["propertyRows"] as JsonArray)?[6]?.GetValue<string>() ?? string.Empty);
                Check.That(
                    "shell-offscreen: canvasTexts includes rendered hierarchy and property rows",
                    (result["canvasTexts"] as JsonArray) != null
                        && (result["canvasTexts"] as JsonArray)!.Count > 5);
                Check.That(
                    "shell-offscreen: BMP capture file written to disk",
                    File.Exists(capturePath) && new FileInfo(capturePath).Length > 54);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static McpHost NewHostWithSceneTools()
        {
            McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            ChangeSetStore store = new ChangeSetStore(host);
            SceneTools.Register(host, store);
            return host;
        }

        private static string NewShellSession(McpHost host)
        {
            string session = host.CreateSession(
                new SessionIdentity(
                    ShellHost.ShellAgentName,
                    ShellHost.ShellClientName,
                    ShellHost.ShellAuthorizedBy));
            host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);
            host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);
            return session;
        }

        private static string CreateSampleProjectCopy(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-workspace-test-" + label + "-" + Guid.NewGuid().ToString("N"));
            string scenes = Path.Combine(root, "Assets", "Scenes");
            Directory.CreateDirectory(scenes);
            File.Copy(ResolveSampleScenePath(), Path.Combine(scenes, "RollABall.unity"));
            return root;
        }

        private static string ResolveSampleScenePath()
        {
            DirectoryInfo? directory = new DirectoryInfo(Directory.GetCurrentDirectory());
            for (int depth = 0; depth < 12 && directory != null; depth++)
            {
                string candidate = Path.Combine(directory.FullName, SampleSceneRelative);
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }

            throw new DirectoryNotFoundException("could not locate " + SampleSceneRelative);
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
