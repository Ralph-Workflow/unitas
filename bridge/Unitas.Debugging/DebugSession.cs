using System;
using System.Threading;

namespace Unitas.Debugging
{
    public enum DebugStepMode
    {
        None = 0,
        StepIn = 1,
        StepOver = 2,
        StepOut = 3
    }

    public sealed class DebugSession : IDisposable
    {
        private readonly DapServer? _server;
        private readonly Action<DebugFrame>? _stoppedCallback;
        private readonly SemaphoreSlim _resume = new SemaphoreSlim(0, 1);
        private readonly object _stateGate = new object();
        private bool _pauseRequested;
        private DebugStepMode _stepMode;
        private int _stepOriginDepth;

        internal DebugSession(DapServer server)
        {
            _server = server;
        }

        internal DebugSession(Action<DebugFrame> stoppedCallback)
        {
            _stoppedCallback = stoppedCallback;
            IsAttached = true;
        }

        public DapServer? Server => _server;

        public DebugFrame? CurrentFrame { get; private set; }

        public bool IsLaunched { get; private set; }

        public bool IsAttached { get; private set; }

        internal void MarkLaunched()
        {
            IsLaunched = true;
            IsAttached = true;
        }

        internal void MarkAttached()
        {
            IsAttached = true;
        }

        internal void RequestPause()
        {
            lock (_stateGate)
            {
                if (CurrentFrame == null)
                {
                    _pauseRequested = true;
                }
            }
        }

        internal void ArmStep(DebugStepMode mode)
        {
            lock (_stateGate)
            {
                DebugFrame? frame = CurrentFrame;
                _stepOriginDepth = frame != null ? frame.Depth : 0;
                _stepMode = mode;
            }
            Resume();
        }

        internal bool TryConsumeStepOrPause(int depth, out string reason)
        {
            lock (_stateGate)
            {
                if (_pauseRequested)
                {
                    _pauseRequested = false;
                    _stepMode = DebugStepMode.None;
                    reason = "pause";
                    return true;
                }
                switch (_stepMode)
                {
                    case DebugStepMode.StepIn:
                        _stepMode = DebugStepMode.None;
                        reason = "step";
                        return true;
                    case DebugStepMode.StepOver:
                        if (depth <= _stepOriginDepth)
                        {
                            _stepMode = DebugStepMode.None;
                            reason = "step";
                            return true;
                        }
                        break;
                    case DebugStepMode.StepOut:
                        if (_stepOriginDepth <= 0 || depth < _stepOriginDepth)
                        {
                            _stepMode = DebugStepMode.None;
                            reason = "step";
                            return true;
                        }
                        break;
                }
            }
            reason = string.Empty;
            return false;
        }

        internal void ClearStepAndPause()
        {
            lock (_stateGate)
            {
                _pauseRequested = false;
                _stepMode = DebugStepMode.None;
            }
        }

        internal void Stop(DebugFrame frame)
        {
            Stop(frame, "breakpoint");
        }

        internal void Stop(DebugFrame frame, string reason)
        {
            CurrentFrame = frame;
            _server?.NotifyStopped(frame, reason);
            _stoppedCallback?.Invoke(frame);
            _resume.Wait(TimeSpan.FromSeconds(30));
            CurrentFrame = null;
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
            _server?.Stop();
            _resume.Dispose();
        }
    }
}
