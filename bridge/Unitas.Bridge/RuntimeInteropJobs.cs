using System;
using System.Runtime.InteropServices;
using UnityEngine.Jobs;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong unitas_runtime_jobs_schedule_parallel(
            IntPtr callback,
            IntPtr context,
            long length,
            long batch,
            ulong dependency);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong unitas_runtime_jobs_schedule(
            IntPtr callback,
            IntPtr context,
            ulong dependency);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void unitas_runtime_jobs_complete(ulong handle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern byte unitas_runtime_jobs_is_complete(ulong handle);

        public static void InitializeJobs()
        {
            JobHost.Current = new NativeJobHost();
        }

        private sealed class NativeJobHost : IJobHost
        {
            public ulong ScheduleParallel(IntPtr callback, IntPtr context, long length, long batch, ulong dependency)
            {
                return unitas_runtime_jobs_schedule_parallel(callback, context, length, batch, dependency);
            }

            public ulong Schedule(IntPtr callback, IntPtr context, ulong dependency)
            {
                return unitas_runtime_jobs_schedule(callback, context, dependency);
            }

            public void Complete(ulong handle)
            {
                unitas_runtime_jobs_complete(handle);
            }

            public bool IsComplete(ulong handle)
            {
                return unitas_runtime_jobs_is_complete(handle) != 0;
            }
        }
    }
}
