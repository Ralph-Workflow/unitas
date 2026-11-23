using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static class RuntimeInterop
    {
        public const string LibraryName = "unitas_runtime";

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateScriptCallback(ulong entity, ulong guidHi, ulong guidLo);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void AwakeCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void OnEnableCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void StartCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FixedUpdateCallback(ulong scriptHandle, double fixedDeltaSeconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void UpdateCallback(ulong scriptHandle, double deltaSeconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void LateUpdateCallback(ulong scriptHandle, double deltaSeconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void OnDisableCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DestroyCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void CollisionCallback(ulong entityA, ulong entityB, uint phase);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TriggerCallback(ulong entityA, ulong entityB, uint phase);

        static RuntimeInterop()
        {
            NativeLibrary.SetDllImportResolver(
                typeof(RuntimeInterop).Assembly,
                (libraryName, assembly, searchPath) =>
                {
                    if (libraryName == LibraryName)
                    {
                        string? envPath = Environment.GetEnvironmentVariable("UNITAS_RUNTIME_LIBRARY");
                        if (!string.IsNullOrEmpty(envPath) && NativeLibrary.TryLoad(envPath, out IntPtr handle))
                        {
                            return handle;
                        }
                    }
                    return IntPtr.Zero;
                });
        }

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong unitas_runtime_new();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_path_buffer(ulong runtime, ulong capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_load_scene(ulong runtime, IntPtr address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_callbacks(
            ulong runtime,
            CreateScriptCallback createScript,
            AwakeCallback awake,
            OnEnableCallback onEnable,
            StartCallback start,
            FixedUpdateCallback fixedUpdate,
            UpdateCallback update,
            LateUpdateCallback lateUpdate,
            OnDisableCallback onDisable,
            DestroyCallback destroy,
            CollisionCallback collision,
            TriggerCallback trigger);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_tick(ulong runtime, double deltaSeconds);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_destroy(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_error_message(ulong runtime);
    }
}
