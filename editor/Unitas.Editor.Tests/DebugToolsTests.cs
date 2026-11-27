using System;
using System.Net;
using System.Net.Sockets;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using Unitas.Debugging;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class DebugToolsTests
    {
        public static void Run()
        {
            DapClientDrivesTheRealDebugAdapterOverLoopback();
            TheDebugToolsAreRegisteredWithTheirScopes();
            WaitStoppedReportsATimeoutInsteadOfHanging();
        }

        private static void DapClientDrivesTheRealDebugAdapterOverLoopback()
        {
            TcpListener listener = DebugAgent.BeginLoopbackListener(IPAddress.Loopback, 0);
            int port = ((IPEndPoint)listener.LocalEndpoint).Port;
            Task<TcpClient> accepted = listener.AcceptTcpClientAsync();

            SequencePointTable table = new SequencePointTable();
            table.Add(101, "/game/Player.cs", 15, "Player.Update", new[] { "count", "label" });
            table.Add(102, "/game/Player.cs", 20, "Player.Helper", new[] { "count" });
            table.Add(103, "/game/Player.cs", 21, "Player.Helper", new[] { "count" });
            table.Add(104, "/game/Player.cs", 11, "Player.Update", new[] { "count" });
            table.Add(105, "/game/Player.cs", 30, "Player.Deep", new[] { "count" });
            table.Add(106, "/game/Player.cs", 12, "Player.Update", new[] { "count" });
            DebugAgent.RegisterSequencePoints(table);

            using DapClient client = DapClient.Connect(port);
            using TcpClient serverSide = accepted.GetAwaiter().GetResult();
            DebugSession session = DebugAgent.BeginSession(serverSide.GetStream());
            try
            {
                JsonObject initialize = client.Initialize();
                Check.That("dap_client: initialize succeeds", ReadBool(initialize["success"]));
                Check.Equal("dap_client: the first response correlates with the first request", 1, ReadInt(initialize["request_seq"]));
                Check.Equal("dap_client: the response names the command", "initialize", ReadString(initialize["command"]));

                Check.That("dap_client: attach succeeds", ReadBool(client.Attach()["success"]));
                JsonObject setBreakpoints = client.SetBreakpoints("/game/Player.cs", new[] { 15 });
                JsonArray verified = (JsonArray)setBreakpoints["body"]!["breakpoints"]!;
                Check.Equal("dap_client: one breakpoint entry comes back", 1, verified.Count);
                Check.That("dap_client: the breakpoint is verified", ReadBool(verified[0]?["verified"]));
                Check.That("dap_client: configurationDone succeeds", ReadBool(client.ConfigurationDone()["success"]));

                JsonObject threads = client.Threads();
                Check.Equal("dap_client: the adapter reports one thread", 1, ((JsonArray)threads["body"]!["threads"]!).Count);
                Check.Equal("dap_client: the threads response correlates with its request", 5, ReadInt(threads["request_seq"]));

                Task worker = Task.Run(() =>
                {
                    DebugAgent.Hit(101, new object?[] { 7, "abc" }, 0);
                    DebugAgent.Hit(102, new object?[] { 2 }, 1);
                    DebugAgent.Hit(103, new object?[] { 3 }, 1);
                    DebugAgent.Hit(104, new object?[] { 4 }, 0);
                    DebugAgent.Hit(105, new object?[] { 5 }, 1);
                    DebugAgent.Hit(106, new object?[] { 6 }, 0);
                });

                JsonObject? stopped = client.WaitForStopped(TimeSpan.FromSeconds(5));
                Check.That("dap_client: the stopped event arrives", stopped != null);
                Check.Equal("dap_client: the stopped event is named", "stopped", ReadString(stopped?["event"]));
                Check.Equal("dap_client: the stop reason is the breakpoint", "breakpoint", ReadString(stopped?["body"]?["reason"]));

                JsonObject stack = client.StackTrace();
                JsonArray frames = (JsonArray)stack["body"]!["stackFrames"]!;
                Check.Equal("dap_client: the stack holds one frame", 1, frames.Count);
                Check.Equal("dap_client: the frame names the method", "Player.Update", ReadString(frames[0]?["name"]));
                Check.Equal("dap_client: the frame reports the line", 15, ReadInt(frames[0]?["line"]));
                Check.Equal("dap_client: the frame reports the source path", "/game/Player.cs", ReadString(frames[0]?["source"]?["path"]));

                JsonObject scopes = client.Scopes();
                JsonArray scopeList = (JsonArray)scopes["body"]!["scopes"]!;
                Check.Equal("dap_client: one scope is exposed", 1, scopeList.Count);
                int reference = ReadInt(scopeList[0]?["variablesReference"]);
                JsonObject variables = client.Variables(reference);
                JsonArray locals = (JsonArray)variables["body"]!["variables"]!;
                Check.Equal("dap_client: every captured local is exposed", 2, locals.Count);
                Check.Equal("dap_client: a local keeps its name", "count", ReadString(locals[0]?["name"]));
                Check.Equal("dap_client: a numeric local renders its value", "7", ReadString(locals[0]?["value"]));
                Check.Equal("dap_client: a string local renders its value", "abc", ReadString(locals[1]?["value"]));

                JsonObject evaluate = client.Evaluate("count");
                Check.That("dap_client: evaluate succeeds", ReadBool(evaluate["success"]));
                Check.Equal("dap_client: evaluate returns the local value", "7", ReadString(evaluate["body"]?["result"]));

                Check.That("dap_client: next succeeds", ReadBool(client.Next()["success"]));
                JsonObject? overStop = client.WaitForStopped(TimeSpan.FromSeconds(5));
                Check.That("dap_client: stepping over stops again", overStop != null);
                Check.Equal("dap_client: stepping over lands on the caller line", 11, StopLine(client));

                Check.That("dap_client: stepIn succeeds", ReadBool(client.StepIn()["success"]));
                JsonObject? inStop = client.WaitForStopped(TimeSpan.FromSeconds(5));
                Check.That("dap_client: stepping in stops again", inStop != null);
                Check.Equal("dap_client: stepping in lands on the helper line", 30, StopLine(client));

                Check.That("dap_client: stepOut succeeds", ReadBool(client.StepOut()["success"]));
                JsonObject? outStop = client.WaitForStopped(TimeSpan.FromSeconds(5));
                Check.That("dap_client: stepping out stops again", outStop != null);
                Check.Equal("dap_client: stepping out lands on the caller line", 12, StopLine(client));

                Check.That("dap_client: continue succeeds", ReadBool(client.Continue()["success"]));
                Check.That("dap_client: the instrumented thread resumes", worker.Wait(TimeSpan.FromSeconds(5)));
                Check.That("dap_client: disconnect succeeds", ReadBool(client.Disconnect()["success"]));
            }
            finally
            {
                session.Dispose();
                listener.Stop();
            }
        }

        private static void TheDebugToolsAreRegisteredWithTheirScopes()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            using PlaySessionManager manager = new PlaySessionManager((bundle, dapPort) => new FakeWorker());
            BuiltinDefaults.RegisterAll(host, manager);

            foreach (string name in new[]
            {
                "debug.set_breakpoints",
                "debug.wait_stopped",
                "debug.stack",
                "debug.locals",
                "debug.continue",
                "debug.step",
                "debug.evaluate",
            })
            {
                Check.That("debug_tools: the live host registers " + name, FindTool(host, name) != null);
            }

            foreach (string name in new[]
            {
                "debug.set_breakpoints",
                "debug.wait_stopped",
                "debug.stack",
                "debug.locals",
                "debug.continue",
                "debug.step",
            })
            {
                Check.That("debug_tools: " + name + " requires read-project", ScopeOf(host, name) == Scope.ReadProject);
            }

            Check.That(
                "debug_tools: debug.evaluate requires execute-project-code",
                ScopeOf(host, "debug.evaluate") == Scope.ExecuteProjectCode);

            string sessionId = host.CreateSession(new SessionIdentity("agent-debug", "http", "person-1"));
            bool refused = false;
            try
            {
                host.CallTool(sessionId, "debug.evaluate", new JsonObject { ["expression"] = "count" });
            }
            catch (McpError.MissingScope missing)
            {
                refused = missing.Scope == Scope.ExecuteProjectCode;
            }

            Check.That("debug_tools: debug.evaluate refuses a caller without the code scope", refused);
        }

        private static void WaitStoppedReportsATimeoutInsteadOfHanging()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            using PlaySessionManager manager = new PlaySessionManager((bundle, dapPort) => new FakeWorker());
            BuiltinDefaults.RegisterAll(host, manager);
            string sessionId = host.CreateSession(new SessionIdentity("agent-debug", "http", "person-1"));
            host.GrantScope(sessionId, Scope.ReadProject, GrantAuthority.Person);
            host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);

            JsonObject enter = (JsonObject)host.CallTool(sessionId, "play.enter", new JsonObject { ["debug"] = true });
            JsonObject entered = (JsonObject)enter["content"]!;
            Check.That("debug_tools: play.enter reports the debug session", ReadBool(entered["debug"]));
            Check.That("debug_tools: play.enter reports the debug port", entered["debugPort"] != null);

            DateTime started = DateTime.UtcNow;
            JsonObject waited = (JsonObject)host.CallTool(
                sessionId,
                "debug.wait_stopped",
                new JsonObject { ["timeoutMs"] = 250 });
            double elapsed = (DateTime.UtcNow - started).TotalSeconds;
            JsonObject content = (JsonObject)waited["content"]!;
            Check.That("debug_tools: wait_stopped returns without hanging", elapsed < 10);
            Check.That("debug_tools: wait_stopped reports that nothing stopped", !ReadBool(content["stopped"]));
            Check.That("debug_tools: wait_stopped reports the timeout", ReadBool(content["timedOut"]));
        }

        private static int StopLine(DapClient client)
        {
            JsonObject stack = client.StackTrace();
            JsonArray frames = (JsonArray)stack["body"]!["stackFrames"]!;
            return frames.Count == 0 ? -1 : ReadInt(frames[0]?["line"]);
        }

        private static ToolDefinition? FindTool(McpHost host, string name)
        {
            foreach (ToolDefinition candidate in host.ListTools())
            {
                if (candidate.Name == name)
                {
                    return candidate;
                }
            }

            return null;
        }

        private static Scope ScopeOf(McpHost host, string name)
        {
            ToolDefinition? definition = FindTool(host, name);
            return definition?.RequiredScope ?? Scope.Review;
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

        private sealed class FakeWorker : IPlayWorker
        {
            public bool HasExited => false;

            public int? ExitCode => null;

            public int? ProcessId => null;

            public JsonObject SendCommand(JsonObject command)
            {
                return new JsonObject
                {
                    ["state"] = PlaySession.StatePlaying,
                    ["frames"] = 0,
                    ["delta_seconds"] = 1.0 / 60.0,
                    ["random_seed"] = 1,
                };
            }

            public void Kill()
            {
            }

            public void Dispose()
            {
            }
        }
    }
}
