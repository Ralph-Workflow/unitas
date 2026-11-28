using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_ensure(ulong handle, ulong entity, uint resolution);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_has(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_get_resolution(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_set_resolution(ulong handle, ulong entity, uint resolution);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_get_size(ulong handle, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_set_size(ulong handle, ulong entity, double sx, double sy, double sz);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_set_position(ulong handle, ulong entity, double px, double py, double pz);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_sample_height(ulong handle, ulong entity, double x, double y, double z, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_get_heights(
            ulong handle,
            ulong entity,
            uint xBase,
            uint yBase,
            uint width,
            uint height,
            IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_terrain_set_heights(
            ulong handle,
            ulong entity,
            uint xBase,
            uint yBase,
            uint width,
            uint height,
            IntPtr inBuffer);
    }
}
