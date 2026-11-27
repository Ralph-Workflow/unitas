using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_reset();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_set_resolution(uint width, uint height, uint fullScreen);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_display_width();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_display_height();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_set_full_screen(uint fullScreen);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_display_full_screen();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_set_v_sync_count(uint vSyncCount);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_display_v_sync_count();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_set_target_frame_rate(int targetFrameRate);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_target_frame_rate();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_set_quality_level(uint index);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_display_quality_level();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_display_request_quit();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_display_quit_requested();

        public static void ApplyDisplayResolution(int width, int height, bool fullScreen)
        {
            if (width <= 0 || height <= 0)
            {
                return;
            }

            try
            {
                unitas_runtime_display_set_resolution((uint)width, (uint)height, fullScreen ? 1U : 0U);
            }
            catch (DllNotFoundException)
            {
            }
            catch (EntryPointNotFoundException)
            {
            }
        }

        public static void ApplyDisplayFullScreen(bool fullScreen)
        {
            try
            {
                unitas_runtime_display_set_full_screen(fullScreen ? 1U : 0U);
            }
            catch (DllNotFoundException)
            {
            }
            catch (EntryPointNotFoundException)
            {
            }
        }

        public static void ApplyDisplayVSyncCount(int vSyncCount)
        {
            uint clamped = vSyncCount <= 0 ? 0U : (uint)Math.Min(vSyncCount, 4);
            try
            {
                unitas_runtime_display_set_v_sync_count(clamped);
            }
            catch (DllNotFoundException)
            {
            }
            catch (EntryPointNotFoundException)
            {
            }
        }

        public static void ApplyDisplayTargetFrameRate(int targetFrameRate)
        {
            try
            {
                unitas_runtime_display_set_target_frame_rate(targetFrameRate);
            }
            catch (DllNotFoundException)
            {
            }
            catch (EntryPointNotFoundException)
            {
            }
        }

        public static void ApplyDisplayQualityLevel(int index)
        {
            if (index < 0)
            {
                return;
            }

            try
            {
                unitas_runtime_display_set_quality_level((uint)index);
            }
            catch (DllNotFoundException)
            {
            }
            catch (EntryPointNotFoundException)
            {
            }
        }

        public static void RequestDisplayQuit()
        {
            try
            {
                unitas_runtime_display_request_quit();
            }
            catch (DllNotFoundException)
            {
            }
            catch (EntryPointNotFoundException)
            {
            }
        }
    }
}
