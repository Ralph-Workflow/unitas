using System;
using System.Diagnostics;
using System.IO;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Play
{
    /// <summary>
    /// The end-to-end DIA-002 proof: the real MCP registry drives a real player
    /// worker, the worker serves a real debug adapter, and an agent breaks in the
    /// real roll-a-ball sample, reads the stop, and lets the session run on.
    /// </summary>
    public static class DebugSessionTests
    {
        private const string SampleScriptRelativePath =
            "samples/roll-a-ball/Assets/Scripts/PlayerController.cs";

        // The instrumented build probes this statement of PlayerController.Update
        // on every frame, with moveX and moveZ in scope at the probe.
        private const int BreakpointLine = 48;

        private const int WaitTimeoutMilliseconds = 60000;

        private const int StageTimeoutMilliseconds = 300000;

        public static void Run()
        {
            string? repositoryRoot = WorkerProcess.FindRepositoryRoot();
            Check.That("debug_session: the repository root is found", repositoryRoot != null);
            if (repositoryRoot == null)
            {
                return;
            }

            string scriptPath = Path.Combine(
                repositoryRoot!,
                SampleScriptRelativePath.Replace('/', Path.DirectorySeparatorChar));
            Check.That("debug_session: the sample script exists", File.Exists(scriptPath));

            string stageDirectory = Path.Combine(
                Path.GetTempPath(),
                "unitas-editor-debug-session-" + Guid.NewGuid().ToString("N"));
            try
            {
                StageInstrumentedBundle(stageDirectory);
                if (!Directory.Exists(Path.Combine(stageDirectory, "Managed")))
                {
                    Check.That("debug_session: the staged bundle carries the managed payload", false);
                    return;
                }

                DriveTheDebugSession(stageDirectory, scriptPath);
            }
            finally
            {
                TryDeleteDirectory(stageDirectory);
            }
        }

        private static void StageInstrumentedBundle(string stageDirectory)
        {
            string player;
            try
            {
                player = WorkerProcess.ResolvePlayerBinaryPath();
            }
            catch (McpError error)
            {
                Check.That("debug_session: the player binary is found (" + error.Message + ")", false);
                return;
            }

            ProcessStartInfo startInfo = new ProcessStartInfo
            {
                FileName = player,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true,
            };
            startInfo.ArgumentList.Add("--sample");
            startInfo.ArgumentList.Add(stageDirectory);
            startInfo.ArgumentList.Add("--debug");
            startInfo.ArgumentList.Add("--frames");
            startInfo.ArgumentList.Add("1");

            using Process process = Process.Start(startInfo)!;
            string stdout = process.StandardOutput.ReadToEnd();
            string stderr = process.StandardError.ReadToEnd();
            process.WaitForExit(StageTimeoutMilliseconds);

            Check.That(
                "debug_session: the player stages the instrumented bundle ("
                    + stdout.Trim()
                    + " "
                    + stderr.Trim()
                    + ")",
                process.HasExited && process.ExitCode == 0);
        }

        private static void DriveTheDebugSession(string stageDirectory, string scriptPath)
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults();
            using PlaySessionManager manager = new PlaySessionManager();
            PlayTools.Register(host, manager);
            DebugTools.Register(host, manager);

            string agentSession = host.CreateSession(new SessionIdentity("agent-debug", "http", "person-1"));
            host.GrantScope(agentSession, Scope.ExecuteProjectCode, GrantAuthority.Person);
            string readerSession = host.CreateSession(new SessionIdentity("agent-debug-reader", "http", "person-1"));

            JsonObject enter = (JsonObject)host.CallTool(agentSession, "play.enter", new JsonObject
            {
                ["bundleDirectory"] = stageDirectory,
                ["debug"] = true,
            });
            JsonObject entered = (JsonObject)enter["content"]!;
            Check.That("debug_session: the play session reports debugging", ReadBool(entered["debug"]));
            Check.That("debug_session: the play session reports a debug port", entered["debugPort"] != null);
            string playSessionId = entered["playSessionId"]!.GetValue<string>();

            JsonObject breakpoints = (JsonObject)host.CallTool(agentSession, "debug.set_breakpoints", new JsonObject
            {
                ["playSessionId"] = playSessionId,
                ["path"] = scriptPath,
                ["lines"] = new JsonArray(BreakpointLine),
            });
            JsonObject breakpointContent = (JsonObject)breakpoints["content"]!;
            Check.That(
                "debug_session: the breakpoint is accepted by the debugged session",
                ReadBool(breakpointContent["applied"]) || ReadBool(breakpointContent["pending"]));

            // The tick that hits the breakpoint parks in the worker, so it runs on
            // its own task and the debug tools stay free to drive the session.
            Task<JsonNode> tick = Task.Run(() => host.CallTool(agentSession, "play.step", new JsonObject
            {
                ["playSessionId"] = playSessionId,
                ["frames"] = 1,
            }));

            JsonObject waited = (JsonObject)host.CallTool(agentSession, "debug.wait_stopped", new JsonObject
            {
                ["playSessionId"] = playSessionId,
                ["timeoutMs"] = WaitTimeoutMilliseconds,
            });
            JsonObject waitContent = (JsonObject)waited["content"]!;
            bool stopped = ReadBool(waitContent["stopped"]);
            Check.That("debug_session: the debugged session stops at the breakpoint", stopped);

            if (stopped)
            {
                AssertTheStopFrame(host, agentSession, playSessionId, scriptPath);
                string? moveX = AssertTheLocals(host, agentSession, playSessionId);
                AssertEvaluateScope(host, agentSession, readerSession, playSessionId, moveX);
            }

            JsonObject continued = (JsonObject)host.CallTool(agentSession, "debug.continue", new JsonObject
            {
                ["playSessionId"] = playSessionId,
            });
            JsonObject continueContent = (JsonObject)continued["content"]!;
            Check.That("debug_session: the debugged session continues", ReadBool(continueContent["continued"]));
            Check.That(
                "debug_session: the parked tick completes",
                tick.Wait(TimeSpan.FromSeconds(30)) && tick.Exception == null);

            JsonObject exited = (JsonObject)host.CallTool(agentSession, "play.exit", new JsonObject
            {
                ["playSessionId"] = playSessionId,
            });
            JsonObject exitContent = (JsonObject)exited["content"]!;
            Check.Equal(
                "debug_session: the play session stops cleanly",
                PlaySession.StateStopped,
                ReadString(exitContent["state"]));
        }

        private static void AssertTheStopFrame(
            McpHost host,
            string sessionId,
            string playSessionId,
            string scriptPath)
        {
            JsonObject stack = (JsonObject)host.CallTool(sessionId, "debug.stack", new JsonObject
            {
                ["playSessionId"] = playSessionId,
            });
            JsonObject stackContent = (JsonObject)stack["content"]!;
            JsonArray frames = (JsonArray)stackContent["frames"]!;
            Check.That("debug_session: the stop carries a stack frame", frames.Count > 0);
            if (frames.Count == 0)
            {
                return;
            }

            JsonObject frame = (JsonObject)frames[0]!;
            Check.Equal(
                "debug_session: the stop frame names the sample script",
                scriptPath,
                ReadString(frame["source"]?["path"]));
            Check.Equal(
                "debug_session: the stop frame reports the breakpoint line",
                BreakpointLine,
                ReadInt(frame["line"]));
        }

        private static string? AssertTheLocals(McpHost host, string sessionId, string playSessionId)
        {
            JsonObject locals = (JsonObject)host.CallTool(sessionId, "debug.locals", new JsonObject
            {
                ["playSessionId"] = playSessionId,
            });
            JsonObject localsContent = (JsonObject)locals["content"]!;
            JsonArray localList = (JsonArray)localsContent["locals"]!;
            Check.That("debug_session: the stop exposes locals", localList.Count > 0);

            string? moveX = FindLocal(localList, "moveX");
            Check.That("debug_session: the locals expose moveX", moveX != null);
            Check.That("debug_session: the locals expose moveZ", FindLocal(localList, "moveZ") != null);
            return moveX;
        }

        private static void AssertEvaluateScope(
            McpHost host,
            string agentSession,
            string readerSession,
            string playSessionId,
            string? moveX)
        {
            bool refused = false;
            try
            {
                host.CallTool(readerSession, "debug.evaluate", new JsonObject
                {
                    ["playSessionId"] = playSessionId,
                    ["expression"] = "moveX",
                });
            }
            catch (McpError.MissingScope missing)
            {
                refused = missing.Scope == Scope.ExecuteProjectCode;
            }

            Check.That(
                "debug_session: debug.evaluate refuses a caller without the code scope",
                refused);

            JsonObject evaluated = (JsonObject)host.CallTool(agentSession, "debug.evaluate", new JsonObject
            {
                ["playSessionId"] = playSessionId,
                ["expression"] = "moveX",
            });
            JsonObject evaluateContent = (JsonObject)evaluated["content"]!;
            Check.Equal(
                "debug_session: debug.evaluate returns the local value",
                moveX ?? string.Empty,
                ReadString(evaluateContent["result"]));
        }

        private static string? FindLocal(JsonArray localList, string name)
        {
            foreach (JsonNode? node in localList)
            {
                if (node is JsonObject entry
                    && string.Equals(ReadString(entry["name"]), name, StringComparison.Ordinal))
                {
                    return ReadString(entry["value"]);
                }
            }

            return null;
        }

        private static void TryDeleteDirectory(string directory)
        {
            try
            {
                if (Directory.Exists(directory))
                {
                    Directory.Delete(directory, true);
                }
            }
            catch (IOException)
            {
            }
            catch (UnauthorizedAccessException)
            {
            }
        }

        private static bool ReadBool(JsonNode? node)
        {
            return node is JsonValue value && value.TryGetValue(out bool parsed) && parsed;
        }

        private static int ReadInt(JsonNode? node)
        {
            return node is JsonValue value && value.TryGetValue(out int number) ? number : 0;
        }

        private static string ReadString(JsonNode? node)
        {
            return node is JsonValue value && value.TryGetValue(out string? text) ? text : string.Empty;
        }
    }
}
