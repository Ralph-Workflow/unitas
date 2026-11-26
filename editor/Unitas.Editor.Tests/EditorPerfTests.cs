using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using System.Threading.Tasks;

namespace Unitas.Editor.Tests
{
    /// <summary>
    /// Editor and MCP performance measurements for the make verify-perf lane. Every
    /// measurement drives the real editor process over its own MCP surface, so the numbers
    /// describe the editor that ships rather than a test double. Each measurement prints one
    /// "name<TAB>value_ms" line that the lane parses.
    /// </summary>
    public static class EditorPerfTests
    {
        public const string StartupMeasurement = "editor_startup";
        public const string ProjectOpenMeasurement = "editor_project_open";
        public const string HierarchyMeasurement = "editor_hierarchy_search_100k";
        public const string PropertyEditUndoMeasurement = "editor_property_edit_undo";
        public const string RoundTripMeasurement = "editor_mcp_round_trip";

        private const string EditorDllRelativePath = "editor/Unitas.Editor/bin/Release/net8.0/Unitas.Editor.dll";
        private const string ProjectRelativePath = "samples/roll-a-ball";
        private const string SceneRelativePath = "Assets/Scenes/RollABall.unity";
        private const string ScaleFloorSceneRelativePath = "Assets/ScaleFloor.unity";
        private const string PersonCredential = "unitas-perf-person";
        private const int ScaleFloorObjectCount = 100000;
        private const int RoundTripSamples = 20;
        private const int ChunkBytes = 1 << 20;

        private static readonly TimeSpan ResponseTimeout = TimeSpan.FromSeconds(120);
        private static readonly TimeSpan HttpStartTimeout = TimeSpan.FromSeconds(60);
        private static readonly string[] MeasurementNames =
        {
            StartupMeasurement,
            ProjectOpenMeasurement,
            HierarchyMeasurement,
            PropertyEditUndoMeasurement,
            RoundTripMeasurement,
        };

        public static int Run(string[] args)
        {
            ArgumentNullException.ThrowIfNull(args);
            string? only = args.Length > 1 ? args[1] : null;
            if (only != null && !IsKnown(only))
            {
                System.Console.Error.WriteLine("unknown measurement: " + only);
                return 2;
            }

            if (only == null || only == StartupMeasurement)
            {
                Report(StartupMeasurement, MeasureStartup());
            }

            if (only == null || only == ProjectOpenMeasurement)
            {
                Report(ProjectOpenMeasurement, MeasureProjectOpen());
            }

            if (only == null || only == HierarchyMeasurement)
            {
                Report(HierarchyMeasurement, MeasureHierarchy());
            }

            if (only == null || only == PropertyEditUndoMeasurement)
            {
                Report(PropertyEditUndoMeasurement, MeasurePropertyEditUndo());
            }

            if (only == null || only == RoundTripMeasurement)
            {
                Report(RoundTripMeasurement, MeasureRoundTrip());
            }

            return 0;
        }

