using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_animator_controller_create(ulong handle, IntPtr address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_controller_destroy(ulong handle, ulong controller);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_animator_attach(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_controller(ulong handle, ulong entity, ulong controller);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_float(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, float value);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_float(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_int(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, int value);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_int(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_bool(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, uint value);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_bool(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_trigger(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_reset_trigger(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_speed(ulong handle, ulong entity, float speed);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_speed(ulong handle, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_state_info(ulong handle, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_state_name(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_drain_fired_events(ulong handle);
    }
}
