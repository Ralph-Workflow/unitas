using System;
using System.Threading;

namespace Unitas.Debugging
{
    public sealed class DebugSession : IDisposable
    {
        private readonly DapServer _server;
        private readonly SemaphoreSlim _resume = new SemaphoreSlim(0, 1);

        internal DebugSession(DapServer server)
        {
            _server = server;
        }

        public DapServer Server => _server;

        public DebugFrame? CurrentFrame { get; private set; }

        internal void Stop(DebugFrame frame)
        {
            CurrentFrame = frame;
            _server.NotifyStopped(frame);
            _resume.Wait(TimeSpan.FromSeconds(30));
        }

        internal void Resume()
        {
            if (_resume.CurrentCount == 0)
            {
                _resume.Release();
            }
        }

        public void Dispose()
        {
            DebugAgent.ReleaseSession(this);
            Resume();
            _server.Stop();
        }
    }
}
