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

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_play(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_play_hash(ulong handle, ulong entity, int hash);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_play_queued(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_cross_fade(ulong handle, ulong entity, IntPtr nameAddress, ulong nameLength, float duration);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_cross_fade_hash(ulong handle, ulong entity, int hash, float duration);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_apply_root_motion(ulong handle, ulong entity, uint apply);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_apply_root_motion(ulong handle, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_root_motion(ulong handle, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_ik_position(ulong handle, ulong entity, uint goal, float x, float y, float z);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_ik_position(ulong handle, ulong entity, uint goal, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_ik_position_weight(ulong handle, ulong entity, uint goal, float weight);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_ik_position_weight(ulong handle, ulong entity, uint goal, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_ik_rotation(ulong handle, ulong entity, uint goal, float x, float y, float z, float w);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_ik_rotation(ulong handle, ulong entity, uint goal, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_ik_rotation_weight(ulong handle, ulong entity, uint goal, float weight);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_ik_rotation_weight(ulong handle, ulong entity, uint goal, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_ik_hint_position(ulong handle, ulong entity, uint hint, float x, float y, float z);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_ik_hint_position(ulong handle, ulong entity, uint hint, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_ik_hint_position_weight(ulong handle, ulong entity, uint hint, float weight);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_get_ik_hint_position_weight(ulong handle, ulong entity, uint hint, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_look_at_position(ulong handle, ulong entity, float x, float y, float z);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_set_look_at_weight(ulong handle, ulong entity, float weight, float bodyWeight, float headWeight, float eyesWeight, float clampWeight);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_animator_string_to_hash(ulong handle, IntPtr nameAddress, ulong nameLength);
    }
}