        private static bool IsKnown(string name)
        {
            foreach (string known in MeasurementNames)
            {
                if (string.Equals(known, name, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static void Report(string name, double valueMs)
        {
            System.Console.WriteLine(name + "\t" + valueMs.ToString("0.000", CultureInfo.InvariantCulture));
        }

        private static double MeasureStartup()
        {
            Stopwatch watch = Stopwatch.StartNew();
            using (StdioEditor editor = StdioEditor.Start(EditorDllPath(), RepoRoot()))
            {
                editor.Initialize();
                watch.Stop();
                return watch.Elapsed.TotalMilliseconds;
            }
        }

        private static double MeasureProjectOpen()
        {
            using (StdioEditor editor = StdioEditor.Start(EditorDllPath(), RepoRoot()))
            {
                editor.Initialize();
                Stopwatch watch = Stopwatch.StartNew();
                ToolCall(editor, "project.status", new JsonObject { ["projectPath"] = ProjectPath() });
                watch.Stop();
                return watch.Elapsed.TotalMilliseconds;
            }
        }

        private static double MeasureHierarchy()
        {
            string project = CreateScaleFloorProject();
            try
            {
                using (StdioEditor editor = StdioEditor.Start(EditorDllPath(), RepoRoot()))
                {
                    editor.Initialize();
                    JsonObject scene = new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = ScaleFloorSceneRelativePath,
                    };
                    ToolCall(editor, "scene.open", scene.DeepClone());
                    Stopwatch watch = Stopwatch.StartNew();
                    ToolCall(editor, "scene.hierarchy", scene);
                    watch.Stop();
                    return watch.Elapsed.TotalMilliseconds;
                }
            }
            finally
            {
                Directory.Delete(project, true);
            }
        }

        private static double MeasurePropertyEditUndo()
        {
            using (HttpEditor editor = HttpEditor.Start(EditorDllPath(), RepoRoot()))
            {
                editor.Initialize();
                ToolCall(editor, "permissions.person_grant", new JsonObject
                {
                    ["scope"] = "edit-content",
                    ["personCredential"] = PersonCredential,
                });
                ToolCall(editor, "scene.open", new JsonObject
                {
                    ["projectPath"] = ProjectPath(),
                    ["path"] = SceneRelativePath,
                });
                JsonNode hierarchy = ToolCall(editor, "scene.hierarchy", new JsonObject());
                long fileId = FirstObjectFileId(hierarchy);
                Stopwatch watch = Stopwatch.StartNew();
                ToolCall(editor, "scene.set_property", new JsonObject
                {
                    ["fileId"] = fileId,
                    ["path"] = "m_Name",
                    ["value"] = "PerfEdited",
                });
                ToolCall(editor, "editor.undo", new JsonObject());
                watch.Stop();
                return watch.Elapsed.TotalMilliseconds;
            }
        }

        private static double MeasureRoundTrip()
        {
            using (StdioEditor editor = StdioEditor.Start(EditorDllPath(), RepoRoot()))
            {
                editor.Initialize();
                ToolCall(editor, "project.status", new JsonObject { ["projectPath"] = ProjectPath() });
                double total = 0;
                for (int index = 0; index < RoundTripSamples; index++)
                {
                    Stopwatch watch = Stopwatch.StartNew();
                    ToolCall(editor, "project.status", new JsonObject());
                    watch.Stop();
                    total += watch.Elapsed.TotalMilliseconds;
                }

                return total / RoundTripSamples;
            }
        }

        private static long FirstObjectFileId(JsonNode result)
        {
            JsonNode? envelope = result["structuredContent"];
            if (envelope == null)
            {
                JsonNode? text = result["content"]?[0]?["text"];
                if (text == null)
                {
                    throw new InvalidOperationException("the editor returned no hierarchy envelope");
                }

                envelope = JsonNode.Parse(text.GetValue<string>());
            }

            JsonNode? objects = envelope?["content"]?["objects"];
            if (objects == null)
            {
                throw new InvalidOperationException("the editor returned no hierarchy objects");
            }

            JsonNode first = objects.AsArray()[0] ?? throw new InvalidOperationException("the opened scene has no objects");
            JsonNode? fileId = first["fileId"];
            if (fileId == null)
            {
                throw new InvalidOperationException("the hierarchy object carries no file id");
            }

            return fileId.GetValue<long>();
        }

        private static JsonNode ToolCall(IEditorTransport transport, string toolName, JsonNode arguments)
        {
            JsonNode response = transport.Call(
                "tools/call",
                new JsonObject
                {
                    ["name"] = toolName,
                    ["arguments"] = arguments,
                });
            JsonNode? error = response["error"];
            if (error != null)
            {
                throw new InvalidOperationException("the editor refused " + toolName + ": " + error.ToJsonString());
            }

            JsonNode? result = response["result"];
            if (result == null)
            {
                throw new InvalidOperationException("the editor returned no result for " + toolName);
            }

            return result;
        }

        // The scale floor of the product criteria is a scene with 100,000 GameObjects, so
        // the hierarchy query is measured against a scene of that size. The scene is a
        // minimal Unity tagged YAML document written to a scratch project outside the
        // repository, and it is deleted once the measurement is taken.
        private static string CreateScaleFloorProject()
        {
            string root = Path.Combine(Path.GetTempPath(), "unitas-perf-scale-" + Guid.NewGuid().ToString("N"));
            string assets = Path.Combine(root, "Assets");
            Directory.CreateDirectory(assets);
            string scenePath = Path.Combine(assets, "ScaleFloor.unity");
            using (StreamWriter writer = new StreamWriter(scenePath, false, new UTF8Encoding(false)))
            {
                writer.NewLine = "\n";
                StringBuilder chunk = new StringBuilder(ChunkBytes);
                chunk.Append("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n");
                for (int index = 0; index < ScaleFloorObjectCount; index++)
                {
                    long gameObjectId = 1000000001L + (index * 10L);
                    long transformId = gameObjectId + 1L;
                    chunk.Append("--- !u!1 &").Append(gameObjectId.ToString(CultureInfo.InvariantCulture)).Append('\n');
                    chunk.Append("GameObject:\n");
                    chunk.Append("  m_Component:\n");
                    chunk.Append("  - component: {fileID: ").Append(transformId.ToString(CultureInfo.InvariantCulture)).Append("}\n");
                    chunk.Append("  m_Name: Scale").Append(index.ToString(CultureInfo.InvariantCulture)).Append('\n');
                    chunk.Append("--- !u!4 &").Append(transformId.ToString(CultureInfo.InvariantCulture)).Append('\n');
                    chunk.Append("Transform:\n");
                    chunk.Append("  m_GameObject: {fileID: ").Append(gameObjectId.ToString(CultureInfo.InvariantCulture)).Append("}\n");
                    chunk.Append("  m_Father: {fileID: 0}\n");
                    if (chunk.Length >= ChunkBytes)
                    {
                        writer.Write(chunk.ToString());
                        chunk.Clear();
                    }
                }

                writer.Write(chunk.ToString());
            }

            return root;
        }

        private static string RepoRoot()
        {
            DirectoryInfo? current = new DirectoryInfo(AppContext.BaseDirectory);
            while (current != null)
            {
                if (Directory.Exists(Path.Combine(current.FullName, ProjectRelativePath)))
                {
                    return current.FullName;
                }

                current = current.Parent;
            }

            throw new InvalidOperationException("the repository root was not found above " + AppContext.BaseDirectory);
        }

        private static string ProjectPath()
        {
            return Path.Combine(RepoRoot(), ProjectRelativePath);
        }

        private static string EditorDllPath()
        {
            string path = Path.Combine(RepoRoot(), EditorDllRelativePath);
            if (File.Exists(path))
            {
                return path;
            }

            string beside = Path.Combine(AppContext.BaseDirectory, "Unitas.Editor.dll");
            if (File.Exists(beside))
            {
                return beside;
            }

            throw new FileNotFoundException("the editor binary was not found", path);
        }

        private static string DotnetHost()
        {
            string? processPath = Environment.ProcessPath;
            if (processPath != null
                && string.Equals(
                    Path.GetFileNameWithoutExtension(processPath),
                    "dotnet",
                    StringComparison.OrdinalIgnoreCase))
            {
                return processPath;
            }

            return "dotnet";
        }

        private interface IEditorTransport
        {
            JsonNode Call(string method, JsonNode parameters);

            void Notify(string method, JsonNode parameters);
        }

        private sealed class StdioEditor : IEditorTransport, IDisposable
        {
            private readonly Process process;
            private readonly StreamWriter input;
            private readonly StreamReader output;
            private readonly StringBuilder errorOutput = new StringBuilder();
            private readonly object errorLock = new object();
            private int nextId;

            private StdioEditor(Process process)
            {
                this.process = process;
                input = process.StandardInput;
                output = process.StandardOutput;
                process.EnableRaisingEvents = true;
                process.ErrorDataReceived += (_, args) => RecordError(args.Data);
                process.BeginErrorReadLine();
            }

            public static StdioEditor Start(string editorDllPath, string workingDirectory)
            {
                ProcessStartInfo startInfo = new ProcessStartInfo
                {
                    FileName = DotnetHost(),
                    WorkingDirectory = workingDirectory,
                    RedirectStandardInput = true,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                };
                startInfo.ArgumentList.Add(editorDllPath);
                startInfo.ArgumentList.Add("--stdio");
                Process process = Process.Start(startInfo)
                    ?? throw new InvalidOperationException("the editor process did not start");
                return new StdioEditor(process);
            }

            public void Initialize()
            {
                Call(
                    "initialize",
                    new JsonObject
                    {
                        ["protocolVersion"] = "2024-11-05",
                        ["capabilities"] = new JsonObject(),
                        ["clientInfo"] = new JsonObject
                        {
                            ["name"] = "unitas-perf",
                            ["version"] = "1.0.0",
                        },
                    });
                Notify("notifications/initialized", new JsonObject());
            }

            public JsonNode Call(string method, JsonNode parameters)
            {
                string id = NextId();
                JsonObject request = new JsonObject
                {
                    ["jsonrpc"] = "2.0",
                    ["id"] = id,
                    ["method"] = method,
                    ["params"] = parameters,
                };
                input.WriteLine(request.ToJsonString());
                input.Flush();
                while (true)
                {
                    string line = ReadLine();
                    JsonNode response = JsonNode.Parse(line)
                        ?? throw new InvalidOperationException("the editor sent an empty response");
                    JsonNode? responseId = response["id"];
                    if (responseId != null
                        && string.Equals(responseId.ToString(), id, StringComparison.Ordinal))
                    {
                        return response;
                    }
                }
            }

            public void Notify(string method, JsonNode parameters)
            {
                JsonObject request = new JsonObject
                {
                    ["jsonrpc"] = "2.0",
                    ["method"] = method,
                    ["params"] = parameters,
                };
                input.WriteLine(request.ToJsonString());
                input.Flush();
            }

            public void Dispose()
            {
                try
                {
                    input.Close();
                }
                catch (IOException)
                {
                }
                catch (ObjectDisposedException)
                {
                }

                if (!process.WaitForExit(5000))
                {
                    process.Kill(true);
                }

                process.Dispose();
            }

            private string NextId()
            {
                nextId++;
                return nextId.ToString(CultureInfo.InvariantCulture);
            }

            private string ReadLine()
            {
                Task<string?> pending = output.ReadLineAsync();
                if (!pending.Wait(ResponseTimeout))
                {
                    throw new TimeoutException(
                        "the editor did not answer within "
                        + ResponseTimeout.TotalSeconds.ToString("0", CultureInfo.InvariantCulture)
                        + "s");
                }

                return pending.Result
                    ?? throw new InvalidOperationException("the editor closed its transport");
            }

            private void RecordError(string? line)
            {
                if (line == null)
                {
                    return;
                }

                lock (errorLock)
                {
                    errorOutput.AppendLine(line);
                }
            }
        }

        // The stdio transport is never treated as a person's transport (AUT-007), so the
        // edit-content scope can only be granted on the editor's loopback HTTP surface.
        // That surface serves the same MCP tools, so the edit and the undo are still the
        // editor's own commands.
        private sealed class HttpEditor : IEditorTransport, IDisposable
        {
            private readonly Process process;
            private readonly HttpClient client = new HttpClient();
            private readonly string endpoint;
            private int nextId;
            private string? sessionId;

            private HttpEditor(Process process, string endpoint)
            {
                this.process = process;
                this.endpoint = endpoint;
                client.Timeout = ResponseTimeout;
            }

            public static HttpEditor Start(string editorDllPath, string workingDirectory)
            {
                int port = FreeLoopbackPort();
                ProcessStartInfo startInfo = new ProcessStartInfo
                {
                    FileName = DotnetHost(),
                    WorkingDirectory = workingDirectory,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                };
                startInfo.ArgumentList.Add(editorDllPath);
                startInfo.ArgumentList.Add("--http");
                startInfo.ArgumentList.Add(port.ToString(CultureInfo.InvariantCulture));
                startInfo.Environment["UNITAS_PERSON_CREDENTIAL"] = PersonCredential;
                Process process = Process.Start(startInfo)
                    ?? throw new InvalidOperationException("the editor process did not start");
                process.EnableRaisingEvents = true;
                process.OutputDataReceived += (_, _) => { };
                process.ErrorDataReceived += (_, _) => { };
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
                string endpoint = "http://127.0.0.1:" + port.ToString(CultureInfo.InvariantCulture) + "/";
                return new HttpEditor(process, endpoint);
            }

            public void Initialize()
            {
                Stopwatch watch = Stopwatch.StartNew();
                while (true)
                {
                    try
                    {
                        string body = Post(new JsonObject
                        {
                            ["jsonrpc"] = "2.0",
                            ["id"] = NextId(),
                            ["method"] = "initialize",
                            ["params"] = new JsonObject
                            {
                                ["protocolVersion"] = "2024-11-05",
                                ["capabilities"] = new JsonObject(),
                                ["clientInfo"] = new JsonObject
                                {
                                    ["name"] = "unitas-perf",
                                    ["version"] = "1.0.0",
                                },
                            },
                        });
                        JsonNode response = ParseResponse(body);
                        JsonNode? error = response["error"];
                        if (error != null)
                        {
                            throw new InvalidOperationException(
                                "the editor refused the handshake: " + error.ToJsonString());
                        }

                        Notify("notifications/initialized", new JsonObject());
                        return;
                    }
                    catch (HttpRequestException)
                    {
                        if (watch.Elapsed >= HttpStartTimeout)
                        {
                            throw;
                        }

                        Task.Delay(100).GetAwaiter().GetResult();
                    }
                }
            }

            public JsonNode Call(string method, JsonNode parameters)
            {
                string id = NextId();
                string body = Post(new JsonObject
                {
                    ["jsonrpc"] = "2.0",
                    ["id"] = id,
                    ["method"] = method,
                    ["params"] = parameters,
                });
                JsonNode response = ParseResponse(body);
                JsonNode? responseId = response["id"];
                if (responseId == null
                    || !string.Equals(responseId.ToString(), id, StringComparison.Ordinal))
                {
                    throw new InvalidOperationException("the editor answered another request");
                }

                return response;
            }

            public void Notify(string method, JsonNode parameters)
            {
                Post(new JsonObject
                {
                    ["jsonrpc"] = "2.0",
                    ["method"] = method,
                    ["params"] = parameters,
                });
            }

            public void Dispose()
            {
                client.Dispose();
                if (!process.WaitForExit(5000))
                {
                    process.Kill(true);
                }

                process.Dispose();
            }

            private string NextId()
            {
                nextId++;
                return nextId.ToString(CultureInfo.InvariantCulture);
            }

            private string Post(JsonNode message)
            {
                using HttpRequestMessage request = new HttpRequestMessage(HttpMethod.Post, endpoint);
                request.Content = new StringContent(message.ToJsonString(), Encoding.UTF8);
                request.Content.Headers.ContentType = new MediaTypeHeaderValue("application/json");
                request.Headers.TryAddWithoutValidation("Accept", "application/json, text/event-stream");
                request.Headers.TryAddWithoutValidation("MCP-Protocol-Version", "2024-11-05");
                if (sessionId != null)
                {
                    request.Headers.TryAddWithoutValidation("Mcp-Session-Id", sessionId);
                }

                using HttpResponseMessage response = client.SendAsync(request).GetAwaiter().GetResult();
                if (response.Headers.TryGetValues("Mcp-Session-Id", out IEnumerable<string>? values)
                    && values != null)
                {
                    foreach (string value in values)
                    {
                        sessionId = value;
                    }
                }

                return response.Content.ReadAsStringAsync().GetAwaiter().GetResult();
            }

            private static JsonNode ParseResponse(string body)
            {
                string text = body;
                bool isEventStream = false;
                foreach (string raw in body.Split('\n'))
                {
                    if (raw.StartsWith("data:", StringComparison.Ordinal))
                    {
                        isEventStream = true;
                        break;
                    }
                }

                if (isEventStream)
                {
                    StringBuilder data = new StringBuilder();
                    foreach (string raw in body.Split('\n'))
                    {
                        string line = raw.TrimEnd('\r');
                        if (!line.StartsWith("data:", StringComparison.Ordinal))
                        {
                            continue;
                        }

                        if (data.Length > 0)
                        {
                            data.Append('\n');
                        }

                        data.Append(line.Substring(5).TrimStart());
                    }

                    text = data.ToString();
                }

                return JsonNode.Parse(text)
                    ?? throw new InvalidOperationException("the editor sent an empty response");
            }
        }

        private static int FreeLoopbackPort()
        {
            using TcpListener listener = new TcpListener(IPAddress.Loopback, 0);
            listener.Start();
            int port = ((IPEndPoint)listener.LocalEndpoint).Port;
            listener.Stop();
            return port;
        }
    }
}
