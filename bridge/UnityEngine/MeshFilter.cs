using System;

namespace UnityEngine
{
    public sealed class MeshFilter : Component
    {
        private Mesh? _sharedMesh;

        public static IRenderProvider? Provider { get; set; }

        public Mesh? sharedMesh
        {
            get => _sharedMesh;
            set
            {
                _sharedMesh = value;
                IRenderProvider? provider = Provider;
                if (provider == null)
                {
                    return;
                }

                MeshRenderer? renderer = GetComponent<MeshRenderer>();
                if (renderer != null)
                {
                    renderer.TryBind();
                }
            }
        }
    }
}
