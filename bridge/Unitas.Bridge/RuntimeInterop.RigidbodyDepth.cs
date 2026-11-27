using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_angular_velocity(
            ulong handle,
            ulong entity,
            ulong outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_set_body_angular_velocity(
            ulong handle,
            ulong entity,
            ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_teleport_body_rotation(
            ulong handle,
            ulong entity,
            IntPtr inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_add_body_torque(
            ulong handle,
            ulong entity,
            ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_add_body_relative_force(
            ulong handle,
            ulong entity,
            ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_add_body_relative_torque(
            ulong handle,
            ulong entity,
            ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_add_body_force_at_position(
            ulong handle,
            ulong entity,
            ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_add_body_explosion_force(
            ulong handle,
            ulong entity,
            ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_sleep(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_wake_up(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_is_sleeping(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_set_interpolation(
            ulong handle,
            ulong entity,
            uint mode);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_get_interpolation(ulong handle, ulong entity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_set_constraints(
            ulong handle,
            ulong entity,
            uint constraints);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_body_get_constraints(ulong handle, ulong entity);
    }
}
