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
            // temporary harness until the player runs the editor project; piped stdin must remain open until responses arrive; must not ship
            if (args == null || args.Length == 0)
            {
                await StdioTransportHost.WithDefaults().RunConsoleAsync().ConfigureAwait(false);
                return 0;
            }

            if (HeadlessCli.IsCliInvocation(args))
            {
                using McpHost cliHost = BuiltinDefaults.CreateStartupHost(args);
                try
                {
                    return HeadlessCli.Run(cliHost, args);
                }
                finally
                {
                    cliHost.DisposeOwnedResources();
                }
            }

            if (args.Length == 1 && string.Equals(args[0], "--stdio", StringComparison.Ordinal))
            {
                await StdioTransportHost.WithDefaults().RunConsoleAsync().ConfigureAwait(false);
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

                McpHost host = BuiltinDefaults.CreateStartupHost(args);
                HttpTransportHost transportHost = new HttpTransportHost(host, port);
                try
                {
                    await transportHost.RunAsync().ConfigureAwait(false);
                }
                finally
                {
                    await transportHost.DisposeAsync().ConfigureAwait(false);
                    host.Dispose();
                }

                return 0;
            }

            return 2;
        }
    }
}
