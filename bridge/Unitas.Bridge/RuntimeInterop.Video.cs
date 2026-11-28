using System;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine.Video;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        private const int VideoStateBlobBytes = 64;
        private const uint DefaultPrepareWaitTimeoutMs = 5000u;

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_create", CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong unitas_runtime_video_create(ulong handle, ulong entity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_destroy", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_destroy(ulong handle, ulong player);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_load_clip", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_load_clip(ulong handle, ulong player, ulong pathPtr, ulong pathLen);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_prepare", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_prepare(ulong handle, ulong player);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_wait_prepared", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_wait_prepared(ulong handle, ulong player, uint timeoutMs);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_play", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_play(ulong handle, ulong player);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_pause", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_pause(ulong handle, ulong player);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_stop", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_stop(ulong handle, ulong player);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_step_forward", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_step_forward(ulong handle, ulong player);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_set_time", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_set_time(ulong handle, ulong player, double seconds);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_set_frame", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_set_frame(ulong handle, ulong player, long frame);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_set_looping", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_set_looping(ulong handle, ulong player, uint looping);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_set_playback_speed", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_set_playback_speed(ulong handle, ulong player, float speed);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_set_send_frame_ready_events", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_set_send_frame_ready_events(ulong handle, ulong player, uint enabled);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_set_target_entity", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_set_target_entity(ulong handle, ulong player, ulong targetEntity);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_advance", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_advance(ulong handle, ulong player, double deltaSeconds);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_get_state", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_get_state(ulong handle, ulong player, ulong outPtr);

        [DllImport(LibraryName, EntryPoint = "unitas_runtime_video_get_frame_rgba", CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_video_get_frame_rgba(ulong handle, ulong player);

        private static readonly RuntimeVideoHost VideoHostInstance = CreateVideoHost();

        private static RuntimeVideoHost CreateVideoHost()
        {
            RuntimeVideoHost host = new RuntimeVideoHost();
            VideoPlayer.RegisterDefaultProviderFactory(() => host.HasActiveRuntime ? host : null);
            return host;
        }

        public static void AttachVideoHost(ulong runtime)
        {
            VideoHostInstance.Attach(runtime);
        }

        public static void DetachVideoHost(ulong runtime)
        {
            VideoHostInstance.Detach(runtime);
        }

        public static ulong ActiveVideoRuntime => VideoHostInstance.CurrentRuntime;

        private sealed class RuntimeVideoHost : IVideoPlayerProvider
        {
            private ulong _liveRuntime;

            private ulong EffectiveRuntime
            {
                get
                {
                    if (_liveRuntime != 0UL)
                    {
                        return _liveRuntime;
                    }

                    if (ActiveLight2DRuntime != 0UL)
                    {
                        return ActiveLight2DRuntime;
                    }

                    return PlayerLoopRuntime;
                }
            }

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

            public ulong CreatePlayer(ulong entityId)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL)
                {
                    return 0UL;
                }

                return unitas_runtime_video_create(runtime, entityId);
            }

            public void DestroyPlayer(ulong playerHandle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_destroy(runtime, playerHandle);
            }

            public bool LoadClip(ulong playerHandle, string path)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL || string.IsNullOrEmpty(path))
                {
                    return false;
                }

                byte[] utf8 = Encoding.UTF8.GetBytes(path);
                IntPtr buffer = unitas_runtime_path_buffer(runtime, (ulong)Math.Max(utf8.Length, 1));
                if (buffer == IntPtr.Zero)
                {
                    return false;
                }

                Marshal.Copy(utf8, 0, buffer, utf8.Length);
                return unitas_runtime_video_load_clip(runtime, playerHandle, (ulong)buffer.ToInt64(), (ulong)utf8.Length) == 0;
            }

            public bool Prepare(ulong playerHandle, bool waitForFirstFrame)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return false;
                }

                if (unitas_runtime_video_prepare(runtime, playerHandle) != 0)
                {
                    return false;
                }

                if (waitForFirstFrame)
                {
                    return unitas_runtime_video_wait_prepared(runtime, playerHandle, DefaultPrepareWaitTimeoutMs) == 0;
                }

                return true;
            }

            public bool Play(ulong playerHandle, bool waitForFirstFrame)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return false;
                }

                if (unitas_runtime_video_play(runtime, playerHandle) != 0)
                {
                    return false;
                }

                if (waitForFirstFrame)
                {
                    if (unitas_runtime_video_wait_prepared(runtime, playerHandle, DefaultPrepareWaitTimeoutMs) != 0)
                    {
                        return false;
                    }

                    unitas_runtime_video_play(runtime, playerHandle);
                }

                return true;
            }

            public void Pause(ulong playerHandle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_pause(runtime, playerHandle);
            }

            public void Stop(ulong playerHandle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_stop(runtime, playerHandle);
            }

            public void StepForward(ulong playerHandle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_step_forward(runtime, playerHandle);
            }

            public void SetTime(ulong playerHandle, double seconds)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_set_time(runtime, playerHandle, seconds);
            }

            public void SetFrame(ulong playerHandle, long frameIndex)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_set_frame(runtime, playerHandle, frameIndex);
            }

            public void SetLooping(ulong playerHandle, bool isLooping)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_set_looping(runtime, playerHandle, isLooping ? 1u : 0u);
            }

            public void SetPlaybackSpeed(ulong playerHandle, float speed)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_set_playback_speed(runtime, playerHandle, speed);
            }

            public void SetSendFrameReadyEvents(ulong playerHandle, bool enabled)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_set_send_frame_ready_events(runtime, playerHandle, enabled ? 1u : 0u);
            }

            public void SetTargetEntity(ulong playerHandle, ulong targetEntityId)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_set_target_entity(runtime, playerHandle, targetEntityId);
            }

            public void Advance(ulong playerHandle, double deltaSeconds)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return;
                }

                unitas_runtime_video_advance(runtime, playerHandle, deltaSeconds);
            }

            public bool TryGetState(ulong playerHandle, out VideoPlayerNativeState state)
            {
                state = default;
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return false;
                }

                IntPtr buffer = unitas_runtime_path_buffer(runtime, VideoStateBlobBytes);
                if (buffer == IntPtr.Zero)
                {
                    return false;
                }

                if (unitas_runtime_video_get_state(runtime, playerHandle, (ulong)buffer.ToInt64()) != 0)
                {
                    return false;
                }

                byte[] blob = new byte[VideoStateBlobBytes];
                Marshal.Copy(buffer, blob, 0, VideoStateBlobBytes);
                uint width = BitConverter.ToUInt32(blob, 0);
                uint height = BitConverter.ToUInt32(blob, 4);
                ulong frameCount = BitConverter.ToUInt64(blob, 8);
                double frameRate = BitConverter.ToDouble(blob, 16);
                double lengthSeconds = BitConverter.ToDouble(blob, 24);
                long currentFrame = BitConverter.ToInt64(blob, 32);
                double currentTime = BitConverter.ToDouble(blob, 40);
                float playbackSpeed = BitConverter.ToSingle(blob, 48);
                uint flags = BitConverter.ToUInt32(blob, 52);
                uint eventBits = BitConverter.ToUInt32(blob, 56);
                uint hasError = BitConverter.ToUInt32(blob, 60);
                string? errorMsg = hasError != 0 ? "VideoPlayer playback failed" : null;
                state = new VideoPlayerNativeState(
                    width,
                    height,
                    frameCount,
                    frameRate,
                    lengthSeconds,
                    currentFrame,
                    currentTime,
                    playbackSpeed,
                    (flags & (1u << 0)) != 0,
                    (flags & (1u << 1)) != 0,
                    (flags & (1u << 2)) != 0,
                    (flags & (1u << 3)) != 0,
                    (flags & (1u << 4)) != 0,
                    eventBits,
                    errorMsg);
                return true;
            }

            public byte[] GetFrameRgba(ulong playerHandle)
            {
                ulong runtime = EffectiveRuntime;
                if (runtime == 0UL || playerHandle == 0UL)
                {
                    return Array.Empty<byte>();
                }

                int byteLen = unitas_runtime_video_get_frame_rgba(runtime, playerHandle);
                if (byteLen <= 0)
                {
                    return Array.Empty<byte>();
                }

                IntPtr ptr = unitas_runtime_path_buffer(runtime, (ulong)byteLen);
                if (ptr == IntPtr.Zero)
                {
                    return Array.Empty<byte>();
                }

                byte[] rgba = new byte[byteLen];
                Marshal.Copy(ptr, rgba, 0, byteLen);
                return rgba;
            }
        }
    }
}
