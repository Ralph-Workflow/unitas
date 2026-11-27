namespace UnityEngine
{
    public class CompositeCollider2D : Collider2D
    {
        public enum GeometryType
        {
            Outlines = 0,
            Polygons = 1,
        }

        public enum GenerationType
        {
            Synchronous = 0,
            Manual = 1,
        }

        private GeometryType _geometryType = GeometryType.Outlines;
        private GenerationType _generationType = GenerationType.Synchronous;
        private float _vertexDistance = 0.00005f;
        private float _offsetDistance = 0.000005f;
        private float _edgeRadius;

        public GeometryType geometryType
        {
            get => _geometryType;
            set
            {
                _geometryType = value;
                ApplyNativeShape();
            }
        }

        public GenerationType generationType
        {
            get => _generationType;
            set => _generationType = value;
        }

        public float vertexDistance
        {
            get => _vertexDistance;
            set => _vertexDistance = value;
        }

        public float offsetDistance
        {
            get => _offsetDistance;
            set => _offsetDistance = value;
        }

        public float edgeRadius
        {
            get => _edgeRadius;
            set => _edgeRadius = value;
        }

        public void GenerateGeometry()
        {
            ApplyNativeShape();
        }

        internal uint NativeGeometryType => _geometryType == GeometryType.Polygons ? 0u : 1u;

        protected override void ApplyNativeShape()
        {
            GameObject? owner = gameObject;
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0 || provider == null)
            {
                return;
            }
            provider.TrySetCompositeCollider2D(owner.NativeEntityId, owner.NativeEntityId, NativeGeometryType);
            ApplyNativeMaterialAndComposite();
        }
    }
}
