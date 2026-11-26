namespace UnityEngine.Jobs
{
    public struct JobHandle
    {
        private readonly JobToken? _token;
        private readonly JobHandle[]? _parts;

        internal JobHandle(JobToken token)
        {
            _token = token;
            _parts = null;
        }

        private JobHandle(JobHandle[] parts)
        {
            _token = null;
            _parts = parts;
        }

        public bool IsCompleted
        {
            get
            {
                if (_parts != null)
                {
                    for (int index = 0; index < _parts.Length; index++)
                    {
                        if (!_parts[index].IsCompleted)
                        {
                            return false;
                        }
                    }

                    return true;
                }

                return _token == null || _token.IsCompleted;
            }
        }

        public void Complete()
        {
            if (_parts != null)
            {
                for (int index = 0; index < _parts.Length; index++)
                {
                    _parts[index].Complete();
                }

                return;
            }

            _token?.Complete();
        }

        public static JobHandle CombineDependencies(JobHandle a, JobHandle b)
        {
            return new JobHandle(new[] { a, b });
        }

        internal bool TryGetNativeHandle(out ulong handle)
        {
            handle = 0;
            if (_parts != null || _token == null)
            {
                return false;
            }

            handle = _token.Handle;
            return handle != 0;
        }

        internal JobToken? Token => _token;
    }
}
