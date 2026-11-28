using System;
using System.CodeDom.Compiler;
using System.Collections.Generic;

namespace UnityEngine
{
    public sealed class TerrainData : Object
    {
        private const int DefaultHeightmapResolution = 33;
        private static readonly Vector3 DefaultTerrainSize = new Vector3(1000f, 600f, 1000f);

        private readonly List<ulong> _boundEntities = new List<ulong>();
        private int _heightmapResolution = DefaultHeightmapResolution;
        private Vector3 _size = DefaultTerrainSize;
        private float[] _heights = new float[DefaultHeightmapResolution * DefaultHeightmapResolution];

        public TerrainData()
        {
            name = "TerrainData";
        }

        public int heightmapResolution
        {
            get
            {
                if (TryGetPrimaryBoundEntity(out ulong entity)
                    && Terrain.Provider != null
                    && Terrain.Provider.TryGetResolution(entity, out int nativeRes)
                    && nativeRes > 0)
                {
                    if (nativeRes != _heightmapResolution)
                    {
                        _heightmapResolution = nativeRes;
                        _heights = new float[nativeRes * nativeRes];
                    }
                    return nativeRes;
                }
                return _heightmapResolution;
            }
            set
            {
                if (value <= 0)
                {
                    throw new ArgumentOutOfRangeException(nameof(value), "heightmapResolution must be positive.");
                }
                if (_heightmapResolution != value)
                {
                    _heightmapResolution = value;
                    _heights = new float[value * value];
                }
                ITerrainProvider? provider = Terrain.Provider;
                if (provider != null)
                {
                    for (int i = 0; i < _boundEntities.Count; i++)
                    {
                        provider.TrySetResolution(_boundEntities[i], value);
                    }
                }
            }
        }

        public Vector3 size
        {
            get
            {
                if (TryGetPrimaryBoundEntity(out ulong entity)
                    && Terrain.Provider != null
                    && Terrain.Provider.TryGetSize(entity, out Vector3 nativeSize))
                {
                    _size = nativeSize;
                    return nativeSize;
                }
                return _size;
            }
            set
            {
                if (value.x <= 0f || value.y <= 0f || value.z <= 0f)
                {
                    throw new ArgumentOutOfRangeException(nameof(value), "TerrainData.size components must be positive.");
                }
                _size = value;
                ITerrainProvider? provider = Terrain.Provider;
                if (provider != null)
                {
                    for (int i = 0; i < _boundEntities.Count; i++)
                    {
                        provider.TrySetSize(_boundEntities[i], value);
                    }
                }
            }
        }

        [GeneratedCode("UnityEngine", "2022.3")]
        public float[,] GetHeights(int xBase, int yBase, int width, int height)
        {
            int res = heightmapResolution;
            ValidateRegion(xBase, yBase, width, height, res);
            if (TryGetPrimaryBoundEntity(out ulong entity)
                && Terrain.Provider != null
                && Terrain.Provider.TryGetHeights(entity, xBase, yBase, width, height, out float[] nativeHeights))
            {
                CopyIntoLocal(xBase, yBase, width, height, nativeHeights);
                float[,] nativeResult = new float[height, width];
                for (int y = 0; y < height; y++)
                {
                    int rowOffset = y * width;
                    for (int x = 0; x < width; x++)
                    {
                        nativeResult[y, x] = nativeHeights[rowOffset + x];
                    }
                }
                return nativeResult;
            }
            float[,] result = new float[height, width];
            for (int y = 0; y < height; y++)
            {
                int localRow = (yBase + y) * _heightmapResolution;
                for (int x = 0; x < width; x++)
                {
                    result[y, x] = _heights[localRow + xBase + x];
                }
            }
            return result;
        }

        [GeneratedCode("UnityEngine", "2022.3")]
        public void SetHeights(int xBase, int yBase, float[,] heights)
        {
            if (heights == null)
            {
                throw new ArgumentNullException(nameof(heights));
            }
            int height = heights.GetLength(0);
            int width = heights.GetLength(1);
            int res = heightmapResolution;
            ValidateRegion(xBase, yBase, width, height, res);
            float[] clamped = new float[height * width];
            for (int y = 0; y < height; y++)
            {
                int blockRow = y * width;
                int localRow = (yBase + y) * _heightmapResolution;
                for (int x = 0; x < width; x++)
                {
                    float sample = heights[y, x];
                    if (sample < 0f)
                    {
                        sample = 0f;
                    }
                    else if (sample > 1f)
                    {
                        sample = 1f;
                    }
                    _heights[localRow + xBase + x] = sample;
                    clamped[blockRow + x] = sample;
                }
            }
            ITerrainProvider? provider = Terrain.Provider;
            if (provider != null)
            {
                for (int i = 0; i < _boundEntities.Count; i++)
                {
                    provider.TrySetHeights(_boundEntities[i], xBase, yBase, width, height, clamped);
                }
            }
        }

