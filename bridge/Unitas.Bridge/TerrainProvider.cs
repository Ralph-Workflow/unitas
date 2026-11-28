using System;
using System.Runtime.InteropServices;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed class RuntimeTerrainProvider : ITerrainProvider
    {
        private const int SizeBufferBytes = 24;
        private const int SampleBufferBytes = 8;

        private readonly ulong _runtimeHandle;
        private readonly double[] _vec3Buffer = new double[3];
        private readonly double[] _singleDoubleBuffer = new double[1];

        public RuntimeTerrainProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public bool EnsureTerrain(ulong entity, int resolution)
        {
            if (_runtimeHandle == 0 || entity == 0 || resolution < 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_terrain_ensure(_runtimeHandle, entity, (uint)resolution) == 0;
        }

        public bool HasTerrain(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_terrain_has(_runtimeHandle, entity) > 0;
        }

        public bool TryGetResolution(ulong entity, out int resolution)
        {
            resolution = 0;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_terrain_get_resolution(_runtimeHandle, entity);
            if (code <= 0)
            {
                return false;
            }
            resolution = code;
            return true;
        }

        public bool TrySetResolution(ulong entity, int resolution)
        {
            if (_runtimeHandle == 0 || entity == 0 || resolution <= 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_terrain_set_resolution(_runtimeHandle, entity, (uint)resolution) == 0;
        }

        public bool TryGetSize(ulong entity, out Vector3 size)
        {
            size = default;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, SizeBufferBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_terrain_get_size(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            Marshal.Copy(buffer, _vec3Buffer, 0, 3);
            size = new Vector3((float)_vec3Buffer[0], (float)_vec3Buffer[1], (float)_vec3Buffer[2]);
            return true;
        }

        public bool TrySetSize(ulong entity, Vector3 size)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_terrain_set_size(_runtimeHandle, entity, size.x, size.y, size.z) == 0;
        }

        public bool TrySetPosition(ulong entity, Vector3 position)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_terrain_set_position(_runtimeHandle, entity, position.x, position.y, position.z) == 0;
        }

        public bool TrySampleHeight(ulong entity, Vector3 worldPosition, out float height)
        {
            height = 0f;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, SampleBufferBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_terrain_sample_height(
                _runtimeHandle,
                entity,
                worldPosition.x,
                worldPosition.y,
                worldPosition.z,
                buffer) != 0)
            {
                return false;
            }
            Marshal.Copy(buffer, _singleDoubleBuffer, 0, 1);
            height = (float)_singleDoubleBuffer[0];
            return true;
        }

        public bool TryGetHeights(ulong entity, int xBase, int yBase, int width, int height, out float[] heights)
        {
            heights = Array.Empty<float>();
            if (_runtimeHandle == 0 || entity == 0 || xBase < 0 || yBase < 0 || width < 0 || height < 0)
            {
                return false;
            }
            int count = width * height;
            if (count == 0)
            {
                return true;
            }
            ulong byteCount = (ulong)count * sizeof(float);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, byteCount);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_terrain_get_heights(
                _runtimeHandle,
                entity,
                (uint)xBase,
                (uint)yBase,
                (uint)width,
                (uint)height,
                buffer) != 0)
            {
                return false;
            }
            float[] flat = new float[count];
            Marshal.Copy(buffer, flat, 0, count);
            heights = flat;
            return true;
        }

        public bool TrySetHeights(ulong entity, int xBase, int yBase, int width, int height, float[] heights)
        {
            if (_runtimeHandle == 0 || entity == 0 || heights == null || xBase < 0 || yBase < 0 || width < 0 || height < 0)
            {
                return false;
            }
            int count = width * height;
            if (count == 0)
            {
                return true;
            }
            if (heights.Length < count)
            {
                return false;
            }
            ulong byteCount = (ulong)count * sizeof(float);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, byteCount);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            Marshal.Copy(heights, 0, buffer, count);
            return RuntimeInterop.unitas_runtime_terrain_set_heights(
                _runtimeHandle,
                entity,
                (uint)xBase,
                (uint)yBase,
                (uint)width,
                (uint)height,
                buffer) == 0;
        }
    }
}
