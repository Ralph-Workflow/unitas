using System;

namespace Unitas.Editor.Mcp
{
    public sealed class EditorRuntime : IDisposable
    {
        private readonly PlaySessionManager playSessions;

        public EditorRuntime(McpHost host, PlaySessionManager playSessions)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(playSessions);
            Host = host;
            this.playSessions = playSessions;
        }

        public McpHost Host { get; }

        public void Dispose()
        {
            playSessions.Dispose();
        }
    }
}
