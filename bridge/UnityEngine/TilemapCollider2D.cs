namespace UnityEngine
{
    public class TilemapCollider2D : Collider2D
    {
        private string _tilemapAssetPath = string.Empty;
        private uint _maximumTileChangeCount = 1000u;
        private float _extrusionFactor;

        public string tilemapAssetPath
        {
            get => _tilemapAssetPath;
            set
            {
                _tilemapAssetPath = value ?? string.Empty;
                ApplyNativeShape();
            }
        }

        public uint maximumTileChangeCount
        {
            get => _maximumTileChangeCount;
            set => _maximumTileChangeCount = value;
        }

        public float extrusionFactor
        {
            get => _extrusionFactor;
            set => _extrusionFactor = value;
        }

        public void ProcessTilemapChanges()
        {
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
            if (!string.IsNullOrEmpty(_tilemapAssetPath))
            {
                CompositeCollider2D? targetComposite = usedByComposite ? composite : null;
                ulong compositeEntity = targetComposite != null && targetComposite.gameObject != null
                    ? targetComposite.gameObject.NativeEntityId
                    : 0UL;
                provider.TryBuildTilemapCollider2D(owner.NativeEntityId, compositeEntity, _tilemapAssetPath);
            }
            ApplyNativeMaterialAndComposite();
        }
    }
}
