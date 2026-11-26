using System;
using System.Runtime.InteropServices;
using UnityEngine.Collections;

namespace UnityEngine.Jobs
{
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate void NativeJobCallback(IntPtr context, long start, long count);

    public static class Job
    {
        public static JobHandle Schedule(IJob job, INativeArray array, JobHandle dependsOn = default)
        {
            if (array == null)
            {
                throw new ArgumentNullException(nameof(array));
            }

            return Schedule(job, new INativeArray[] { array }, dependsOn);
        }

        public static JobHandle Schedule(IJob job, INativeArray[] arrays, JobHandle dependsOn = default)
        {
            if (job == null)
            {
                throw new ArgumentNullException(nameof(job));
            }

            if (arrays == null)
            {
                throw new ArgumentNullException(nameof(arrays));
            }

            IJobHost host = JobHost.Require();
            JobToken token = new JobToken(ParentToken(dependsOn));
            SingleJobRunner runner = new SingleJobRunner(job, token);
            token.AttachRunner(runner);
            ulong handle = host.Schedule(runner.FunctionPointer, runner.Context, DependencyHandle(dependsOn));
            token.SetHandle(handle);
            RegisterArrays(arrays, token);
            return new JobHandle(token);
        }

        public static JobHandle ScheduleParallelFor(
            IJobParallelFor job,
            int length,
            int batchSize,
            INativeArray array,
            JobHandle dependsOn = default)
        {
            if (array == null)
            {
                throw new ArgumentNullException(nameof(array));
            }

            return ScheduleParallelFor(job, length, batchSize, new INativeArray[] { array }, dependsOn);
        }

        public static JobHandle ScheduleParallelFor(
            IJobParallelFor job,
            int length,
            int batchSize,
            INativeArray[] arrays,
            JobHandle dependsOn = default)
        {
            if (job == null)
            {
                throw new ArgumentNullException(nameof(job));
            }

            if (length < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(length));
            }

            if (arrays == null)
            {
                throw new ArgumentNullException(nameof(arrays));
            }

            IJobHost host = JobHost.Require();
            JobToken token = new JobToken(ParentToken(dependsOn));
            ParallelJobRunner runner = new ParallelJobRunner(job, token);
            token.AttachRunner(runner);
            ulong handle = host.ScheduleParallel(
                runner.FunctionPointer,
                runner.Context,
                length,
                batchSize,
                DependencyHandle(dependsOn));
            token.SetHandle(handle);
            RegisterArrays(arrays, token);
            return new JobHandle(token);
        }

        private static JobToken? ParentToken(JobHandle dependsOn)
        {
            if (dependsOn.TryGetNativeHandle(out ulong _))
            {
                return dependsOn.Token;
            }

            return null;
        }

        private static ulong DependencyHandle(JobHandle dependsOn)
        {
            if (dependsOn.TryGetNativeHandle(out ulong handle))
            {
                return handle;
            }

            dependsOn.Complete();
            return 0;
        }

        private static void RegisterArrays(INativeArray[] arrays, JobToken token)
        {
            for (int index = 0; index < arrays.Length; index++)
            {
                INativeArray? array = arrays[index];
                if (array is null)
                {
                    continue;
                }

                NativeArraySafety.Register(array, token);
            }
        }
    }
}
