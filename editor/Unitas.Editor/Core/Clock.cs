using System;

namespace Unitas.Editor.Mcp
{
    public interface IClock
    {
        long NowMs();
    }

    public sealed class SystemClock : IClock
    {
        public long NowMs()
        {
            return DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        }
    }

    public sealed class FixedClock : IClock
    {
        private long _nowMs;

        public FixedClock(long nowMs)
        {
            _nowMs = nowMs;
        }

        public void Advance(long deltaMs)
        {
            _nowMs += deltaMs;
        }

        public long NowMs()
        {
            return _nowMs;
        }
    }
}
