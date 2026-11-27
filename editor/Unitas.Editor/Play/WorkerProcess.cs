using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Unitas.Debugging;

namespace Unitas.Editor.Mcp
{
    public interface IPlayWorker : IDisposable
    {
        bool HasExited { get; }

        int? ExitCode { get; }

        int? ProcessId { get; }

        JsonObject SendCommand(JsonObject command);

        void Kill();
    }

    public delegate IPlayWorker PlayWorkerFactory(string? bundleDirectory);

    /// <summary>
    /// A worker factory that also receives the loopback port the worker exposes its
    /// debug adapter on. The port is null when the session runs without debugging.
    /// </summary>
    public delegate IPlayWorker PlayWorkerFactoryWithDebugPort(string? bundleDirectory, int? dapPort);

    public sealed class WorkerProcess : IPlayWorker
    {
        public const string PlayerBinaryEnvVar = "UNITAS_PLAYER_BINARY";
        public const string BundleDirectoryEnvVar = "UNITAS_PLAYER_BUNDLE_DIR";

        // A worker command that does not answer in this window means the
        // worker is wedged; the session ends with a report instead of the
        // editor waiting on a line that never comes.
        public static readonly TimeSpan DefaultCommandTimeout = TimeSpan.FromSeconds(30);

        private readonly Process process;
        private readonly TimeSpan commandTimeout;
        private readonly object syncLock = new object();
        private bool disposed;
        private bool exitedObserved;
        private int? observedExitCode;

        public WorkerProcess(
            string executablePath,
            IReadOnlyList<string> arguments,
            string? workingDirectory = null,
            TimeSpan? commandTimeout = null,
            IReadOnlyDictionary<string, string>? environment = null)
        {
            if (string.IsNullOrWhiteSpace(executablePath))
            {
                throw new ArgumentException("Worker executable path must not be empty.", nameof(executablePath));
            }

            ArgumentNullException.ThrowIfNull(arguments);

            this.commandTimeout = commandTimeout ?? DefaultCommandTimeout;

            ProcessStartInfo startInfo = new ProcessStartInfo
            {
                FileName = executablePath,
                UseShellExecute = false,
                RedirectStandardInput = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true,
                StandardInputEncoding = new UTF8Encoding(false),
                StandardOutputEncoding = new UTF8Encoding(false),
                StandardErrorEncoding = new UTF8Encoding(false),
            };

            if (!string.IsNullOrWhiteSpace(workingDirectory))
            {
                startInfo.WorkingDirectory = workingDirectory;
            }

            foreach (string argument in arguments)
            {
                startInfo.ArgumentList.Add(argument);
            }

            if (environment != null)
            {
                foreach (KeyValuePair<string, string> entry in environment)
                {
                    startInfo.Environment[entry.Key] = entry.Value;
                }
            }

            Process started = new Process
            {
                StartInfo = startInfo,
            };

            try
            {
                if (!started.Start())
                {
                    started.Dispose();
                    throw new McpError.InvalidRequest("failed to start play worker process");
                }
            }
            catch (Exception error) when (!(error is McpError))
            {
                started.Dispose();
                throw new McpError.Io(error);
            }

            process = started;
        }

        public bool HasExited
        {
            get
            {
                lock (syncLock)
                {
                    return RefreshExitedLocked();
                }
            }
        }

        public int? ExitCode
        {
            get
            {
                lock (syncLock)
                {
                    RefreshExitedLocked();
                    return observedExitCode;
                }
            }
        }

        public int? ProcessId
        {
            get
            {
                lock (syncLock)
                {
                    if (disposed)
                    {
                        return null;
                    }

                    try
                    {
                        return process.Id;
                    }
                    catch (InvalidOperationException)
                    {
                        return null;
                    }
                }
            }
        }

        public static WorkerProcess StartDefault(string? bundleDirectory = null, int? dapPort = null)
        {
            string executable = ResolvePlayerBinaryPath();
            List<string> arguments = new List<string>();
            string? bundle = ResolveBundleDirectory(bundleDirectory);
            if (!string.IsNullOrWhiteSpace(bundle))
            {
                arguments.Add(bundle!);
            }

            arguments.Add("--control-stdio");
            IReadOnlyDictionary<string, string>? environment = null;
            if (dapPort.HasValue)
            {
                environment = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    [DebugAgent.DapPortEnvironmentVariable] = dapPort.Value.ToString(CultureInfo.InvariantCulture),
                };
            }

