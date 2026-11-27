using System;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Shell
{
    public sealed class ShellPlay
    {
        public const string CommandEnter = "play.enter";
        public const string CommandExit = "play.exit";
        public const string CommandToggle = "play.toggle";
        public const string CommandPause = "play.pause";
        public const string CommandStep = "play.step";
        public const string CommandState = "play.state";

        public const string StateStopped = "stopped";
        public const string StatePlaying = "playing";
        public const string StatePaused = "paused";
        public const string StateError = "error";

        private readonly McpHost host;
        private readonly string sessionId;
        private readonly WorkspaceState state;

        public ShellPlay(McpHost host, string sessionId, WorkspaceState state)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            ArgumentNullException.ThrowIfNull(state);
            this.host = host;
            this.sessionId = sessionId;
            this.state = state;
        }

        public McpHost Host => host;

        public string SessionId => sessionId;

        public WorkspaceState State => state;

        public string PlayState => state.PlayState;

        public string? ActivePlaySessionId => state.ActivePlaySessionId;

        public bool IsPlaying =>
            string.Equals(state.PlayState, StatePlaying, StringComparison.OrdinalIgnoreCase);

        public bool IsPaused =>
            string.Equals(state.PlayState, StatePaused, StringComparison.OrdinalIgnoreCase);

        public bool IsStopped =>
            string.Equals(state.PlayState, StateStopped, StringComparison.OrdinalIgnoreCase);

        public void Enter(JsonObject? arguments = null)
        {
            ExecuteCommand(host, sessionId, state, CommandEnter, arguments);
        }

        public void Exit(JsonObject? arguments = null)
        {
            ExecuteCommand(host, sessionId, state, CommandExit, arguments);
        }

        public void Toggle(JsonObject? arguments = null)
        {
            ExecuteCommand(host, sessionId, state, CommandToggle, arguments);
        }

        public void Pause(JsonObject? arguments = null)
        {
            ExecuteCommand(host, sessionId, state, CommandPause, arguments);
        }

        public void Step(long frames = 1L)
        {
            ExecuteCommand(
                host,
                sessionId,
                state,
                CommandStep,
                new JsonObject
                {
                    ["frames"] = frames,
                });
        }

        public void RefreshStatus(JsonObject? arguments = null)
        {
            if (string.IsNullOrWhiteSpace(state.ActivePlaySessionId)
                && (arguments == null || arguments["playSessionId"] == null))
            {
                state.PlayState = StateStopped;
                state.RebuildFormattedTexts();
                return;
            }

            ExecuteCommand(host, sessionId, state, CommandState, arguments);
        }

        public void Dispatch(string command, JsonObject? arguments = null)
        {
            ExecuteCommand(host, sessionId, state, command, arguments);
        }

        public static string ResolveEffectiveCommand(string currentPlayState, string command)
        {
            ArgumentNullException.ThrowIfNull(command);
            if (!string.Equals(command, CommandToggle, StringComparison.Ordinal))
            {
                return command;
            }

            bool active = string.Equals(currentPlayState, StatePlaying, StringComparison.OrdinalIgnoreCase)
                || string.Equals(currentPlayState, StatePaused, StringComparison.OrdinalIgnoreCase);
            return active ? CommandExit : CommandEnter;
        }

        public static void ExecuteCommand(
            McpHost host,
            string sessionId,
            WorkspaceState state,
            string command,
            JsonObject? arguments)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            ArgumentNullException.ThrowIfNull(state);
            ArgumentNullException.ThrowIfNull(command);

            string effectiveCommand = ResolveEffectiveCommand(state.PlayState, command);
            try
            {
                JsonObject callArgs = BuildCallArguments(state.ActivePlaySessionId, arguments);
                JsonNode envelope = host.CallTool(sessionId, effectiveCommand, callArgs);
                ApplyPayload(state, UnwrapContent(envelope), effectiveCommand);
                state.RebuildFormattedTexts();
            }
            catch (McpError error)
            {
                state.PlayState = StateError;
                state.RebuildFormattedTexts();
                state.PlayStateText = "Play: Error (" + error.Message + ")";
            }
        }

        public static void ApplyPayload(WorkspaceState state, JsonObject? content, string effectiveCommand)
        {
            ArgumentNullException.ThrowIfNull(state);
            if (content == null)
            {
                return;
            }

            string? playSessionId = content["playSessionId"]?.GetValue<string>();
            if (!string.IsNullOrWhiteSpace(playSessionId))
            {
                state.ActivePlaySessionId = playSessionId;
            }

            string? reportedState = content["state"]?.GetValue<string>();
            if (!string.IsNullOrWhiteSpace(reportedState))
            {
                state.PlayState = reportedState!;
            }
            else if (string.Equals(effectiveCommand, CommandExit, StringComparison.Ordinal))
            {
                state.PlayState = StateStopped;
            }

            if (string.Equals(state.PlayState, StateStopped, StringComparison.OrdinalIgnoreCase))
            {
                state.ActivePlaySessionId = null;
            }
        }

        private static JsonObject BuildCallArguments(string? activePlaySessionId, JsonObject? arguments)
        {
            JsonObject callArgs = arguments != null
                ? (JsonObject)arguments.DeepClone()
                : new JsonObject();
            if (!string.IsNullOrWhiteSpace(activePlaySessionId) && callArgs["playSessionId"] == null)
            {
                callArgs["playSessionId"] = activePlaySessionId;
            }

            return callArgs;
        }

        private static JsonObject? UnwrapContent(JsonNode? envelope)
        {
            if (envelope is JsonObject obj)
            {
                if (obj["content"] is JsonObject content)
                {
                    return content;
                }

                return obj;
            }

            return null;
        }
    }
}
