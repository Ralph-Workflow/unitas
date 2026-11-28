using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;
using Unitas.Editor.Shell;

namespace Unitas.Editor.Tests
{
    public static class ShellPlayTests
    {
        private const string SampleSceneRelative = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";

        public static void Run()
        {
            EnterPauseStepAndExitTransitionWorkspaceStateWithStubWorker();
            ToggleTransitionsBetweenStoppedPlayingAndPausedStates();
            ShellHostRoutesPlayCommandEventsThroughShellPlay();
            MissingScopeOrWorkerFailureTransitionsWorkspaceStateToError();
            SearchSetCaptureRendersNonUniformSearchUiWhenAbiPresent();
        }

        private static void EnterPauseStepAndExitTransitionWorkspaceStateWithStubWorker()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            using StubWorker worker = new StubWorker();
            using PlaySessionManager manager = new PlaySessionManager(_ => worker);
            PlayTools.Register(host, manager);

            string session = host.CreateSession(
                new SessionIdentity(
                    ShellHost.ShellAgentName,
                    ShellHost.ShellClientName,
                    ShellHost.ShellAuthorizedBy));
            host.GrantScope(session, Scope.ExecuteProjectCode, GrantAuthority.Person);

            WorkspaceState state = new WorkspaceState();
            ShellPlay play = new ShellPlay(host, session, state);

            Check.Equal("shell-play: initial state is stopped", ShellPlay.StateStopped, play.PlayState);
            Check.That("shell-play: IsStopped is true initially", play.IsStopped);

            play.Enter();
            Check.Equal("shell-play: Enter transitions to playing", ShellPlay.StatePlaying, play.PlayState);
            Check.That("shell-play: IsPlaying is true after Enter", play.IsPlaying);
            Check.That(
                "shell-play: ActivePlaySessionId is recorded on Enter",
                !string.IsNullOrWhiteSpace(play.ActivePlaySessionId));
            Check.Equal("shell-play: PlayStateText reports playing", "Play: playing", state.PlayStateText);

            play.Step(3);
            Check.Equal("shell-play: Step keeps playing state", ShellPlay.StatePlaying, play.PlayState);
            Check.Equal("shell-play: StubWorker advanced 3 frames", 3, (int)worker.Frames);

            play.Pause();
            Check.Equal("shell-play: Pause transitions to paused", ShellPlay.StatePaused, play.PlayState);
            Check.That("shell-play: IsPaused is true after Pause", play.IsPaused);
            Check.Equal("shell-play: PlayStateText reports paused", "Play: paused", state.PlayStateText);

            play.RefreshStatus();
            Check.Equal("shell-play: RefreshStatus preserves paused state", ShellPlay.StatePaused, play.PlayState);

            play.Exit();
            Check.Equal("shell-play: Exit transitions to stopped", ShellPlay.StateStopped, play.PlayState);
            Check.That("shell-play: IsStopped is true after Exit", play.IsStopped);
            Check.That(
                "shell-play: ActivePlaySessionId is cleared on Exit",
                string.IsNullOrWhiteSpace(play.ActivePlaySessionId));
            Check.Equal("shell-play: PlayStateText reports stopped", "Play: stopped", state.PlayStateText);
        }

        private static void ToggleTransitionsBetweenStoppedPlayingAndPausedStates()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            using StubWorker worker = new StubWorker();
            using PlaySessionManager manager = new PlaySessionManager(_ => worker);
            PlayTools.Register(host, manager);

            string session = host.CreateSession(
                new SessionIdentity("shell-toggle", "shell-ui", "person-local"));
            host.GrantScope(session, Scope.ExecuteProjectCode, GrantAuthority.Person);

            WorkspaceState state = new WorkspaceState();
            ShellPlay play = new ShellPlay(host, session, state);

            play.Toggle();
            Check.Equal(
                "shell-play-toggle: Toggle from stopped enters playing",
                ShellPlay.StatePlaying,
                play.PlayState);

