using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Bounds : IEquatable<Bounds>
    {
        private Vector3 _center;
        private Vector3 _extents;

        public Bounds(Vector3 center, Vector3 size)
        {
            _center = center;
            _extents = size * 0.5f;
        }

        public Vector3 center
        {
            get => _center;
            set => _center = value;
        }

        public Vector3 size
        {
            get => _extents * 2f;
            set => _extents = value * 0.5f;
        }

        public Vector3 extents
        {
            get => _extents;
            set => _extents = value;
        }

        public Vector3 min
        {
            get => center - extents;
            set => SetMinMax(value, max);
        }

        public Vector3 max
        {
            get => center + extents;
            set => SetMinMax(min, value);
        }

        public void SetMinMax(Vector3 min, Vector3 max)
        {
            extents = (max - min) * 0.5f;
            center = min + extents;
        }

        public void Encapsulate(Vector3 point)
        {
            Vector3 currentMin = min;
            Vector3 currentMax = max;
            SetMinMax(
                new Vector3(Mathf.Min(currentMin.x, point.x), Mathf.Min(currentMin.y, point.y), Mathf.Min(currentMin.z, point.z)),
                new Vector3(Mathf.Max(currentMax.x, point.x), Mathf.Max(currentMax.y, point.y), Mathf.Max(currentMax.z, point.z)));
        }

        public void Encapsulate(Bounds bounds)
        {
            Encapsulate(bounds.center - bounds.extents);
            Encapsulate(bounds.center + bounds.extents);
        }

        public void Expand(float amount)
        {
            amount *= 0.5f;
            extents += new Vector3(amount, amount, amount);
        }

        public void Expand(Vector3 amount)
        {
            extents += amount * 0.5f;
        }

        public bool Intersects(Bounds bounds)
        {
            return min.x <= bounds.max.x && max.x >= bounds.min.x &&
                   min.y <= bounds.max.y && max.y >= bounds.min.y &&
                   min.z <= bounds.max.z && max.z >= bounds.min.z;
        }

        public bool Contains(Vector3 point)
        {
            Vector3 currentMin = min;
            Vector3 currentMax = max;
            return point.x >= currentMin.x && point.x <= currentMax.x &&
                   point.y >= currentMin.y && point.y <= currentMax.y &&
                   point.z >= currentMin.z && point.z <= currentMax.z;
        }

        public float SqrDistance(Vector3 point)
        {
            Vector3 closest = ClosestPoint(point);
            Vector3 diff = point - closest;
            return (diff.x * diff.x) + (diff.y * diff.y) + (diff.z * diff.z);
        }

        public Vector3 ClosestPoint(Vector3 point)
        {
            Vector3 currentMin = min;
            Vector3 currentMax = max;
            return new Vector3(
                Mathf.Clamp(point.x, currentMin.x, currentMax.x),
                Mathf.Clamp(point.y, currentMin.y, currentMax.y),
                Mathf.Clamp(point.z, currentMin.z, currentMax.z));
        }

        public bool IntersectRay(Ray ray)
        {
            return IntersectRay(ray, out _);
        }

        public bool IntersectRay(Ray ray, out float distance)
        {
            distance = 0f;
            float tMin = 0f;
            float tMax = float.PositiveInfinity;
            Vector3 currentMin = min;
            Vector3 currentMax = max;

            if (!IntersectAxis(ray.origin.x, ray.direction.x, currentMin.x, currentMax.x, ref tMin, ref tMax))
            {
                return false;
            }
            if (!IntersectAxis(ray.origin.y, ray.direction.y, currentMin.y, currentMax.y, ref tMin, ref tMax))
            {
                return false;
            }
            if (!IntersectAxis(ray.origin.z, ray.direction.z, currentMin.z, currentMax.z, ref tMin, ref tMax))
            {
                return false;
            }

            distance = tMin;
            return true;
        }

        private static bool IntersectAxis(float origin, float dir, float minVal, float maxVal, ref float tMin, ref float tMax)
        {
            if (Mathf.Abs(dir) < 1e-7f)
            {
                return origin >= minVal && origin <= maxVal;
            }

            float invDir = 1f / dir;
            float t1 = (minVal - origin) * invDir;
            float t2 = (maxVal - origin) * invDir;
            if (t1 > t2)
            {
                float temp = t1;
                t1 = t2;
                t2 = temp;
            }
            if (t1 > tMin)
            {
                tMin = t1;
            }
            if (t2 < tMax)
            {
                tMax = t2;
            }
            return tMin <= tMax && tMax >= 0f;
        }

        public static bool operator ==(Bounds lhs, Bounds rhs) =>
            lhs.center == rhs.center && lhs.extents == rhs.extents;

        public static bool operator !=(Bounds lhs, Bounds rhs) => !(lhs == rhs);

        public bool Equals(Bounds other) => center.Equals(other.center) && extents.Equals(other.extents);

        public override bool Equals(object? obj) => obj is Bounds b && Equals(b);

        public override int GetHashCode() => HashCode.Combine(center, extents);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "Center: {0}, Extents: {1}", _center, _extents);
    }
}
