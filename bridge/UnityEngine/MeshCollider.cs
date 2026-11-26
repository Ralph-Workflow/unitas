namespace UnityEngine
{
    public class MeshCollider : Collider
    {
        private Mesh? _sharedMesh;
        private bool _convex;
        private Vector3 _center;

        public Mesh? sharedMesh
        {
            get => _sharedMesh;
            set => _sharedMesh = value;
        }

        public bool convex
        {
            get => _convex;
            set => _convex = value;
        }

        public Vector3 center
        {
            get => _center;
            set => _center = value;
        }
    }
}
