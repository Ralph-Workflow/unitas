using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_trail_get_emitting(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_trail_set_emitting(ulong handle, ulong entity, byte emitting);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_trail_get_settings(ulong handle, ulong entity, IntPtr blob);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_trail_configure(ulong handle, ulong entity, IntPtr blob);
    }
}
