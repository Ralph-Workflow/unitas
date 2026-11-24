using System;

namespace UnityEngine
{
    public sealed class Mesh : Object
    {
        private Vector3[] _vertices = Array.Empty<Vector3>();
        private Vector3[] _normals = Array.Empty<Vector3>();
        private int[] _triangles = Array.Empty<int>();
        private ulong _nativeMeshHandle;

        public static IRenderProvider? Provider { get; set; }

        public Vector3[] vertices
        {
            get => _vertices;
            set
            {
                _vertices = value ?? Array.Empty<Vector3>();
                PushMesh();
            }
        }

        public Vector3[] normals
        {
            get => _normals;
            set
            {
                _normals = value ?? Array.Empty<Vector3>();
                PushMesh();
            }
        }

        public int[] triangles
        {
            get => _triangles;
            set
            {
                _triangles = value ?? Array.Empty<int>();
                PushMesh();
            }
        }

        public void SetVertices(Vector3[] value)
        {
            vertices = value;
        }

        public void SetNormals(Vector3[] value)
        {
            normals = value;
        }

        public void SetTriangles(int[] value)
        {
            triangles = value;
        }

        internal ulong NativeMeshHandle => EnsureNativeMesh();

        private ulong EnsureNativeMesh()
        {
            if (_nativeMeshHandle != 0UL)
            {
                return _nativeMeshHandle;
            }

            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            _nativeMeshHandle = provider.CreateMesh(_vertices, _normals, _triangles);
            return _nativeMeshHandle;
        }

        private void PushMesh()
        {
            IRenderProvider? provider = Provider;
            if (provider == null || _nativeMeshHandle == 0UL)
            {
                return;
            }

            provider.DestroyMesh(_nativeMeshHandle);
            _nativeMeshHandle = 0UL;
            EnsureNativeMesh();
        }
    }
}