            play.Toggle();
            Check.Equal(
                "shell-play-toggle: Toggle from playing exits to stopped",
                ShellPlay.StateStopped,
                play.PlayState);
            Check.That(
                "shell-play-toggle: ActivePlaySessionId is null when stopped",
                play.ActivePlaySessionId == null);

            play.Enter();
            play.Pause();
            Check.Equal(
                "shell-play-toggle: session is paused before toggle",
                ShellPlay.StatePaused,
                play.PlayState);

            play.Toggle();
            Check.Equal(
                "shell-play-toggle: Toggle from paused exits to stopped",
                ShellPlay.StateStopped,
                play.PlayState);
        }

        private static void ShellHostRoutesPlayCommandEventsThroughShellPlay()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            using StubWorker worker = new StubWorker();
            using PlaySessionManager manager = new PlaySessionManager(_ => worker);
            PlayTools.Register(host, manager);

            ShellHost shell = new ShellHost(host);
            Check.Equal(
                "shell-host-play: initial playMode in DTO is stopped",
                ShellPlay.StateStopped,
                shell.State.ToJsonObject()["playMode"]?.GetValue<string>() ?? string.Empty);

            string enterDtoJson = shell.HandleNativeEvent(
                ShellEvent.InvokeCommand(ShellPlay.CommandToggle).ToJsonString());
            JsonObject? enterDto = JsonNode.Parse(enterDtoJson) as JsonObject;
            Check.Equal(
                "shell-host-play: play.toggle event transitions DTO playMode to playing",
                ShellPlay.StatePlaying,
                enterDto?["playMode"]?.GetValue<string>() ?? string.Empty);

            string exitDtoJson = shell.HandleNativeEvent(
                ShellEvent.InvokeCommand(ShellPlay.CommandToggle).ToJsonString());
            JsonObject? exitDto = JsonNode.Parse(exitDtoJson) as JsonObject;
            Check.Equal(
                "shell-host-play: second play.toggle event transitions DTO playMode to stopped",
                ShellPlay.StateStopped,
                exitDto?["playMode"]?.GetValue<string>() ?? string.Empty);
        }

        private static void MissingScopeOrWorkerFailureTransitionsWorkspaceStateToError()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            using StubWorker worker = new StubWorker();
            using PlaySessionManager manager = new PlaySessionManager(_ => worker);
            PlayTools.Register(host, manager);

            string unprivilegedSession = host.CreateSession(
                new SessionIdentity("unprivileged-shell", "shell-ui", "person-local"));
            WorkspaceState state = new WorkspaceState();
            ShellPlay play = new ShellPlay(host, unprivilegedSession, state);

            play.Enter();
            Check.Equal(
                "shell-play-error: missing execute-project-code scope sets PlayState to error",
                ShellPlay.StateError,
                play.PlayState);
            Check.That(
                "shell-play-error: PlayStateText includes Play: Error details",
                state.PlayStateText.StartsWith("Play: Error (", StringComparison.Ordinal));

            host.GrantScope(unprivilegedSession, Scope.ExecuteProjectCode, GrantAuthority.Person);
            worker.FailNextCommand = true;
            play.Enter();
            Check.Equal(
                "shell-play-error: worker failure on play.enter sets PlayState to error",
                ShellPlay.StateError,
                play.PlayState);
        }

        private static void SearchSetCaptureRendersNonUniformSearchUiWhenAbiPresent()
        {
            if (!NativeLibraryLoader.TryOpen(null, out NativeLibraryLoader? loader, out _)
                || loader == null)
            {
                return;
            }

            string project = CreateSampleProjectCopy("search-capture");
            string capturePath = Path.Combine(project, "shell-search-capture.bmp");
            try
            {
                using McpHost host = NewHostWithSceneTools();
                ShellHost shell = new ShellHost(host);
                ShellRunOptions options = new ShellRunOptions
                {
                    TargetProjectPath = project,
                    Headless = true,
                    MaxFrames = 4,
                    Width = 320,
                    Height = 180,
                    CaptureOut = capturePath,
                };
                options.Events.Add(ShellEvent.Init(project));
                options.Events.Add(ShellEvent.SearchSet("pickup"));

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

                Check.Equal("shell-search-capture: native run reports ok=true", true, result["ok"]?.GetValue<bool>() ?? false);
                Check.Equal(
                    "shell-search-capture: lastWorkspaceState carries the applied query",
                    "pickup",
                    result["lastWorkspaceState"]?["searchQuery"]?.GetValue<string>() ?? string.Empty);
                Check.That(
                    "shell-search-capture: lastWorkspaceState reports matching rows",
                    (result["lastWorkspaceState"]?["resultCount"]?.GetValue<int>() ?? 0) > 0);
                Check.That(
                    "shell-search-capture: canvasTexts renders the search query in the search field",
                    CanvasTextsContain(result, "pickup"));
                Check.That(
                    "shell-search-capture: BMP capture file written to disk",
                    File.Exists(capturePath) && new FileInfo(capturePath).Length > 54);
                Check.That(
                    "shell-search-capture: capture BMP is non-uniform where the search UI renders",
                    CountPixelsDifferingFromFirst(capturePath) > 100);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static bool CanvasTextsContain(JsonObject result, string expected)
        {
            if (result["canvasTexts"] is not JsonArray texts)
            {
                return false;
            }

            foreach (JsonNode? node in texts)
            {
                if (string.Equals(node?.GetValue<string>(), expected, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static int CountPixelsDifferingFromFirst(string bmpPath)
        {
            byte[] bytes = File.ReadAllBytes(bmpPath);
            const int HeaderLength = 54;
            if (bytes.Length <= HeaderLength + 3)
            {
                return 0;
            }

            int differing = 0;
            for (int offset = HeaderLength + 3; offset + 2 < bytes.Length; offset += 3)
            {
                if (bytes[offset] != bytes[HeaderLength]
                    || bytes[offset + 1] != bytes[HeaderLength + 1]
                    || bytes[offset + 2] != bytes[HeaderLength + 2])
                {
                    differing++;
                }
            }

            return differing;
        }

        private static McpHost NewHostWithSceneTools()
        {
            McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            ChangeSetStore store = new ChangeSetStore(host);
            SceneTools.Register(host, store);
            return host;
        }

        private static string CreateSampleProjectCopy(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-shell-play-test-" + label + "-" + Guid.NewGuid().ToString("N"));
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

        private sealed class StubWorker : IPlayWorker
        {
            private string currentState = PlaySession.StatePaused;

            public long Frames { get; private set; }

            public bool FailNextCommand { get; set; }

            public bool Disposed { get; private set; }

            public bool HasExited => false;

            public int? ExitCode => null;

            public int? ProcessId => 4242;

            public JsonObject SendCommand(JsonObject command)
            {
                if (FailNextCommand)
                {
                    FailNextCommand = false;
                    throw new McpError.InvalidRequest("simulated worker failure");
                }

                string cmd = command["cmd"]?.GetValue<string>() ?? "state";
                if (string.Equals(cmd, "play", StringComparison.Ordinal))
                {
                    currentState = PlaySession.StatePlaying;
                }
                else if (string.Equals(cmd, "pause", StringComparison.Ordinal))
                {
                    currentState = PlaySession.StatePaused;
                }
                else if (string.Equals(cmd, "step", StringComparison.Ordinal))
                {
                    long delta = command["frames"]?.GetValue<long>() ?? 1L;
                    Frames += delta;
                }
                else if (string.Equals(cmd, "stop", StringComparison.Ordinal))
                {
                    currentState = PlaySession.StateStopped;
                }

                return new JsonObject
                {
                    ["state"] = currentState,
                    ["frames"] = Frames,
                    ["delta_seconds"] = 1.0 / 60.0,
                    ["random_seed"] = 12345UL,
                };
            }

            public void Kill()
            {
            }

            public void Dispose()
            {
                Disposed = true;
            }
        }
    }
}
