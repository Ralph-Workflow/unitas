using System;
using System.Globalization;
using System.Threading.Tasks;
using Unitas.Editor.Mcp;

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
    }
}
