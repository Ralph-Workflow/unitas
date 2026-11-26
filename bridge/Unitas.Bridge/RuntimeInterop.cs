using System;
using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        public const string LibraryName = "unitas_runtime";

        public const int QueryRecordSize = 64;

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateScriptCallback(ulong entity, ulong guidHi, ulong guidLo);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void AwakeCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void OnEnableCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void StartCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FixedUpdateCallback(ulong scriptHandle, double fixedDeltaSeconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void UpdateCallback(ulong scriptHandle, double deltaSeconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void LateUpdateCallback(ulong scriptHandle, double deltaSeconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void OnDisableCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DestroyCallback(ulong scriptHandle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void CollisionCallback(ulong entityA, ulong entityB, uint phase);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TriggerCallback(ulong entityA, ulong entityB, uint phase);

        static RuntimeInterop()
        {
            NativeLibrary.SetDllImportResolver(
                typeof(RuntimeInterop).Assembly,
                (libraryName, assembly, searchPath) =>
                {
                    if (libraryName == LibraryName)
                    {
                        string? envPath = Environment.GetEnvironmentVariable("UNITAS_RUNTIME_LIBRARY");
                        if (!string.IsNullOrEmpty(envPath) && NativeLibrary.TryLoad(envPath, out IntPtr handle))
                        {
                            return handle;
                        }
                    }
                    return IntPtr.Zero;
                });
            UnityEngine.Time.TimeScaleHost = TimeScaleHostImpl;
        }

        private sealed class TimeScaleHost : UnityEngine.ITimeScaleHost
        {
            // A fresh runtime clamps fixed-step catch-up at 1/3 s, so the default is never pushed again.
            private const double NativeDefaultMaximumDeltaTime = 1.0 / 3.0;

            private ulong _liveRuntime;
            private double _appliedScale = 1.0;
            private float _requestedMaximumDeltaTime = UnityEngine.Time.DefaultMaximumDeltaTime;
            private double _appliedMaximumDeltaTime = NativeDefaultMaximumDeltaTime;

            public void Attach(ulong runtime)
            {
                _liveRuntime = runtime;
                _appliedMaximumDeltaTime = NativeDefaultMaximumDeltaTime;
                if (_requestedMaximumDeltaTime != UnityEngine.Time.DefaultMaximumDeltaTime)
                {
                    ApplyMaximumDeltaTime(_requestedMaximumDeltaTime);
                }
            }

            public void Detach(ulong runtime)
            {
                if (_liveRuntime == runtime)
                {
                    _liveRuntime = 0;
                }
            }

            public void ApplyTimeScale(float timeScale)
            {
                _appliedScale = timeScale;
                if (_liveRuntime == 0)
                {
                    return;
                }
                if (unitas_runtime_set_time_scale(_liveRuntime, timeScale) != 0)
                {
                    _liveRuntime = 0;
                }
            }

            public bool TryGetTimeScale(out float timeScale)
            {
                timeScale = 0f;
                if (_liveRuntime == 0)
                {
                    return false;
                }
                double nativeScale = unitas_runtime_time_scale(_liveRuntime);
                if (nativeScale != _appliedScale)
                {
                    return false;
                }
                timeScale = (float)nativeScale;
                return true;
            }

            public void ApplyMaximumDeltaTime(float maximumDeltaTime)
            {
                _requestedMaximumDeltaTime = maximumDeltaTime;
                if (_liveRuntime == 0)
                {
                    return;
                }
                if (maximumDeltaTime == _appliedMaximumDeltaTime)
                {
                    return;
                }
                if (unitas_runtime_set_maximum_delta_time(_liveRuntime, maximumDeltaTime) != 0)
                {
                    _liveRuntime = 0;
                    return;
                }
                _appliedMaximumDeltaTime = maximumDeltaTime;
            }

            public bool TryGetMaximumDeltaTime(out float maximumDeltaTime)
            {
                maximumDeltaTime = 0f;
                if (_liveRuntime == 0)
                {
                    return false;
                }
                double nativeMaximumDeltaTime = unitas_runtime_maximum_delta_time(_liveRuntime);
                if (nativeMaximumDeltaTime != _appliedMaximumDeltaTime)
                {
                    return false;
                }
                maximumDeltaTime = (float)nativeMaximumDeltaTime;
                return true;
            }
        }

        private static readonly TimeScaleHost TimeScaleHostImpl = new TimeScaleHost();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_new")]
        private static extern ulong unitas_runtime_new_native();

        public static ulong unitas_runtime_new()
        {
            ulong handle = unitas_runtime_new_native();
            TimeScaleHostImpl.Attach(handle);
            UnityEngine.Time.PushTimeScaleToHost();
            return handle;
        }

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_path_buffer(ulong runtime, ulong capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_load_scene(ulong runtime, IntPtr address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_callbacks(
            ulong runtime,
            CreateScriptCallback createScript,
            AwakeCallback awake,
            OnEnableCallback onEnable,
            StartCallback start,
            FixedUpdateCallback fixedUpdate,
            UpdateCallback update,
            LateUpdateCallback lateUpdate,
            OnDisableCallback onDisable,
            DestroyCallback destroy,
            CollisionCallback collision,
            TriggerCallback trigger);

        // deltaSeconds is the unscaled wall delta; the runtime applies its own time scale.
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_tick(ulong runtime, double deltaSeconds);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_raycast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double directionX,
            double directionY,
            double directionZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_linecast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double dirX,
            double dirY,
            double dirZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_sphere_cast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double dirX,
            double dirY,
            double dirZ,
            double radius,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_box_cast(
            ulong runtime,
            double originX,
            double originY,
            double originZ,
            double dirX,
            double dirY,
            double dirZ,
            double halfX,
            double halfY,
            double halfZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_overlap_sphere(
            ulong runtime,
            double centerX,
            double centerY,
            double centerZ,
            double radius,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_query_buffer(ulong runtime, uint capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_destroy")]
        private static extern int unitas_runtime_destroy_native(ulong runtime);

        public static int unitas_runtime_destroy(ulong runtime)
        {
            int result = unitas_runtime_destroy_native(runtime);
            TimeScaleHostImpl.Detach(runtime);
            return result;
        }

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr unitas_runtime_error_message(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_fixed_timestep(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_time_scale(ulong runtime, double timeScale);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_time_scale(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_maximum_delta_time(ulong runtime, double seconds);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_maximum_delta_time(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_body_position(ulong runtime, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_body_velocity(ulong runtime, ulong entity, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_body_position(ulong runtime, ulong entity, IntPtr inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_body_rotation(ulong runtime, ulong entity, IntPtr inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_teleport_body_position(ulong runtime, ulong entity, IntPtr inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_set_body_velocity(ulong runtime, ulong entity, IntPtr inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_body_set_collision_detection_mode(ulong runtime, ulong entity, int mode);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_add_body_force(ulong runtime, ulong entity, IntPtr inBuffer);

        [StructLayout(LayoutKind.Sequential)]
        public struct AbiCharacterMoveInput
        {
            public ulong EntityId;
            public double PositionX;
            public double PositionY;
            public double PositionZ;
            public double MotionX;
            public double MotionY;
            public double MotionZ;
            public double Radius;
            public double Height;
            public double CenterX;
            public double CenterY;
            public double CenterZ;
            public double SlopeLimitDeg;
            public double StepOffset;
            public double SkinWidth;
            public double MinMoveDistance;
            public double DeltaSeconds;
            public uint LayerMask;
            public byte Enabled;
            public byte Reserved0;
            public byte Reserved1;
            public byte Reserved2;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct AbiCharacterMoveOutput
        {
            public double PositionX;
            public double PositionY;
            public double PositionZ;
            public ulong GroundEntityId;
            public uint CollisionFlags;
            public byte IsGrounded;
            public byte Reserved0;
            public byte Reserved1;
            public byte Reserved2;
        }

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_character_move(
            ulong runtime,
            ref AbiCharacterMoveInput input,
            out AbiCharacterMoveOutput output);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_character_teleport(
            ulong runtime,
            ulong entity,
            IntPtr inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_character_controller_config(
            ulong runtime,
            ulong entity,
            IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_key_event(ulong runtime, uint key, uint pressed);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_mouse_button_event(ulong runtime, uint btn, uint pressed);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_pointer_move(ulong runtime, double x, double y);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_scroll(ulong runtime, double x, double y);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_axis_event(ulong runtime, IntPtr address, ulong length, double value);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_input_key_state(ulong runtime, uint key);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint unitas_runtime_input_mouse_state(ulong runtime, uint btn);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_pointer(ulong runtime, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_scroll_state(ulong runtime, IntPtr outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_input_axis(ulong runtime, IntPtr address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_input_axis_raw(ulong runtime, IntPtr address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern double unitas_runtime_input_injected_axis(ulong runtime, IntPtr address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_input_button_state(ulong runtime, IntPtr address, ulong length);

        public static QueryHit[] ReadQueryHits(IntPtr buffer, int count)
        {
            if (count <= 0)
            {
                return Array.Empty<QueryHit>();
            }
            byte[] bytes = new byte[count * QueryRecordSize];
            Marshal.Copy(buffer, bytes, 0, bytes.Length);
            QueryHit[] hits = new QueryHit[count];
            for (int index = 0; index < count; index++)
            {
                int offset = index * QueryRecordSize;
                hits[index] = new QueryHit
                {
                    Entity = BitConverter.ToUInt64(bytes, offset),
                    Distance = BitConverter.ToDouble(bytes, offset + 8),
                    X = BitConverter.ToDouble(bytes, offset + 16),
                    Y = BitConverter.ToDouble(bytes, offset + 24),
                    Z = BitConverter.ToDouble(bytes, offset + 32),
                    NX = BitConverter.ToDouble(bytes, offset + 40),
                    NY = BitConverter.ToDouble(bytes, offset + 48),
                    NZ = BitConverter.ToDouble(bytes, offset + 56),
                };
            }
            return hits;
        }
    }

    public struct QueryHit
    {
        public ulong Entity;
        public double Distance;
        public double X;
        public double Y;
        public double Z;
        public double NX;
        public double NY;
        public double NZ;
    }
}
