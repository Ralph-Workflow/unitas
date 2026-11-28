using System;

namespace UnityEngine
{
    public interface ITerrainProvider
    {
        bool EnsureTerrain(ulong entity, int resolution);

        bool HasTerrain(ulong entity);

        bool TryGetResolution(ulong entity, out int resolution);

        bool TrySetResolution(ulong entity, int resolution);

        bool TryGetSize(ulong entity, out Vector3 size);

        bool TrySetSize(ulong entity, Vector3 size);

        bool TrySetPosition(ulong entity, Vector3 position);

        bool TrySampleHeight(ulong entity, Vector3 worldPosition, out float height);

        bool TryGetHeights(ulong entity, int xBase, int yBase, int width, int height, out float[] heights);

        bool TrySetHeights(ulong entity, int xBase, int yBase, int width, int height, float[] heights);
    }

    public sealed class Terrain : Behaviour
    {
        private TerrainData? _terrainData;
        private bool _boundFromNative;

        public static ITerrainProvider? Provider { get; set; }

        public static Terrain? activeTerrain
        {
            get
            {
                Terrain[] terrains = activeTerrains;
                return terrains.Length > 0 ? terrains[0] : null;
            }
        }

        public static Terrain[] activeTerrains => FindObjectsOfType<Terrain>(false);

        internal static void BindNativeTerrain(GameObject owner)
        {
            ITerrainProvider? provider = Provider;
            ulong entity = owner.NativeEntityId;
            if (provider == null || entity == 0 || owner.GetComponent<Terrain>() != null)
            {
                return;
            }
            if (!provider.HasTerrain(entity))
            {
                return;
            }
            Terrain terrain = new Terrain();
            owner.AttachComponent(terrain, typeof(Terrain));
            TerrainData data = new TerrainData();
            data.BindEntity(entity, true);
            terrain._terrainData = data;
            terrain._boundFromNative = true;
        }

        public TerrainData? terrainData
        {
            get
            {
                EnsureNativeTerrainDataIfAvailable();
                return _terrainData;
            }
            set
            {
                if (ReferenceEquals(_terrainData, value))
                {
                    return;
                }
                if (_terrainData != null && TryGetOwnerEntity(out ulong prevEntity))
                {
                    _terrainData.UnbindEntity(prevEntity);
                }
                _terrainData = value;
                _boundFromNative = false;
                if (value != null && TryGetOwnerEntity(out ulong entity))
                {
                    value.BindEntity(entity, false);
                    SyncNativePosition(entity, value.size);
                }
            }
        }

        public float SampleHeight(Vector3 worldPosition)
        {
            EnsureNativeTerrainDataIfAvailable();
            TerrainData? data = _terrainData;
            if (data == null)
            {
                return 0f;
            }
            if (TryGetOwnerEntity(out ulong entity) && Provider != null)
            {
                if (!_boundFromNative && !Provider.HasTerrain(entity))
                {
                    data.BindEntity(entity, false);
                }
                SyncNativePosition(entity, data.size);
                if (Provider.TrySampleHeight(entity, worldPosition, out float nativeSampled))
                {
                    return nativeSampled;
                }
            }
            Vector3 origin = transform.position;
            Vector3 terrainSize = data.size;
            float nx = terrainSize.x > 0f ? (worldPosition.x - origin.x) / terrainSize.x : 0f;
            float nz = terrainSize.z > 0f ? (worldPosition.z - origin.z) / terrainSize.z : 0f;
            return data.GetInterpolatedHeight(nx, nz);
        }

        private void EnsureNativeTerrainDataIfAvailable()
        {
            if (_terrainData != null)
            {
                if (TryGetOwnerEntity(out ulong boundEntity))
                {
                    _terrainData.BindEntity(boundEntity, _boundFromNative);
                }
                return;
            }
            if (TryGetOwnerEntity(out ulong entity) && Provider != null && Provider.HasTerrain(entity))
            {
                TerrainData nativeData = new TerrainData();
                nativeData.BindEntity(entity, true);
                _terrainData = nativeData;
                _boundFromNative = true;
            }
        }

        private void SyncNativePosition(ulong entity, Vector3 terrainSize)
        {
            ITerrainProvider? provider = Provider;
            if (provider == null || entity == 0)
            {
                return;
            }
            Vector3 origin = transform.position;
            Vector3 center = new Vector3(
                origin.x + terrainSize.x * 0.5f,
                origin.y,
                origin.z + terrainSize.z * 0.5f);
            provider.TrySetPosition(entity, center);
        }

        private bool TryGetOwnerEntity(out ulong entity)
        {
            entity = 0;
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return false;
            }
            entity = owner.NativeEntityId;
            return entity != 0;
        }
    }
}
