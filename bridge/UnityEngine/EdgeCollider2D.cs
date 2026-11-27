using System;

namespace UnityEngine
{
    public class EdgeCollider2D : Collider2D
    {
        private Vector2[] _points = new Vector2[]
        {
            new Vector2(-0.5f, 0f),
            new Vector2(0.5f, 0f),
        };
        private float _edgeRadius;

        public Vector2[] points
        {
            get => (Vector2[])_points.Clone();
            set
            {
                _points = value == null ? Array.Empty<Vector2>() : (Vector2[])value.Clone();
                ApplyNativeShape();
            }
        }

        public int pointCount => _points.Length;

        public int edgeCount => Math.Max(0, _points.Length - 1);

        public float edgeRadius
        {
            get => _edgeRadius;
            set => _edgeRadius = Math.Max(0f, value);
        }

        public void Reset()
        {
            _points = new Vector2[]
            {
                new Vector2(-0.5f, 0f),
                new Vector2(0.5f, 0f),
            };
            ApplyNativeShape();
        }

        protected override void ApplyNativeShape()
        {
            GameObject? owner = gameObject;
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0 || provider == null)
            {
                return;
            }
            if (_points.Length < 2)
            {
                return;
            }
            Rigidbody2D? attached = attachedRigidbody;
            ulong attachedEntity = attached != null && attached.gameObject != null ? attached.gameObject.NativeEntityId : owner.NativeEntityId;
            provider.TryCreateEdgeCollider2D(
                owner.NativeEntityId,
                attachedEntity,
                _points,
                offset,
                isTrigger,
                (uint)owner.layer);
            ApplyNativeMaterialAndComposite();
        }
    }
}
