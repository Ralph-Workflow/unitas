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

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_id_map", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_render_id_map(ulong handle, ulong camera, uint width, uint height);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_camera_world_to_screen_point", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_camera_world_to_screen_point(ulong handle, ulong camera, float x, float y, float z, IntPtr outBuffer);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_camera_screen_point_to_ray", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_camera_screen_point_to_ray(ulong handle, ulong camera, float x, float y, float z, IntPtr outBuffer);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_buffer", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_runtime_render_buffer(ulong handle, ulong capacity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_create", CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong unitas_runtime_render_light2d_create(ulong handle, ulong entity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_destroy", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_destroy(ulong handle, ulong light);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_kind", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_kind(ulong handle, ulong light, uint kind);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_get_kind", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_get_kind(ulong handle, ulong light);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_color", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_color(ulong handle, ulong light, float r, float g, float b);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_intensity", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_intensity(ulong handle, ulong light, float intensity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_radii", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_radii(ulong handle, ulong light, float innerRadius, float outerRadius);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_angles", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_angles(ulong handle, ulong light, float innerAngleDeg, float outerAngleDeg);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_position", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_position(ulong handle, ulong light, float x, float y);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_direction", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_direction(ulong handle, ulong light, float x, float y);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_set_enabled", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_set_enabled(ulong handle, ulong light, byte enabled);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_render_light2d_get_state", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_render_light2d_get_state(ulong handle, ulong light, IntPtr blob);

        private static readonly RuntimeLight2DHost Light2DHostInstance = CreateLight2DHost();

        private static RuntimeLight2DHost CreateLight2DHost()
        {
            RuntimeLight2DHost host = new RuntimeLight2DHost();
            UnityEngine.Rendering.Universal.Light2D.RegisterDefaultProviderFactory(
                () => host.HasActiveRuntime ? host : null);
            return host;
        }

        public static void AttachLight2DHost(ulong runtime)
        {
            Light2DHostInstance.Attach(runtime);
        }

        public static void DetachLight2DHost(ulong runtime)
        {
            Light2DHostInstance.Detach(runtime);
        }

        public static ulong ActiveLight2DRuntime => Light2DHostInstance.CurrentRuntime;

        private sealed class RuntimeLight2DHost : UnityEngine.Rendering.Universal.ILight2DProvider
        {
            private ulong _liveRuntime;

            private ulong EffectiveRuntime => _liveRuntime != 0UL ? _liveRuntime : PlayerLoopRuntime;

            internal ulong CurrentRuntime => EffectiveRuntime;

            internal bool HasActiveRuntime => EffectiveRuntime != 0UL;

            public void Attach(ulong runtime)
            {
                _liveRuntime = runtime;
            }

            public void Detach(ulong runtime)
            {
                if (_liveRuntime == runtime)
                {
                    _liveRuntime = 0UL;
                }
            }

            public ulong CreateLight2D(ulong entityId)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL)
                {
                    return 0UL;
                }

                return unitas_runtime_render_light2d_create(runtime, entityId);
            }

            public void DestroyLight2D(ulong lightHandle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_destroy(runtime, lightHandle);
            }

            public void SetLight2DKind(ulong lightHandle, UnityEngine.Rendering.Universal.Light2D.LightType kind)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_kind(runtime, lightHandle, (uint)kind);
            }

            public void SetLight2DColor(ulong lightHandle, UnityEngine.Color color)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_color(runtime, lightHandle, color.r, color.g, color.b);
            }

            public void SetLight2DIntensity(ulong lightHandle, float intensity)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_intensity(runtime, lightHandle, intensity);
            }

            public void SetLight2DRadii(ulong lightHandle, float innerRadius, float outerRadius)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_radii(runtime, lightHandle, innerRadius, outerRadius);
            }

            public void SetLight2DAngles(ulong lightHandle, float innerAngle, float outerAngle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_angles(runtime, lightHandle, innerAngle, outerAngle);
            }

            public void SetLight2DPosition(ulong lightHandle, UnityEngine.Vector2 position)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_position(runtime, lightHandle, position.x, position.y);
            }

            public void SetLight2DDirection(ulong lightHandle, UnityEngine.Vector2 direction)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_direction(runtime, lightHandle, direction.x, direction.y);
            }

            public void SetLight2DEnabled(ulong lightHandle, bool enabled)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || lightHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_render_light2d_set_enabled(runtime, lightHandle, enabled ? (byte)1 : (byte)0);
            }
        }
    }
}
