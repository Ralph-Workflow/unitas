using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace Unitas.Bridge
{
    /// Reads and writes the native sprite behind a managed SpriteRenderer.
    public sealed class RuntimeSpriteProvider : ISpriteProvider
    {
        private const int StateFloats = 12;
        private const int StateBytes = StateFloats * sizeof(float);

        private readonly ulong _runtimeHandle;
        private readonly Func<string> _assetRoot;
        private readonly float[] _state = new float[StateFloats];

        public RuntimeSpriteProvider(ulong runtimeHandle, Func<string> assetRoot)
        {
            _runtimeHandle = runtimeHandle;
            _assetRoot = assetRoot;
        }

        public bool TrySetSprite(ulong entity, string texturePath)
        {
            if (entity == 0)
            {
                return false;
            }

            if (string.IsNullOrEmpty(texturePath))
            {
                return RuntimeInterop.unitas_runtime_sprite_create(_runtimeHandle, entity, 0, 0) == 0;
            }

            byte[] bytes = Encoding.UTF8.GetBytes(ResolveTexturePath(texturePath));
            IntPtr buffer = AcquirePathBuffer(bytes.Length);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return RuntimeInterop.unitas_runtime_sprite_create(
                _runtimeHandle,
                entity,
                (ulong)buffer,
                (ulong)bytes.Length) == 0;
        }

        public bool TrySetSpriteColor(ulong entity, Color color)
        {
            if (entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_sprite_set_color(
                _runtimeHandle,
                entity,
                color.r,
                color.g,
                color.b,
                color.a) == 0;
        }

        public bool TrySetSpriteSorting(ulong entity, int sortingLayerId, int sortingOrder)
        {
            if (entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_sprite_set_sorting(
                _runtimeHandle,
                entity,
                sortingLayerId,
                sortingOrder) == 0;
        }

        public bool TrySetSpriteFlip(ulong entity, bool flipX, bool flipY)
        {
            if (entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_sprite_set_flip(
                _runtimeHandle,
                entity,
                flipX ? (byte)1 : (byte)0,
                flipY ? (byte)1 : (byte)0) == 0;
        }

        public bool TrySetSpriteEnabled(ulong entity, bool enabled)
        {
            if (entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_sprite_set_enabled(
                _runtimeHandle,
                entity,
                enabled ? (byte)1 : (byte)0) == 0;
        }

        public bool TrySetSpriteDrawMode(ulong entity, SpriteDrawMode drawMode)
        {
            if (entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_sprite_set_draw_mode(
                _runtimeHandle,
                entity,
                (uint)drawMode) == 0;
        }

        public bool TryGetSpriteState(ulong entity, out SpriteState state)
        {
            state = null!;
            if (entity == 0)
            {
                return false;
            }

            IntPtr buffer = AcquirePathBuffer(StateBytes);
            if (RuntimeInterop.unitas_runtime_sprite_get_state(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }

            Marshal.Copy(buffer, _state, 0, StateFloats);
            state = new SpriteState(
                new Color(_state[0], _state[1], _state[2], _state[3]),
                (int)_state[4],
                (int)_state[5],
                _state[6] != 0f,
                _state[7] != 0f,
                _state[8] != 0f,
                _state[9] != 0f);
            return true;
        }

        /// The texture path the native runtime reads, resolved against the
        /// project asset folder when the script names it relatively.
        private string ResolveTexturePath(string texturePath)
        {
            if (Path.IsPathRooted(texturePath))
            {
                return Path.GetFullPath(texturePath);
            }

            string root = _assetRoot();
            if (string.IsNullOrEmpty(root))
            {
                return texturePath;
            }

            return Path.GetFullPath(Path.Combine(root, texturePath));
        }

        private IntPtr AcquirePathBuffer(int size)
        {
            IntPtr buffer = (IntPtr)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)size);
            if (buffer == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to allocate path buffer in native runtime.");
            }

            return buffer;
        }
    }
}
