using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging.Abstractions;
using ModelContextProtocol;
using ModelContextProtocol.Client;
using ModelContextProtocol.Protocol;

namespace Unitas.Benchmark
{
    public sealed record ToolCallOutcome(bool Ok, string? Error, JsonNode? Result);

    public sealed class McpSession : IAsyncDisposable
    {
        public const string PersonCredentialEnvironmentVariable = "UNITAS_PERSON_CREDENTIAL";
        public const string PersonCredentialPlaceholder = "$UNITAS_PERSON_CREDENTIAL";
        public const string AgentHeaderName = "X-Unitas-Agent";
        public const string AuthorizedByHeaderName = "X-Unitas-Authorized-By";
        public const string ClientName = "unitas-benchmark";

        private static readonly TimeSpan StartupTimeout = TimeSpan.FromSeconds(15);

        private readonly Process process;
        private readonly HttpClientTransport transport;
        private readonly McpClient client;
        private readonly string? personCredential;

        private McpSession(
            Process process,
            HttpClientTransport transport,
            McpClient client,
            string? personCredential)
        {
            this.process = process;
            this.transport = transport;
            this.client = client;
            this.personCredential = personCredential;
        }

        public static async Task<McpSession> StartAsync(
            string editorPath,
            string workingDirectory,
            string? personCredential,
            CancellationToken cancellationToken)
        {
            int port = FreeLoopbackPort();
            ProcessStartInfo startInfo = new ProcessStartInfo
            {
                FileName = ResolveDotnetHost(),
                WorkingDirectory = workingDirectory,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            startInfo.ArgumentList.Add(editorPath);
            startInfo.ArgumentList.Add("--http");
            startInfo.ArgumentList.Add(port.ToString(CultureInfo.InvariantCulture));
            if (!string.IsNullOrEmpty(personCredential))
            {
                startInfo.Environment[PersonCredentialEnvironmentVariable] = personCredential;
            }

            Process started = Process.Start(startInfo)
                ?? throw new InvalidOperationException("the editor process did not start");
            started.EnableRaisingEvents = true;
            started.OutputDataReceived += (_, _) => { };
            started.ErrorDataReceived += (_, _) => { };
            started.BeginOutputReadLine();
            started.BeginErrorReadLine();

            Uri endpoint = new Uri("http://127.0.0.1:" + port.ToString(CultureInfo.InvariantCulture) + "/");
            Stopwatch watch = Stopwatch.StartNew();
            while (true)
            {
                HttpClientTransportOptions transportOptions = new HttpClientTransportOptions
                {
                    Endpoint = endpoint,
                    Name = ClientName,
                    AdditionalHeaders = new Dictionary<string, string>
                    {
                        [AgentHeaderName] = ClientName,
                        [AuthorizedByHeaderName] = "person-local",
                    },
                };
                HttpClientTransport clientTransport = new HttpClientTransport(
                    transportOptions,
                    NullLoggerFactory.Instance);
                try
                {
                    McpClientOptions clientOptions = new McpClientOptions
                    {
                        ClientInfo = new Implementation
                        {
                            Name = ClientName,
                            Version = "1.0.0",
                        },
                    };
                    McpClient connected = await McpClient.CreateAsync(
                        clientTransport,
                        clientOptions,
                        NullLoggerFactory.Instance,
                        cancellationToken).ConfigureAwait(false);
                    return new McpSession(started, clientTransport, connected, personCredential);
                }
                catch (Exception) when (!cancellationToken.IsCancellationRequested && watch.Elapsed < StartupTimeout)
                {
                    await clientTransport.DisposeAsync().ConfigureAwait(false);
                    await Task.Delay(100, cancellationToken).ConfigureAwait(false);
                }
                catch
                {
                    await clientTransport.DisposeAsync().ConfigureAwait(false);
                    StopProcess(started);
                    throw;
                }
            }
        }

        public async Task<IReadOnlyList<string>> ListToolNamesAsync(CancellationToken cancellationToken)
        {
            IList<McpClientTool> tools = await client.ListToolsAsync(
                cancellationToken: cancellationToken).ConfigureAwait(false);
            List<string> names = new List<string>();
            foreach (McpClientTool tool in tools)
            {
                names.Add(tool.Name);
            }

            return names;
        }

        public async Task<ToolCallOutcome> CallAsync(
            string tool,
            JsonObject arguments,
            CancellationToken cancellationToken)
        {
            try
            {
                CallToolResult result = await client.CallToolAsync(
                    new CallToolRequestParams
                    {
                        Name = tool,
                        Arguments = ToArguments(arguments, personCredential),
                    },
                    cancellationToken).ConfigureAwait(false);
                JsonNode? payload = ToPayload(result);
                if (result.IsError == true)
                {
                    return new ToolCallOutcome(false, "the editor reported a tool error", payload);
                }

                return new ToolCallOutcome(true, null, payload);
            }
            catch (Exception error) when (error is McpException
                || error is JsonException
                || error is InvalidOperationException
                || error is IOException)
            {
                return new ToolCallOutcome(false, error.Message, null);
            }
        }

        public async ValueTask DisposeAsync()
        {
            try
            {
                await client.DisposeAsync().ConfigureAwait(false);
            }
            finally
            {
                try
                {
                    await transport.DisposeAsync().ConfigureAwait(false);
                }
                finally
                {
                    StopProcess(process);
                }
            }
        }

        private static Dictionary<string, JsonElement> ToArguments(
            JsonObject arguments,
            string? personCredential)
        {
            Dictionary<string, JsonElement> converted = new Dictionary<string, JsonElement>();
            foreach (KeyValuePair<string, JsonNode?> pair in arguments)
            {
                JsonNode? value = pair.Value;
                if (string.Equals(pair.Key, "personCredential", StringComparison.OrdinalIgnoreCase)
                    && !string.IsNullOrEmpty(personCredential)
                    && (value == null
                        || (value is JsonValue textValue
                            && textValue.TryGetValue(out string? text)
                            && string.Equals(text, PersonCredentialPlaceholder, StringComparison.Ordinal))))
                {
                    value = JsonValue.Create(personCredential);
                }

                string json = value == null ? "null" : value.ToJsonString();
                converted[pair.Key] = JsonSerializer.Deserialize<JsonElement>(json);
            }

            return converted;
        }

        private static JsonNode? ToPayload(CallToolResult result)
        {
            if (result.StructuredContent is JsonElement structured)
            {
                return JsonNode.Parse(structured.GetRawText());
            }

            foreach (ContentBlock block in result.Content)
            {
                if (block is TextContentBlock text)
                {
                    try
                    {
                        return JsonNode.Parse(text.Text);
                    }
                    catch (JsonException)
                    {
                        return JsonValue.Create(text.Text);
                    }
                }
            }

            return null;
        }

        private static int FreeLoopbackPort()
        {
            using TcpListener listener = new TcpListener(IPAddress.Loopback, 0);
            listener.Start();
            return ((IPEndPoint)listener.LocalEndpoint).Port;
        }

        private static void StopProcess(Process started)
        {
            try
            {
                if (!started.HasExited)
                {
                    started.Kill(true);
                    started.WaitForExit(5000);
                }
            }
            catch (InvalidOperationException)
            {
            }
            finally
            {
                started.Dispose();
            }
        }

        private static string ResolveDotnetHost()
        {
            string? hostPath = Environment.GetEnvironmentVariable("DOTNET_HOST_PATH");
            if (!string.IsNullOrWhiteSpace(hostPath) && File.Exists(hostPath))
            {
                return hostPath;
            }

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
    }
}
