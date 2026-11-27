using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_create(ulong handle, ulong entity, ulong address, ulong length);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_set_color", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_set_color(ulong handle, ulong entity, float r, float g, float b, float a);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_set_sorting", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_set_sorting(ulong handle, ulong entity, int sortingLayer, int sortingOrder);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_set_flip", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_set_flip(ulong handle, ulong entity, byte flipX, byte flipY);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_set_enabled", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_set_enabled(ulong handle, ulong entity, byte enabled);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_set_draw_mode", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_set_draw_mode(ulong handle, ulong entity, uint mode);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_sprite_get_state", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_sprite_get_state(ulong handle, ulong entity, IntPtr blob);
    }
}