            return new WorkerProcess(executable, arguments, FindRepositoryRoot(), null, environment);
        }

        /// <summary>
        /// A free loopback port the worker can bind for its debug adapter. The port
        /// is released again, so the worker claims it when it starts.
        /// </summary>
        public static int PickFreeLoopbackPort()
        {
            TcpListener listener = DebugAgent.BeginLoopbackListener(IPAddress.Loopback, 0);
            try
            {
                return ((IPEndPoint)listener.LocalEndpoint).Port;
            }
            finally
            {
                listener.Stop();
            }
        }

        /// The bundle a play worker runs against. A caller-supplied bundle wins;
        /// otherwise the staged bundle the environment points at is reused, so a
        /// worker start does not re-stage the project for every play session.
        private static string? ResolveBundleDirectory(string? requested)
        {
            if (!string.IsNullOrWhiteSpace(requested))
            {
                return requested;
            }

            string? configured = Environment.GetEnvironmentVariable(BundleDirectoryEnvVar);
            if (!string.IsNullOrWhiteSpace(configured) && Directory.Exists(configured))
            {
                return configured;
            }

            return null;
        }

        public static string ResolvePlayerBinaryPath()
        {
            string? configured = Environment.GetEnvironmentVariable(PlayerBinaryEnvVar);
            if (!string.IsNullOrWhiteSpace(configured) && File.Exists(configured))
            {
                return Path.GetFullPath(configured);
            }

            string? repoRoot = FindRepositoryRoot();
            if (repoRoot != null)
            {
                string debugBinary = Path.Combine(repoRoot, "target", "debug", BinaryFileName());
                if (File.Exists(debugBinary))
                {
                    return debugBinary;
                }

                string releaseBinary = Path.Combine(repoRoot, "target", "release", BinaryFileName());
                if (File.Exists(releaseBinary))
                {
                    return releaseBinary;
                }
            }

            throw new McpError.InvalidRequest(
                "unitas-player binary was not found; set " + PlayerBinaryEnvVar + " or build target/debug/unitas-player");
        }

        public static string? FindRepositoryRoot()
        {
            string? current = AppContext.BaseDirectory;
            for (int depth = 0; depth < 12 && !string.IsNullOrEmpty(current); depth++)
            {
                if (File.Exists(Path.Combine(current, "Cargo.toml"))
                    && Directory.Exists(Path.Combine(current, "crates")))
                {
                    return current;
                }

                DirectoryInfo? parent = Directory.GetParent(current);
                current = parent?.FullName;
            }

            string cwd = Directory.GetCurrentDirectory();
            if (File.Exists(Path.Combine(cwd, "Cargo.toml"))
                && Directory.Exists(Path.Combine(cwd, "crates")))
            {
                return cwd;
            }

            return null;
        }

