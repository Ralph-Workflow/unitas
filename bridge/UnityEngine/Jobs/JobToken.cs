using System;
using System.Threading;
using UnityEngine.Collections;

namespace UnityEngine.Jobs
{
    internal sealed class JobToken : INativeArrayDependency
    {
        private ulong _handle;
        private bool _pending;
        private Exception? _failure;
        private JobRunner? _runner;

        internal JobToken(JobToken? parent)
        {
            _pending = true;
            Parent = parent;
        }

        internal ulong Handle => _handle;

        public INativeArrayDependency? Parent { get; }

        public bool IsCompleted => !_pending && (_handle == 0 || JobHost.Require().IsComplete(_handle));

        internal void AttachRunner(JobRunner runner)
        {
            _runner = runner;
        }

        internal void SetHandle(ulong handle)
        {
            _handle = handle;
            _pending = false;
        }

        internal void RecordFailure(Exception error)
        {
            Volatile.Write(ref _failure, error);
        }

        public void Complete()
        {
            if (_handle != 0)
            {
                JobHost.Require().Complete(_handle);
            }

            JobRunner? runner = _runner;
            if (runner != null)
            {
                _runner = null;
                runner.Release();
            }

            Exception? failure = Volatile.Read(ref _failure);
            if (failure != null)
            {
                throw new InvalidOperationException("The scheduled job failed.", failure);
            }
        }
    }
}
