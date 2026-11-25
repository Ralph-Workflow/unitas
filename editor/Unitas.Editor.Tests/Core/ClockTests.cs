using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class ClockTests
    {
        public static void Run()
        {
            FixedClockIsDeterministic();
            SystemClockReportsAPositiveTimestamp();
        }

        private static void FixedClockIsDeterministic()
        {
            FixedClock clock = new FixedClock(1_700_000_000_000);
            Check.That("fixed_clock_is_deterministic: initial", clock.NowMs() == 1_700_000_000_000);
            clock.Advance(25);
            Check.That("fixed_clock_is_deterministic: advanced", clock.NowMs() == 1_700_000_000_025);
        }

        private static void SystemClockReportsAPositiveTimestamp()
        {
            Check.That("system_clock_reports_a_positive_timestamp", new SystemClock().NowMs() > 0);
        }
    }
}
