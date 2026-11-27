using System;
using System.Globalization;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using Unitas.Editor.Mcp;
using Unitas.Editor.Shell;

namespace Unitas.Editor
{
    public static class Program
    {
        public static async Task<int> Main(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                using EditorRuntime runtime = BuiltinDefaults.CreateStartupRuntime();
                await new StdioTransportHost(runtime.Host).RunConsoleAsync().ConfigureAwait(false);
                return 0;
            }

            if (CliArgs.IsShellInvocation(args))
            {
                return RunShellMode(args);
            }

            if (HeadlessCli.IsCliInvocation(args))
            {
                using EditorRuntime runtime = BuiltinDefaults.CreateStartupRuntime(args);
                return HeadlessCli.Run(runtime.Host, args);
            }

            if (args.Length == 1 && string.Equals(args[0], "--stdio", StringComparison.Ordinal))
            {
                using EditorRuntime runtime = BuiltinDefaults.CreateStartupRuntime();
                await new StdioTransportHost(runtime.Host).RunConsoleAsync().ConfigureAwait(false);
                return 0;
            }

            if (string.Equals(args[0], "--http", StringComparison.Ordinal))
            {
                int port = 0;
                if (args.Length == 2)
                {
                    if (!int.TryParse(
                        args[1],
                        NumberStyles.Integer,
                        CultureInfo.InvariantCulture,
                        out port)
                        || port < 0
                        || port > 65535)
                    {
                        return 2;
                    }
                }
                else if (args.Length > 2)
                {
                    return 2;
                }

                using EditorRuntime runtime = BuiltinDefaults.CreateStartupRuntime(args);
                HttpTransportHost transportHost = new HttpTransportHost(runtime.Host, port);
                try
                {
                    await transportHost.RunAsync().ConfigureAwait(false);
                }
                finally
                {
                    await transportHost.DisposeAsync().ConfigureAwait(false);
                }

                return 0;
            }

            return 2;
        }

        private static int RunShellMode(string[] args)
        {
            if (!TryParseShellOptions(args, out ShellRunOptions options))
            {
                return 2;
            }

            using EditorRuntime runtime = BuiltinDefaults.CreateStartupRuntime(args);
            try
            {
                ShellHost shellHost = new ShellHost(runtime.Host);
                JsonObject result = shellHost.Run(options);
                Console.Out.WriteLine(result.ToJsonString());
                return 0;
            }
            catch (McpError error)
            {
                Console.Error.WriteLine("[unitas-shell] " + error.Message);
                return 1;
            }
        }

        private static bool TryParseShellOptions(string[] args, out ShellRunOptions options)
        {
            options = new ShellRunOptions();
            int index = 0;
            while (index < args.Length)
            {
                string current = args[index] ?? string.Empty;
                if (string.Equals(current, CliArgs.ShellFlag, StringComparison.Ordinal))
                {
                    index++;
                }
                else if (string.Equals(current, CliArgs.HeadlessFlag, StringComparison.Ordinal))
                {
                    options.Headless = true;
                    index++;
                }
                else if (!TryConsumeShellOptionValue(args, ref index, current, options))
                {
                    return false;
                }
            }

            return true;
        }

        private static bool TryConsumeShellOptionValue(
            string[] args,
            ref int index,
            string current,
            ShellRunOptions options)
        {
            if (string.Equals(current, CliArgs.ProjectOption, StringComparison.Ordinal)
                || string.Equals(current, CliArgs.ProjectPathOption, StringComparison.Ordinal))
            {
                if (index + 1 >= args.Length)
                {
                    return false;
                }

                options.TargetProjectPath = args[index + 1];
                index += 2;
                return true;
            }

            if (string.Equals(current, CliArgs.ShellProjectOption, StringComparison.Ordinal))
            {
                if (index + 1 >= args.Length)
                {
                    return false;
                }

                options.ShellProjectPath = args[index + 1];
                index += 2;
                return true;
            }

            if (string.Equals(current, CliArgs.CaptureOutOption, StringComparison.Ordinal))
            {
                if (index + 1 >= args.Length)
                {
                    return false;
                }

                options.CaptureOut = args[index + 1];
                index += 2;
                return true;
            }

            if (string.Equals(current, CliArgs.FramesOption, StringComparison.Ordinal))
            {
                if (index + 1 >= args.Length
                    || !long.TryParse(
                        args[index + 1],
                        NumberStyles.Integer,
                        CultureInfo.InvariantCulture,
                        out long frames)
                    || frames <= 0)
                {
                    return false;
                }

                options.MaxFrames = frames;
                index += 2;
                return true;
            }

            if (!current.StartsWith('-') && string.IsNullOrEmpty(options.TargetProjectPath))
            {
                options.TargetProjectPath = current;
                index++;
                return true;
            }

            return false;
        }
    }
}
