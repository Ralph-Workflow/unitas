using System;
using System.Runtime.InteropServices;
using System.Text;
using Cinemachine;
using UnityEngine;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_cinemachine_world_create();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_destroy(ulong world);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_cinemachine_world_buffer(ulong world, ulong capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong unitas_cinemachine_world_add_camera(
            ulong world,
            ulong nameAddress,
            ulong nameLength,
            int priority);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_priority(
            ulong world,
            ulong camera,
            int priority);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_active(ulong world, ulong camera, byte active);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_follow(
            ulong world,
            ulong camera,
            byte hasFollow,
            ulong entity,
            double offsetX,
            double offsetY,
            double offsetZ,
            double damping);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_look_at(
            ulong world,
            ulong camera,
            byte hasLookAt,
            ulong entity,
            double damping);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_target_position(
            ulong world,
            ulong entity,
            double x,
            double y,
            double z);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_default_blend(
            ulong world,
            int style,
            double duration);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_set_custom_blend(
            ulong world,
            ulong fromAddress,
            ulong fromLength,
            ulong toAddress,
            ulong toLength,
            int style,
            double duration);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_update(ulong world, double deltaSeconds);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_cinemachine_world_state(ulong world, ulong address, ulong capacity);
    }

    public sealed class RuntimeCinemachineProvider : ICinemachineProvider
    {
        private const int BufferSize = 1024;
        private const int NameSlotSize = 128;
        private const int InputOffset = 512;
        private const int StateOffset = 0;
        private const int StateSize = 264;
        private const int LiveNameOffset = 0;
        private const int PoseOffset = 64;
        private const int BlendFromOffset = 120;
        private const int BlendToOffset = 184;
        private const int BlendWeightOffset = 248;
        private const int LiveFlagOffset = 256;
        private const int BlendFlagOffset = 260;

        public ulong CreateWorld()
        {
            return RuntimeInterop.unitas_cinemachine_world_create();
        }

        public void DestroyWorld(ulong world)
        {
            if (world == 0UL)
            {
                return;
            }
            RuntimeInterop.unitas_cinemachine_world_destroy(world);
        }

        public ulong AddCamera(ulong world, string name, int priority)
        {
            IntPtr buffer = Buffer(world);
            if (buffer == IntPtr.Zero)
            {
                return 0UL;
            }
            int length = WriteName(buffer, InputOffset, name);
            IntPtr nameAddress = At(buffer, InputOffset);
            return RuntimeInterop.unitas_cinemachine_world_add_camera(
                world,
                (ulong)nameAddress.ToInt64(),
                (ulong)length,
                priority);
        }

        public void SetCameraPriority(ulong world, ulong camera, int priority)
        {
            RuntimeInterop.unitas_cinemachine_world_set_priority(world, camera, priority);
        }

        public void SetCameraActive(ulong world, ulong camera, bool active)
        {
            RuntimeInterop.unitas_cinemachine_world_set_active(world, camera, active ? (byte)1 : (byte)0);
        }

        public void SetCameraFollow(
            ulong world,
            ulong camera,
            bool hasFollow,
            ulong followTarget,
            Vector3 offset,
            float damping)
        {
            RuntimeInterop.unitas_cinemachine_world_set_follow(
                world,
                camera,
                hasFollow ? (byte)1 : (byte)0,
                followTarget,
                offset.x,
                offset.y,
                offset.z,
                damping);
        }

        public void SetCameraLookAt(
            ulong world,
            ulong camera,
            bool hasLookAt,
            ulong lookAtTarget,
            float damping)
        {
            RuntimeInterop.unitas_cinemachine_world_set_look_at(
                world,
                camera,
                hasLookAt ? (byte)1 : (byte)0,
                lookAtTarget,
                damping);
        }

        public void SetTargetPosition(ulong world, ulong target, Vector3 position)
        {
            RuntimeInterop.unitas_cinemachine_world_set_target_position(world, target, position.x, position.y, position.z);
        }

        public void SetDefaultBlend(ulong world, CinemachineBlendStyle style, float duration)
        {
            RuntimeInterop.unitas_cinemachine_world_set_default_blend(world, (int)style, duration);
        }

        public void SetCustomBlend(
            ulong world,
            string from,
            string to,
            CinemachineBlendStyle style,
            float duration)
        {
            IntPtr buffer = Buffer(world);
            if (buffer == IntPtr.Zero)
            {
                return;
            }
            int fromLength = WriteName(buffer, InputOffset, from);
            int toLength = WriteName(buffer, InputOffset + NameSlotSize, to);
            IntPtr fromAddress = At(buffer, InputOffset);
            IntPtr toAddress = At(buffer, InputOffset + NameSlotSize);
            RuntimeInterop.unitas_cinemachine_world_set_custom_blend(
                world,
                (ulong)fromAddress.ToInt64(),
                (ulong)fromLength,
                (ulong)toAddress.ToInt64(),
                (ulong)toLength,
                (int)style,
                duration);
        }

        public void UpdateWorld(ulong world, double deltaSeconds)
        {
            RuntimeInterop.unitas_cinemachine_world_update(world, deltaSeconds);
        }

        public bool TryReadState(ulong world, out CinemachineBrainState state)
        {
            state = default;
            IntPtr buffer = Buffer(world);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            IntPtr record = At(buffer, StateOffset);
            int result = RuntimeInterop.unitas_cinemachine_world_state(
                world,
                (ulong)record.ToInt64(),
                (ulong)StateSize);
            if (result != 0)
            {
                return false;
            }
            byte[] bytes = new byte[StateSize];
            Marshal.Copy(record, bytes, 0, StateSize);
            Vector3 position = new Vector3(
                (float)BitConverter.ToDouble(bytes, PoseOffset),
                (float)BitConverter.ToDouble(bytes, PoseOffset + 8),
                (float)BitConverter.ToDouble(bytes, PoseOffset + 16));
            Quaternion rotation = new Quaternion(
                (float)BitConverter.ToDouble(bytes, PoseOffset + 24),
                (float)BitConverter.ToDouble(bytes, PoseOffset + 32),
                (float)BitConverter.ToDouble(bytes, PoseOffset + 40),
                (float)BitConverter.ToDouble(bytes, PoseOffset + 48));
            state = new CinemachineBrainState(
                BitConverter.ToInt32(bytes, LiveFlagOffset) != 0,
                ReadName(bytes, LiveNameOffset),
                position,
                rotation,
                BitConverter.ToInt32(bytes, BlendFlagOffset) != 0,
                ReadName(bytes, BlendFromOffset),
                ReadName(bytes, BlendToOffset),
                (float)BitConverter.ToDouble(bytes, BlendWeightOffset));
            return true;
        }

        private static IntPtr Buffer(ulong world)
        {
            ulong address = RuntimeInterop.unitas_cinemachine_world_buffer(world, BufferSize);
            return address == 0UL ? IntPtr.Zero : new IntPtr((long)address);
        }

        private static IntPtr At(IntPtr buffer, int offset)
        {
            return new IntPtr(buffer.ToInt64() + offset);
        }

        private static int WriteName(IntPtr buffer, int offset, string value)
        {
            byte[] bytes = Encoding.UTF8.GetBytes(value ?? string.Empty);
            int length = Math.Min(bytes.Length, NameSlotSize);
            if (length > 0)
            {
                Marshal.Copy(bytes, 0, At(buffer, offset), length);
            }
            return length;
        }

        private static string ReadName(byte[] bytes, int offset)
        {
            int end = offset;
            while (end < bytes.Length && bytes[end] != 0)
            {
                end++;
            }
            return end == offset ? string.Empty : Encoding.UTF8.GetString(bytes, offset, end - offset);
        }
    }
}
