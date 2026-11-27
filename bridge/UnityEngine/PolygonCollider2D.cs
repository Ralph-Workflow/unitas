using System;

namespace UnityEngine
{
    public class PolygonCollider2D : Collider2D
    {
        private Vector2[][] _paths = new Vector2[][]
        {
            new Vector2[]
            {
                new Vector2(0f, 1f),
                new Vector2(-0.9510565f, 0.309017f),
                new Vector2(-0.5877852f, -0.8090171f),
                new Vector2(0.5877854f, -0.8090169f),
                new Vector2(0.9510565f, 0.3090171f),
            },
        };

        public Vector2[] points
        {
            get => _paths.Length > 0 ? _paths[0] : Array.Empty<Vector2>();
            set
            {
                Vector2[] copy = value == null ? Array.Empty<Vector2>() : (Vector2[])value.Clone();
                if (_paths.Length == 0)
                {
                    _paths = new Vector2[][] { copy };
                }
                else
                {
                    _paths[0] = copy;
                }
                ApplyNativeShape();
            }
        }

        public int pathCount
        {
            get => _paths.Length;
            set
            {
                int clamped = Math.Max(0, value);
                if (clamped == _paths.Length)
                {
                    return;
                }
                Vector2[][] resized = new Vector2[clamped][];
                int copyCount = Math.Min(clamped, _paths.Length);
                for (int i = 0; i < copyCount; i++)
                {
                    resized[i] = _paths[i];
                }
                for (int i = copyCount; i < clamped; i++)
                {
                    resized[i] = Array.Empty<Vector2>();
                }
                _paths = resized;
                ApplyNativeShape();
            }
        }

        public Vector2[] GetPath(int index)
        {
            if (index < 0 || index >= _paths.Length)
            {
                throw new ArgumentOutOfRangeException(nameof(index));
            }
            return (Vector2[])_paths[index].Clone();
        }

        public void SetPath(int index, Vector2[] points)
        {
            if (index < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(index));
            }
            if (index >= _paths.Length)
            {
                pathCount = index + 1;
            }
            _paths[index] = points == null ? Array.Empty<Vector2>() : (Vector2[])points.Clone();
            ApplyNativeShape();
        }

        public int GetTotalPointCount()
        {
            int count = 0;
            for (int i = 0; i < _paths.Length; i++)
            {
                count += _paths[i].Length;
            }
            return count;
        }

        protected override void ApplyNativeShape()
        {
            GameObject? owner = gameObject;
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0 || provider == null)
            {
                return;
            }
            Vector2[] activePoints = points;
            if (activePoints.Length < 3)
            {
                return;
            }
            Rigidbody2D? attached = attachedRigidbody;
            ulong attachedEntity = attached != null && attached.gameObject != null ? attached.gameObject.NativeEntityId : owner.NativeEntityId;
            provider.TryCreatePolygonCollider2D(
                owner.NativeEntityId,
                attachedEntity,
                activePoints,
                offset,
                isTrigger,
                (uint)owner.layer);
            ApplyNativeMaterialAndComposite();
        }
    }
}
