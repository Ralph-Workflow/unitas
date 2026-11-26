using System;

namespace UnityEngine.Jobs
{
    public interface IJobHost
    {
        ulong ScheduleParallel(IntPtr callback, IntPtr context, long length, long batch, ulong dependency);

        ulong Schedule(IntPtr callback, IntPtr context, ulong dependency);

        void Complete(ulong handle);

        bool IsComplete(ulong handle);
    }

    public static class JobHost
    {
        public static IJobHost? Current { get; set; }

        internal static IJobHost Require()
        {
            IJobHost? host = Current;
            if (host == null)
            {
                throw new InvalidOperationException(
                    "No job host is installed; the native job system is unavailable.");
            }

            return host;
        }
    }
}
