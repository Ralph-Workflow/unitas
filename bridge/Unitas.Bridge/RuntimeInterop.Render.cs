using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_mesh_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_mesh_create(ulong handle, IntPtr positions, uint positionCount, IntPtr normals, uint normalCount, IntPtr indices, uint indexCount);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_mesh_destroy", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_mesh_destroy(ulong handle, ulong mesh);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_material_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_material_create(ulong handle);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_material_set_color", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_material_set_color(ulong handle, ulong material, float r, float g, float b, float a);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_camera_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_camera_create(ulong handle, ulong entity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_camera_update", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_camera_update(ulong handle, ulong camera, IntPtr blob);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_renderer_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_renderer_create(ulong handle, ulong entity, ulong mesh, ulong material);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_renderer_set_enabled", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_renderer_set_enabled(ulong handle, ulong renderer, byte enabled);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_light_create(ulong handle, ulong entity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light_update", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_light_update(ulong handle, ulong light, float directionX, float directionY, float directionZ, float colorR, float colorG, float colorB, float intensity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_frame", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_render_frame(ulong handle, ulong camera, uint width, uint height);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_camera_world_to_screen_point", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_camera_world_to_screen_point(ulong handle, ulong camera, float x, float y, float z, IntPtr outBuffer);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_camera_screen_point_to_ray", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_camera_screen_point_to_ray(ulong handle, ulong camera, float x, float y, float z, IntPtr outBuffer);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_buffer", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_render_buffer(ulong handle, ulong capacity);
    }
}
