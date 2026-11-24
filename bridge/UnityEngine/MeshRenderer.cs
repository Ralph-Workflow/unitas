using System;

namespace UnityEngine
{
    public sealed class MeshRenderer : Behaviour
    {
        private Material? _sharedMaterial;
        private ulong _nativeRendererHandle;

        public static IRenderProvider? Provider { get; set; }

        public Material? sharedMaterial
        {
            get => _sharedMaterial;
            set
            {
                _sharedMaterial = value;
                TryBind();
            }
        }

        internal ulong NativeRendererHandle => EnsureNativeRenderer();

        internal void TryBind()
        {
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong rendererHandle = EnsureNativeRenderer();
            if (rendererHandle == 0UL)
            {
                return;
            }

            provider.SetRendererEnabled(rendererHandle, enabled);
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong rendererHandle = EnsureNativeRenderer();
            if (rendererHandle == 0UL)
            {
                return;
            }

            provider.SetRendererEnabled(rendererHandle, isEnabled);
        }

        private ulong EnsureNativeRenderer()
        {
            if (_nativeRendererHandle != 0UL)
            {
                return _nativeRendererHandle;
            }

            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            MeshFilter? filter = GetComponent<MeshFilter>();
            Mesh? mesh = filter != null ? filter.sharedMesh : null;
            ulong meshHandle = mesh != null ? mesh.NativeMeshHandle : 0UL;
            ulong materialHandle = _sharedMaterial != null ? _sharedMaterial.NativeMaterialHandle : 0UL;
            if (meshHandle == 0UL || materialHandle == 0UL)
            {
                return 0UL;
            }

            GameObject? owner = gameObject;
            ulong entityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            _nativeRendererHandle = provider.CreateRenderer(entityId, meshHandle, materialHandle);
            return _nativeRendererHandle;
        }
    }
}