        public float GetHeight(int x, int y)
        {
            int res = heightmapResolution;
            if (x < 0 || x >= res)
            {
                throw new ArgumentOutOfRangeException(nameof(x));
            }
            if (y < 0 || y >= res)
            {
                throw new ArgumentOutOfRangeException(nameof(y));
            }
            if (TryGetPrimaryBoundEntity(out ulong entity)
                && Terrain.Provider != null
                && Terrain.Provider.TryGetHeights(entity, x, y, 1, 1, out float[] sample)
                && sample.Length > 0)
            {
                _heights[y * _heightmapResolution + x] = sample[0];
                return sample[0] * size.y;
            }
            return _heights[y * _heightmapResolution + x] * size.y;
        }

        public float GetInterpolatedHeight(float x, float y)
        {
            int res = heightmapResolution;
            if (res <= 1)
            {
                return GetHeight(0, 0);
            }
            float u = Mathf.Clamp01(x) * (res - 1);
            float v = Mathf.Clamp01(y) * (res - 1);
            int x0 = (int)MathF.Floor(u);
            int y0 = (int)MathF.Floor(v);
            int x1 = Math.Min(x0 + 1, res - 1);
            int y1 = Math.Min(y0 + 1, res - 1);
            float tx = u - x0;
            float ty = v - y0;
            float h00 = GetHeight(x0, y0);
            float h10 = GetHeight(x1, y0);
            float h01 = GetHeight(x0, y1);
            float h11 = GetHeight(x1, y1);
            float lower = h00 + (h10 - h00) * tx;
            float upper = h01 + (h11 - h01) * tx;
            return lower + (upper - lower) * ty;
        }

        internal void BindEntity(ulong entity, bool pullFromNative)
        {
            if (entity == 0)
            {
                return;
            }
            if (!_boundEntities.Contains(entity))
            {
                _boundEntities.Add(entity);
            }
            ITerrainProvider? provider = Terrain.Provider;
            if (provider == null)
            {
                return;
            }
            if (pullFromNative && provider.HasTerrain(entity))
            {
                if (provider.TryGetResolution(entity, out int nativeRes) && nativeRes > 0)
                {
                    _heightmapResolution = nativeRes;
                    _heights = new float[nativeRes * nativeRes];
                }
                if (provider.TryGetSize(entity, out Vector3 nativeSize))
                {
                    _size = nativeSize;
                }
                if (provider.TryGetHeights(entity, 0, 0, _heightmapResolution, _heightmapResolution, out float[] all)
                    && all.Length == _heightmapResolution * _heightmapResolution)
                {
                    _heights = all;
                }
                return;
            }
            provider.EnsureTerrain(entity, _heightmapResolution);
            provider.TrySetResolution(entity, _heightmapResolution);
            provider.TrySetSize(entity, _size);
            provider.TrySetHeights(entity, 0, 0, _heightmapResolution, _heightmapResolution, _heights);
        }

        internal void UnbindEntity(ulong entity)
        {
            _boundEntities.Remove(entity);
        }

        private bool TryGetPrimaryBoundEntity(out ulong entity)
        {
            for (int i = 0; i < _boundEntities.Count; i++)
            {
                if (_boundEntities[i] != 0)
                {
                    entity = _boundEntities[i];
                    return true;
                }
            }
            entity = 0;
            return false;
        }

        private void CopyIntoLocal(int xBase, int yBase, int width, int height, float[] block)
        {
            int res = _heightmapResolution;
            for (int y = 0; y < height && yBase + y < res; y++)
            {
                int blockRow = y * width;
                int localRow = (yBase + y) * res;
                for (int x = 0; x < width && xBase + x < res; x++)
                {
                    int idx = blockRow + x;
                    if (idx < block.Length)
                    {
                        _heights[localRow + xBase + x] = block[idx];
                    }
                }
            }
        }

        private static void ValidateRegion(int xBase, int yBase, int width, int height, int resolution)
        {
            if (xBase < 0 || yBase < 0 || width < 0 || height < 0)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(xBase),
                    "Heightmap region coordinates and dimensions must be non-negative.");
            }
            if (xBase + width > resolution || yBase + height > resolution)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(width),
                    "Heightmap region exceeds heightmapResolution.");
            }
        }
    }
}
