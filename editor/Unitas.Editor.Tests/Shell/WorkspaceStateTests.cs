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

                Check.Equal(
                    "workspace-init: active scene path is RollABall.unity",
                    ProjectSceneRelative,
                    state.ActiveScenePath);
                Check.That(
                    "workspace-init: hierarchy entries populated from scene.hierarchy",
                    state.HierarchyEntries.Count > 5);
                Check.That(
                    "workspace-init: first object is selected by default",
                    !string.IsNullOrWhiteSpace(state.SelectedObjectId)
                        && !string.IsNullOrWhiteSpace(state.SelectedObjectName));
                Check.That(
                    "workspace-init: title text names active scene",
                    state.TitleText.Contains("RollABall.unity", StringComparison.Ordinal));
                Check.That(
                    "workspace-init: hierarchy text includes selected marker",
                    state.HierarchyText.Contains("> " + state.SelectedObjectName, StringComparison.Ordinal));
                Check.That(
                    "workspace-init: inspector components populated for selected object",
                    state.InspectorComponents.Count >= 1);
                Check.That(
                    "workspace-init: inspector text includes component brackets",
                    state.InspectorText.Contains("[Transform]", StringComparison.Ordinal));
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
                Check.Equal(
                    "workspace-select: SelectById selects Pickup_1 id",
                    "1000000091",
                    state.SelectedObjectId);
                Check.Equal(
                    "workspace-select: SelectById updates SelectedObjectName",
                    "Pickup_1",
                    state.SelectedObjectName);
                Check.Equal(
                    "workspace-select: Pickup_1 has 5 components in inspector",
                    5,
                    state.InspectorComponents.Count);
                Check.That(
                    "workspace-select: InspectorSummary reports active state and position",
                    state.InspectorSummary.Contains("Pickup_1 [Active]", StringComparison.Ordinal)
                        && state.InspectorSummary.Contains("Pos={x: 3, y: 0.75, z: 3}", StringComparison.Ordinal));
                Check.That(
                    "workspace-select: Transform component has properties populated",
                    state.InspectorComponents[0].Properties.Count > 0);

                state.SelectByIndex(host, session, 0);
                string firstId = state.SelectedObjectId;
                string secondExpectedId = state.HierarchyEntries[1].FileId.ToString(
                    System.Globalization.CultureInfo.InvariantCulture);

                state.SelectNext(host, session);
                Check.Equal(
                    "workspace-select: SelectNext advances from index 0 to index 1",
                    secondExpectedId,
                    state.SelectedObjectId);

                state.SelectByIndex(host, session, state.HierarchyEntries.Count - 1);
                state.SelectNext(host, session);
                Check.Equal(
                    "workspace-select: SelectNext wraps around to index 0",
                    firstId,
                    state.SelectedObjectId);
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

                string initJson = shell.HandleNativeEvent(
                    ShellEvent.Init(project).ToJsonString());
                JsonObject? initDto = JsonNode.Parse(initJson) as JsonObject;
                Check.That("shell-host: init event returns JSON DTO", initDto != null);
                Check.Equal(
                    "shell-host: init DTO activeScenePath",
                    ProjectSceneRelative,
                    initDto?["activeScenePath"]?.GetValue<string>() ?? string.Empty);

                string selectJson = shell.HandleNativeEvent(
                    ShellEvent.SelectById("1000000021").ToJsonString());
                JsonObject? selectDto = JsonNode.Parse(selectJson) as JsonObject;
                Check.Equal(
                    "shell-host: select by id updates selectedObjectId",
                    "1000000021",
                    selectDto?["selectedObjectId"]?.GetValue<string>() ?? string.Empty);

                int countBefore = shell.State.HierarchyEntries.Count;
                shell.ApplyEvent(
                    ShellEvent.InvokeCommand(
                        "scene.create_object",
                        new JsonObject
                        {
                            ["name"] = "ShellCreatedObject",
                            ["parentFileId"] = ArenaFileId,
                        }));
                Check.Equal(
                    "shell-host: command event invokes tool and refreshes hierarchy count",
                    countBefore + 1,
                    shell.State.HierarchyEntries.Count);

                shell.ApplyEvent(
                    ShellEvent.InvokeCommand(
                        "scene.set_property",
                        new JsonObject
                        {
                            ["fileId"] = PickupFileId,
                            ["path"] = "m_Name",
                            ["value"] = "Pickup_Renamed_By_Shell",
                        }));
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
                ShellRunOptions options = new ShellRunOptions
                {
                    TargetProjectPath = project,
                    Headless = true,
                    MaxFrames = 3,
                    Width = 320,
                    Height = 180,
                    CaptureOut = capturePath,
                };
                options.Events.Add(ShellEvent.Init(project));
                options.Events.Add(ShellEvent.SelectById("1000000091"));

                JsonObject result;
                try
                {
                    result = shell.Run(options);
                }
                catch (McpError.InvalidRequest error)
                    when (error.Message.Contains("unitas_editor_run_shell export is not available", StringComparison.Ordinal))
                {
                    return;
                }

                Check.Equal(
                    "shell-offscreen: native run reports ok=true",
                    true,
                    result["ok"]?.GetValue<bool>() ?? false);
                Check.That(
                    "shell-offscreen: renders non-background pixels",
                    (result["nonBackgroundPixels"]?.GetValue<long>() ?? 0L) > 0L);
                Check.Equal(
                    "shell-offscreen: lastWorkspaceState selectedObjectName is Pickup_1",
                    "Pickup_1",
                    result["lastWorkspaceState"]?["selectedObjectName"]?.GetValue<string>() ?? string.Empty);
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
