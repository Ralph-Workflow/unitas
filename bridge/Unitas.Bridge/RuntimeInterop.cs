using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static class RuntimeInterop
    {
        public const string LibraryName = "unitas_runtime";

        public const int QueryRecordSize = 64;

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
        public static extern int unitas_runtime_raycast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double directionX,
            double directionY,
            double directionZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_linecast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double dirX,
            double dirY,
            double dirZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_sphere_cast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double dirX,
            double dirY,
            double dirZ,
            double radius,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_box_cast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double dirX,
            double dirY,
            double dirZ,
            double halfX,
            double halfY,
            double halfZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_overlap_sphere(
            ulong runtime,
            double centerX,
            double centerY,
            double centerZ,
            double radius,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_query_buffer(ulong runtime, uint capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_destroy(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_error_message(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_fixed_timestep(ulong runtime);

        public static QueryHit[] ReadQueryHits(IntPtr buffer, int count)
        {
            if (count <= 0)
            {
                return Array.Empty<QueryHit>();
            }
            byte[] bytes = new byte[count * QueryRecordSize];
            Marshal.Copy(buffer, bytes, 0, bytes.Length);
            QueryHit[] hits = new QueryHit[count];
            for (int index = 0; index < count; index++)
            {
                int offset = index * QueryRecordSize;
                hits[index] = new QueryHit
                {
                    Entity = BitConverter.ToUInt64(bytes, offset),
                    Distance = BitConverter.ToDouble(bytes, offset + 8),
                    X = BitConverter.ToDouble(bytes, offset + 16),
                    Y = BitConverter.ToDouble(bytes, offset + 24),
                    Z = BitConverter.ToDouble(bytes, offset + 32),
                    NX = BitConverter.ToDouble(bytes, offset + 40),
                    NY = BitConverter.ToDouble(bytes, offset + 48),
                    NZ = BitConverter.ToDouble(bytes, offset + 56),
                };
            }
            return hits;
        }
    }

    public struct QueryHit
    {
        public ulong Entity;
        public double Distance;
        public double X;
        public double Y;
        public double Z;
        public double NX;
        public double NY;
        public double NZ;
    }
}
