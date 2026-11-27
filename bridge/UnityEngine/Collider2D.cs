namespace UnityEngine
{
    public abstract class Collider2D : Component
    {
        private bool _enabled = true;
        private bool _isTrigger;
        private bool _usedByEffector;
        private bool _usedByComposite;
        private Vector2 _offset;
        private PhysicsMaterial2D? _sharedMaterial;

        public bool enabled
        {
            get => _enabled;
            set => _enabled = value;
        }

        public bool isTrigger
        {
            get => _isTrigger;
            set
            {
                _isTrigger = value;
                ApplyNativeShape();
            }
        }

        public bool usedByEffector
        {
            get => _usedByEffector;
            set => _usedByEffector = value;
        }

        public bool usedByComposite
        {
            get => _usedByComposite;
            set
            {
                _usedByComposite = value;
                ApplyNativeMaterialAndComposite();
            }
        }

        public Vector2 offset
        {
            get => _offset;
            set
            {
                _offset = value;
                ApplyNativeShape();
            }
        }

        public PhysicsMaterial2D? sharedMaterial
        {
            get => _sharedMaterial;
            set
            {
                _sharedMaterial = value;
                ApplyNativeMaterialAndComposite();
            }
        }

        public Rigidbody2D? attachedRigidbody
        {
            get
            {
                GameObject? owner = gameObject;
                return owner == null ? null : owner.GetComponentInParent<Rigidbody2D>();
            }
        }

        public CompositeCollider2D? composite
        {
            get
            {
                if (!_usedByComposite || this is CompositeCollider2D)
                {
                    return null;
                }
                GameObject? owner = gameObject;
                if (owner == null)
                {
                    return null;
                }
                CompositeCollider2D? local = owner.GetComponent<CompositeCollider2D>();
                if (local != null)
                {
                    return local;
                }
                Rigidbody2D? body = attachedRigidbody;
                return body != null && body.gameObject != null
                    ? body.gameObject.GetComponent<CompositeCollider2D>()
                    : owner.GetComponentInParent<CompositeCollider2D>();
            }
        }

        protected virtual void ApplyNativeShape()
        {
        }

        internal void ApplyNativeMaterialAndComposite()
        {
            GameObject? owner = gameObject;
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0 || provider == null)
            {
                return;
            }
            if (_sharedMaterial != null)
            {
                provider.TrySetColliderMaterial2D(
                    owner.NativeEntityId,
                    _sharedMaterial.friction,
                    _sharedMaterial.bounciness);
            }
            if (_usedByComposite && !(this is CompositeCollider2D))
            {
                CompositeCollider2D? targetComposite = composite;
                if (targetComposite != null && targetComposite.gameObject != null && targetComposite.gameObject.NativeEntityId != 0)
                {
                    provider.TrySetCompositeCollider2D(
                        owner.NativeEntityId,
                        targetComposite.gameObject.NativeEntityId,
                        targetComposite.NativeGeometryType);
                }
            }
        }
    }
}
