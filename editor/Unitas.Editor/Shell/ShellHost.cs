using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Shell
{
    public sealed class ShellRunOptions
    {
        public string TargetProjectPath { get; set; } = string.Empty;

        public string? ShellProjectPath { get; set; }

        public bool Headless { get; set; }

        public long? MaxFrames { get; set; }

        public int Width { get; set; } = 1280;

        public int Height { get; set; } = 720;

        public string? CaptureOut { get; set; }

        public IList<ShellEvent> Events { get; } = new List<ShellEvent>();

        public JsonObject ToNativeOptionsJson()
        {
            JsonObject obj = new JsonObject
            {
                ["targetProjectPath"] = TargetProjectPath,
                ["headless"] = Headless,
                ["width"] = Width,
                ["height"] = Height,
            };

            if (MaxFrames.HasValue)
            {
                obj["maxFrames"] = MaxFrames.Value;
            }

            if (!string.IsNullOrWhiteSpace(CaptureOut))
            {
                obj["captureOut"] = CaptureOut;
            }

            if (Events.Count > 0)
            {
                JsonArray eventsArray = new JsonArray();
                foreach (ShellEvent ev in Events)
                {
                    eventsArray.Add(ev.ToJsonObject());
                }

                obj["events"] = eventsArray;
            }

            return obj;
        }
    }

    public sealed class ShellHost
    {
        public const string ShellAgentName = "workspace-shell";
        public const string ShellClientName = "shell-ui";
        public const string ShellAuthorizedBy = "person-local";

        private readonly McpHost host;
        private readonly string sessionId;
        private readonly WorkspaceState workspaceState;
        private readonly ShellPlay playController;

        public ShellHost(McpHost host)
            : this(host, new WorkspaceState())
        {
        }

        public ShellHost(McpHost host, WorkspaceState workspaceState)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(workspaceState);
            this.host = host;
            this.workspaceState = workspaceState;
            sessionId = host.CreateSession(
                new SessionIdentity(ShellAgentName, ShellClientName, ShellAuthorizedBy));
            host.GrantScope(sessionId, Scope.EditContent, GrantAuthority.Person);
            host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);
            playController = new ShellPlay(host, sessionId, workspaceState);
        }

        public McpHost Host => host;

        public string SessionId => sessionId;

        public WorkspaceState State => workspaceState;

        public ShellPlay Play => playController;

        public string HandleNativeEvent(string eventJson)
        {
            ShellEvent parsed = ShellEvent.Parse(eventJson);
            ApplyEvent(parsed);
            return workspaceState.ToJson();
        }

        public void ApplyEvent(ShellEvent shellEvent)
        {
            ArgumentNullException.ThrowIfNull(shellEvent);

            if (string.Equals(shellEvent.Type, ShellEvent.TypeInit, StringComparison.Ordinal))
            {
                string target = !string.IsNullOrWhiteSpace(shellEvent.ProjectPath)
                    ? shellEvent.ProjectPath!
                    : workspaceState.ProjectPath;
                workspaceState.InitializeProject(host, sessionId, target);
                return;
            }

            if (string.Equals(shellEvent.Type, ShellEvent.TypeSelect, StringComparison.Ordinal))
            {
                if (!string.IsNullOrWhiteSpace(shellEvent.TargetId))
                {
                    workspaceState.SelectById(host, sessionId, shellEvent.TargetId!);
                }
                else if (shellEvent.Index.HasValue)
                {
                    workspaceState.SelectByIndex(host, sessionId, shellEvent.Index.Value);
                }

                return;
            }

            if (string.Equals(shellEvent.Type, ShellEvent.TypeSelectNext, StringComparison.Ordinal))
            {
                workspaceState.SelectNext(host, sessionId);
                return;
            }

            if (string.Equals(shellEvent.Type, ShellEvent.TypeCommand, StringComparison.Ordinal)
                && !string.IsNullOrWhiteSpace(shellEvent.Command))
            {
                DispatchCommandEvent(shellEvent.Command!, shellEvent.Arguments);
            }
        }

        public JsonObject Run(ShellRunOptions options, string? explicitLibraryPath = null)
        {
            ArgumentNullException.ThrowIfNull(options);

            if (!string.IsNullOrWhiteSpace(options.TargetProjectPath))
            {
                workspaceState.ProjectPath = Path.GetFullPath(options.TargetProjectPath);
            }
            else if (string.IsNullOrWhiteSpace(workspaceState.ProjectPath))
            {
                workspaceState.ProjectPath = Directory.GetCurrentDirectory();
            }

            string resolvedShellProject = ResolveShellProjectPath(options.ShellProjectPath);
            if (!NativeLibraryLoader.TryOpen(
                explicitLibraryPath,
                out NativeLibraryLoader? loader,
                out NativeLoadOutcome failure)
                || loader == null)
            {
                throw new McpError.InvalidRequest(
                    "failed to load unitas-editor-abi native library for shell: " + failure.Reason);
            }

            string optionsJson = options.ToNativeOptionsJson().ToJsonString();
            string rawResponse = loader.RunShellJson(
                resolvedShellProject,
                optionsJson,
                HandleNativeEvent);

            JsonNode? parsed;
            try
            {
                parsed = JsonNode.Parse(rawResponse);
            }
            catch (JsonException error)
            {
                throw new McpError.ParseError("invalid shell response JSON: " + error.Message);
            }

            if (parsed is JsonObject obj)
            {
                if (obj["error"] is JsonObject errObj)
                {
                    string msg = errObj["message"]?.GetValue<string>() ?? "native shell run failed";
                    throw new McpError.InvalidRequest(msg);
                }

                return obj;
            }

            return new JsonObject
            {
                ["ok"] = false,
            };
        }

        public static string ResolveShellProjectPath(string? explicitShellProjectPath = null)
        {
            if (!string.IsNullOrWhiteSpace(explicitShellProjectPath))
            {
                return Path.GetFullPath(explicitShellProjectPath);
            }

            string? envPath = Environment.GetEnvironmentVariable("UNITAS_SHELL_PROJECT_PATH");
            if (!string.IsNullOrWhiteSpace(envPath) && Directory.Exists(envPath))
            {
                return Path.GetFullPath(envPath);
            }

            string[] baseDirs =
            {
                Directory.GetCurrentDirectory(),
                AppContext.BaseDirectory,
            };

            foreach (string startDir in baseDirs)
            {
                if (string.IsNullOrWhiteSpace(startDir))
                {
                    continue;
                }

                DirectoryInfo? current = new DirectoryInfo(startDir);
                for (int depth = 0; depth < 7 && current != null; depth++)
                {
                    string candidate = Path.Combine(current.FullName, "editor", "shell-project");
                    if (Directory.Exists(candidate))
                    {
                        return candidate;
                    }

                    current = current.Parent;
                }
            }

            return Path.Combine(Directory.GetCurrentDirectory(), "editor", "shell-project");
        }

        private void DispatchCommandEvent(string command, JsonObject? arguments)
        {
            if (command.StartsWith("play.", StringComparison.Ordinal))
            {
                playController.Dispatch(command, arguments);
                return;
            }

            if (string.Equals(command, "scene.open", StringComparison.Ordinal))
            {
                string? path = arguments?["path"]?.GetValue<string>();
                if (!string.IsNullOrWhiteSpace(path))
                {
                    workspaceState.OpenScene(host, sessionId, path!);
                }

                return;
            }

            try
            {
                JsonObject callArgs = arguments != null
                    ? (JsonObject)arguments.DeepClone()
                    : new JsonObject();
                host.CallTool(sessionId, command, callArgs);
                workspaceState.RefreshFromOpenScene(host, sessionId);
            }
            catch (McpError)
            {
            }
        }
    }
}
