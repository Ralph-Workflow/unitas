using System;
using System.Globalization;
using System.Runtime.InteropServices;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed class RuntimeRenderProvider : IRenderProvider
    {
        private const int CameraBlobFloats = 8;
        private const int CameraBlobBytes = CameraBlobFloats * sizeof(float);
        private const int ScreenPointFloats = 3;
        private const int RayFloats = 6;
        private const int VectorBytes = 12;
        private const int VectorFloats = 3;
        private const int IndexBytes = 4;

        private readonly ulong _runtimeHandle;

        public RuntimeRenderProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public ulong CreateMesh(Vector3[] vertices, Vector3[] normals, int[] triangles)
        {
            if (_runtimeHandle == 0UL)
            {
                return 0UL;
            }

            int positionBytes = vertices.Length * VectorBytes;
            int normalBytes = normals.Length * VectorBytes;
            int indexBytes = triangles.Length * IndexBytes;
            IntPtr buffer = AcquirePathBuffer(positionBytes + normalBytes + indexBytes);
            Marshal.Copy(EncodeVectors(vertices), 0, buffer, positionBytes);
            Marshal.Copy(EncodeVectors(normals), 0, buffer + positionBytes, normalBytes);
            Marshal.Copy(EncodeIndices(triangles), 0, buffer + positionBytes + normalBytes, indexBytes);
            return RuntimeInterop.unitas_runtime_mesh_create(
                _runtimeHandle,
                buffer,
                (uint)(vertices.Length * VectorFloats),
                buffer + positionBytes,
                (uint)(normals.Length * VectorFloats),
                buffer + positionBytes + normalBytes,
                (uint)triangles.Length);
        }

        public void DestroyMesh(ulong meshHandle)
        {
            if (_runtimeHandle == 0UL || meshHandle == 0UL)
            {
                return;
            }

            RuntimeInterop.unitas_runtime_mesh_destroy(_runtimeHandle, meshHandle);
        }

        public ulong CreateMaterial()
        {
            if (_runtimeHandle == 0UL)
            {
                return 0UL;
            }

            return RuntimeInterop.unitas_runtime_material_create(_runtimeHandle);
        }

        public void SetMaterialColor(ulong materialHandle, Color color)
        {
            if (_runtimeHandle == 0UL || materialHandle == 0UL)
            {
                return;
            }

            int result = RuntimeInterop.unitas_runtime_material_set_color(_runtimeHandle, materialHandle, color.r, color.g, color.b, color.a);
            ThrowIfFailed(result, "Failed to set material color");
        }

        public ulong CreateCamera(ulong entityId)
        {
            if (_runtimeHandle == 0UL)
            {
                return 0UL;
            }

            return RuntimeInterop.unitas_runtime_camera_create(_runtimeHandle, entityId);
        }

        public void UpdateCamera(ulong cameraHandle, float orthographicSize, float fieldOfView, float nearClipPlane, float farClipPlane, Color backgroundColor)
        {
            if (_runtimeHandle == 0UL || cameraHandle == 0UL)
            {
                return;
            }

            float[] blob = new float[CameraBlobFloats];
            blob[0] = orthographicSize;
            blob[1] = fieldOfView;
            blob[2] = nearClipPlane;
            blob[3] = farClipPlane;
            blob[4] = backgroundColor.r;
            blob[5] = backgroundColor.g;
            blob[6] = backgroundColor.b;
            blob[7] = 0f;
            IntPtr buffer = AcquirePathBuffer(CameraBlobBytes);
            Marshal.Copy(blob, 0, buffer, CameraBlobFloats);
            int result = RuntimeInterop.unitas_runtime_camera_update(_runtimeHandle, cameraHandle, buffer);
            ThrowIfFailed(result, "Failed to update native camera");
        }

        public Vector3 WorldToScreenPoint(ulong cameraHandle, Vector3 worldPoint)
        {
            if (_runtimeHandle == 0UL || cameraHandle == 0UL)
            {
                return Vector3.zero;
            }

            IntPtr buffer = AcquirePathBuffer(ScreenPointFloats * sizeof(float));
            int result = RuntimeInterop.unitas_runtime_camera_world_to_screen_point(_runtimeHandle, cameraHandle, worldPoint.x, worldPoint.y, worldPoint.z, buffer);
            ThrowIfFailed(result, "Failed to project world point");
            float[] values = new float[ScreenPointFloats];
            Marshal.Copy(buffer, values, 0, ScreenPointFloats);
            return new Vector3(values[0], values[1], values[2]);
        }

        public Ray ScreenPointToRay(ulong cameraHandle, Vector3 screenPoint)
        {
            if (_runtimeHandle == 0UL || cameraHandle == 0UL)
            {
                return default;
            }

            IntPtr buffer = AcquirePathBuffer(RayFloats * sizeof(float));
            int result = RuntimeInterop.unitas_runtime_camera_screen_point_to_ray(_runtimeHandle, cameraHandle, screenPoint.x, screenPoint.y, screenPoint.z, buffer);
            ThrowIfFailed(result, "Failed to build screen point ray");
            float[] values = new float[RayFloats];
            Marshal.Copy(buffer, values, 0, RayFloats);
            return new Ray(
                new Vector3(values[0], values[1], values[2]),
                new Vector3(values[3], values[4], values[5]));
        }

        public ulong CreateRenderer(ulong entityId, ulong meshHandle, ulong materialHandle)
        {
            if (_runtimeHandle == 0UL || meshHandle == 0UL || materialHandle == 0UL)
            {
                return 0UL;
            }

            return RuntimeInterop.unitas_runtime_renderer_create(_runtimeHandle, entityId, meshHandle, materialHandle);
        }

        public void SetRendererEnabled(ulong rendererHandle, bool enabled)
        {
            if (_runtimeHandle == 0UL || rendererHandle == 0UL)
            {
                return;
            }

            int result = RuntimeInterop.unitas_runtime_renderer_set_enabled(_runtimeHandle, rendererHandle, enabled ? (byte)1 : (byte)0);
            ThrowIfFailed(result, "Failed to set renderer enabled state");
        }

        public ulong CreateLight(ulong entityId)
        {
            if (_runtimeHandle == 0UL)
            {
                return 0UL;
            }

            return RuntimeInterop.unitas_runtime_light_create(_runtimeHandle, entityId);
        }

        public void UpdateLight(ulong lightHandle, Vector3 direction, Color color, float intensity)
        {
            if (_runtimeHandle == 0UL || lightHandle == 0UL)
            {
                return;
            }

            int result = RuntimeInterop.unitas_runtime_light_update(
                _runtimeHandle,
                lightHandle,
                direction.x,
                direction.y,
                direction.z,
                color.r,
                color.g,
                color.b,
                intensity);
            ThrowIfFailed(result, "Failed to update native light");
        }

        public byte[] RenderFrame(ulong cameraHandle, uint width, uint height)
        {
            if (_runtimeHandle == 0UL || cameraHandle == 0UL)
            {
                return Array.Empty<byte>();
            }

            int byteCount = RuntimeInterop.unitas_runtime_render_frame(_runtimeHandle, cameraHandle, width, height);
            if (byteCount < 0)
            {
                throw new InvalidOperationException("Failed to render frame (error code " + byteCount.ToString(CultureInfo.InvariantCulture) + "): " + GetErrorMessage());
            }

            if (byteCount == 0)
            {
                return Array.Empty<byte>();
            }

            IntPtr buffer = (IntPtr)RuntimeInterop.unitas_runtime_render_buffer(_runtimeHandle, (ulong)byteCount);
            if (buffer == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to allocate render buffer in native runtime.");
            }

            byte[] frame = new byte[byteCount];
            Marshal.Copy(buffer, frame, 0, byteCount);
            return frame;
        }

        private void ThrowIfFailed(int result, string message)
        {
            if (result == 0)
            {
                return;
            }

            string? error = GetErrorMessage();
            string code = result.ToString(CultureInfo.InvariantCulture);
            if (string.IsNullOrEmpty(error))
            {
                throw new InvalidOperationException(message + " (error code " + code + ").");
            }

            throw new InvalidOperationException(message + " (error code " + code + "): " + error);
        }

        private string? GetErrorMessage()
        {
            IntPtr errorPointer = RuntimeInterop.unitas_runtime_error_message(_runtimeHandle);
            if (errorPointer == IntPtr.Zero)
            {
                return null;
            }

            return Marshal.PtrToStringUTF8(errorPointer);
        }

        private IntPtr AcquirePathBuffer(int size)
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)size);
            if (buffer == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to allocate path buffer in native runtime.");
            }

            return buffer;
        }

        private static byte[] EncodeVectors(Vector3[] values)
        {
            byte[] bytes = new byte[values.Length * VectorBytes];
            for (int index = 0; index < values.Length; index++)
            {
                int offset = index * VectorBytes;
                WriteSingle(bytes, offset, values[index].x);
                WriteSingle(bytes, offset + 4, values[index].y);
                WriteSingle(bytes, offset + 8, values[index].z);
            }

            return bytes;
        }

        private static byte[] EncodeIndices(int[] values)
        {
            byte[] bytes = new byte[values.Length * IndexBytes];
            for (int index = 0; index < values.Length; index++)
            {
                byte[] encoded = BitConverter.GetBytes(values[index]);
                Buffer.BlockCopy(encoded, 0, bytes, index * IndexBytes, IndexBytes);
            }

            return bytes;
        }

        private static void WriteSingle(byte[] bytes, int offset, float value)
        {
            byte[] encoded = BitConverter.GetBytes(value);
            Buffer.BlockCopy(encoded, 0, bytes, offset, IndexBytes);
        }
    }
}