        public JsonObject SendCommand(JsonObject command)
        {
            ArgumentNullException.ThrowIfNull(command);

            lock (syncLock)
            {
                if (disposed || RefreshExitedLocked())
                {
                    throw SessionEndedErrorLocked();
                }

                string requestLine = command.ToJsonString();
                string? responseLine;
                try
                {
                    process.StandardInput.WriteLine(requestLine);
                    process.StandardInput.Flush();
                    responseLine = ReadResponseLineLocked();
                }
                catch (IOException)
                {
                    RefreshExitedLocked();
                    exitedObserved = true;
                    throw SessionEndedErrorLocked();
                }
                catch (ObjectDisposedException)
                {
                    exitedObserved = true;
                    throw SessionEndedErrorLocked();
                }
                catch (InvalidOperationException)
                {
                    exitedObserved = true;
                    throw SessionEndedErrorLocked();
                }

                if (responseLine == null)
                {
                    try
                    {
                        process.WaitForExit(200);
                    }
                    catch (InvalidOperationException)
                    {
                    }

                    RefreshExitedLocked();
                    exitedObserved = true;
                    throw SessionEndedErrorLocked();
                }

                JsonNode? parsed;
                try
                {
                    parsed = JsonNode.Parse(responseLine);
                }
                catch (Exception error)
                {
                    throw new McpError.ParseError("invalid worker response JSON: " + error.Message);
                }

                if (!(parsed is JsonObject responseObject))
                {
                    throw new McpError.ParseError("worker response must be a JSON object");
                }

                bool ok = false;
                if (responseObject["ok"] is JsonValue okValue && okValue.TryGetValue(out bool parsedOk))
                {
                    ok = parsedOk;
                }

                if (!ok)
                {
                    string errorMessage = "unknown worker error";
                    if (responseObject["error"] is JsonValue errorValue
                        && errorValue.TryGetValue(out string? parsedError)
                        && !string.IsNullOrEmpty(parsedError))
                    {
                        errorMessage = parsedError!;
                    }

                    throw new McpError.InvalidRequest("worker command failed: " + errorMessage);
                }

                return responseObject;
            }
        }

        private string? ReadResponseLineLocked()
        {
            Task<string?> readTask = process.StandardOutput.ReadLineAsync();
            using CancellationTokenSource timeout = new CancellationTokenSource();
            Task delay = Task.Delay(commandTimeout, timeout.Token);
            Task finished = Task.WhenAny(readTask, delay).GetAwaiter().GetResult();
            timeout.Cancel();
            if (finished != readTask)
            {
                // The worker is not answering; the session cannot continue on a
                // stream that may still deliver a late line.
                _ = readTask.ContinueWith(static task => _ = task.Exception, TaskScheduler.Default);
                Kill();
                throw new McpError.InvalidRequest(
                    "play session ended: worker did not answer within "
                    + commandTimeout.TotalSeconds.ToString("0.###", CultureInfo.InvariantCulture)
                    + " seconds");
            }

            return readTask.GetAwaiter().GetResult();
        }

        public void Kill()
        {
            lock (syncLock)
            {
                if (disposed)
                {
                    return;
                }

                try
                {
                    if (!process.HasExited)
                    {
                        process.Kill(true);
                        process.WaitForExit(1000);
                    }
                }
                catch (InvalidOperationException)
                {
                }
                catch (NotSupportedException)
                {
                }

                RefreshExitedLocked();
                exitedObserved = true;
            }
        }

        public void Dispose()
        {
            lock (syncLock)
            {
                if (disposed)
                {
                    return;
                }

                try
                {
                    if (!process.HasExited)
                    {
                        try
                        {
                            process.StandardInput.Close();
                        }
                        catch (IOException)
                        {
                        }
                        catch (ObjectDisposedException)
                        {
                        }
                        catch (InvalidOperationException)
                        {
                        }

                        if (!process.WaitForExit(200))
                        {
                            process.Kill(true);
                            process.WaitForExit(1000);
                        }
                    }
                }
                catch (InvalidOperationException)
                {
                }
                catch (NotSupportedException)
                {
                }

                RefreshExitedLocked();
                exitedObserved = true;
                disposed = true;
                process.Dispose();
            }
        }

        private static string BinaryFileName()
        {
            return OperatingSystem.IsWindows() ? "unitas-player.exe" : "unitas-player";
        }

        private bool RefreshExitedLocked()
        {
            if (exitedObserved)
            {
                return true;
            }

            if (disposed)
            {
                exitedObserved = true;
                return true;
            }

            try
            {
                if (process.HasExited)
                {
                    exitedObserved = true;
                    observedExitCode = process.ExitCode;
                    return true;
                }
            }
            catch (InvalidOperationException)
            {
                exitedObserved = true;
                return true;
            }

            return false;
        }

        private McpError.InvalidRequest SessionEndedErrorLocked()
        {
            string suffix = observedExitCode.HasValue
                ? (" (exit code " + observedExitCode.Value + ")")
                : string.Empty;
            return new McpError.InvalidRequest("play session ended: worker process exited" + suffix);
        }
    }
}
