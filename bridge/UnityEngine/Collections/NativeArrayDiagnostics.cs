using System;
using System.Threading;

namespace UnityEngine.Collections
{
    public static class NativeArrayDiagnostics
    {
        private static int _live;
        private static int _leaked;

        public static int LiveAllocationCount => Volatile.Read(ref _live);

        public static int LeakedAllocationCount => Volatile.Read(ref _leaked);

        internal static void Register()
        {
            Interlocked.Increment(ref _live);
        }

        internal static void Release()
        {
            Interlocked.Decrement(ref _live);
        }

        internal static void ReportLeak(string name)
        {
            Interlocked.Increment(ref _leaked);
            Console.Error.WriteLine("Undisposed NativeArray reported at teardown: " + name);
        }
    }
}
