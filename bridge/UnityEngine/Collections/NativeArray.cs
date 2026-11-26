using System;
using System.Runtime.InteropServices;

namespace UnityEngine.Collections
{
    public sealed class NativeArray<T> : IDisposable, INativeArray where T : struct
    {
        private IntPtr _pointer;
        private readonly int _length;
        private readonly int _elementSize;
        private readonly Allocator _allocator;
        private readonly string _name;
        private bool _disposed;

        public NativeArray(int length, Allocator allocator, string? name = null)
        {
            if (length < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(length));
            }

            try
            {
                _elementSize = Marshal.SizeOf<T>();
            }
            catch (ArgumentException error)
            {
                throw new ArgumentException(
                    "NativeArray element type must be a blittable value type.", error);
            }

            _length = length;
            _allocator = allocator;
            _name = name ?? ("NativeArray<" + typeof(T).Name + ">");
            _pointer = Marshal.AllocHGlobal(Math.Max(length * _elementSize, 1));
            NativeArrayDiagnostics.Register();
        }

        ~NativeArray()
        {
            if (_disposed)
            {
                return;
            }

            NativeArrayDiagnostics.ReportLeak(_name);
            Release();
        }

        public int Length
        {
            get
            {
                ThrowIfDisposed();
                return _length;
            }
        }

        public string Name => _name;

        public Allocator Allocator => _allocator;

        public bool IsCreated => !_disposed && _pointer != IntPtr.Zero;

        public T this[int index]
        {
            get
            {
                ThrowIfDisposed();
                NativeArraySafety.ThrowIfInUse(this);
                return Marshal.PtrToStructure<T>(ElementPointer(index));
            }

            set
            {
                ThrowIfDisposed();
                NativeArraySafety.ThrowIfInUse(this);
                Marshal.StructureToPtr(value, ElementPointer(index), false);
            }
        }

        public void Dispose()
        {
            if (_disposed)
            {
                throw new InvalidOperationException(
                    "NativeArray '" + _name + "' has already been disposed.");
            }

            try
            {
                NativeArraySafety.CompletePending(this);
            }
            finally
            {
                Release();
                GC.SuppressFinalize(this);
            }
        }

        private IntPtr ElementPointer(int index)
        {
            if (index < 0 || index >= _length)
            {
                throw new ArgumentOutOfRangeException(nameof(index));
            }

            return new IntPtr(_pointer.ToInt64() + (index * _elementSize));
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
            {
                throw new ObjectDisposedException(_name);
            }
        }

        private void Release()
        {
            _disposed = true;
            NativeArraySafety.Forget(this);
            Marshal.FreeHGlobal(_pointer);
            _pointer = IntPtr.Zero;
            NativeArrayDiagnostics.Release();
        }
    }
}
