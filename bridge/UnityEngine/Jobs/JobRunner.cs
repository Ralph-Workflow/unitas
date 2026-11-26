using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using UnityEngine.Collections;

namespace UnityEngine.Jobs
{
    internal abstract class JobRunner
    {
        private static readonly object Gate = new object();

        private static readonly Dictionary<int, JobRunner> Runners = new Dictionary<int, JobRunner>();

        private static int _nextId;

        private readonly int _id;
        private readonly NativeJobCallback _callback;
        private readonly IntPtr _functionPointer;

        protected JobRunner(JobToken token)
        {
            Token = token;
            _callback = OnNativeCallback;
            _functionPointer = Marshal.GetFunctionPointerForDelegate(_callback);
            lock (Gate)
            {
                _id = ++_nextId;
                Runners.Add(_id, this);
            }
        }

        protected JobToken Token { get; }

        internal IntPtr Context => new IntPtr(_id);

        internal IntPtr FunctionPointer => _functionPointer;

        internal void Release()
        {
            lock (Gate)
            {
                Runners.Remove(_id);
            }
        }

        protected abstract void Invoke(long start, long count);

        private void OnNativeCallback(IntPtr context, long start, long count)
        {
            JobRunner? runner = Resolve(context);
            if (runner == null)
            {
                return;
            }

            NativeArraySafety.EnterJob(runner.Token);
            try
            {
                runner.Invoke(start, count);
            }
            finally
            {
                NativeArraySafety.ExitJob();
            }
        }

        private static JobRunner? Resolve(IntPtr context)
        {
            lock (Gate)
            {
                Runners.TryGetValue(context.ToInt32(), out JobRunner? runner);
                return runner;
            }
        }
    }

    internal sealed class SingleJobRunner : JobRunner
    {
        private readonly IJob _job;

        internal SingleJobRunner(IJob job, JobToken token)
            : base(token)
        {
            _job = job;
        }

        protected override void Invoke(long start, long count)
        {
            try
            {
                _job.Execute();
            }
            catch (Exception error) when (!(error is OutOfMemoryException))
            {
                Token.RecordFailure(error);
            }
        }
    }

    internal sealed class ParallelJobRunner : JobRunner
    {
        private readonly IJobParallelFor _job;

        internal ParallelJobRunner(IJobParallelFor job, JobToken token)
            : base(token)
        {
            _job = job;
        }

        protected override void Invoke(long start, long count)
        {
            try
            {
                for (long index = start; index < start + count; index++)
                {
                    _job.Execute((int)index);
                }
            }
            catch (Exception error) when (!(error is OutOfMemoryException))
            {
                Token.RecordFailure(error);
            }
        }
    }
}
