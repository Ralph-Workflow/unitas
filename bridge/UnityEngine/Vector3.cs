using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Vector3 : IEquatable<Vector3>
    {
        public float x;
        public float y;
        public float z;

        public Vector3(float x, float y, float z)
        {
            this.x = x;
            this.y = y;
            this.z = z;
        }

        public static Vector3 zero => new Vector3(0f, 0f, 0f);
        public static Vector3 one => new Vector3(1f, 1f, 1f);
        public static Vector3 up => new Vector3(0f, 1f, 0f);
        public static Vector3 right => new Vector3(1f, 0f, 0f);
        public static Vector3 forward => new Vector3(0f, 0f, 1f);

        public float magnitude => (float)Math.Sqrt((x * x) + (y * y) + (z * z));

        public Vector3 normalized
        {
            get
            {
                float mag = magnitude;
                if (mag > 1e-5f)
                {
                    return new Vector3(x / mag, y / mag, z / mag);
                }
                return zero;
            }
        }

        public static Vector3 operator +(Vector3 a, Vector3 b) => new Vector3(a.x + b.x, a.y + b.y, a.z + b.z);

        public static Vector3 operator -(Vector3 a) => new Vector3(-a.x, -a.y, -a.z);

        public static Vector3 operator -(Vector3 a, Vector3 b) => new Vector3(a.x - b.x, a.y - b.y, a.z - b.z);

        public static Vector3 operator *(Vector3 a, float d) => new Vector3(a.x * d, a.y * d, a.z * d);

        public static Vector3 operator *(float d, Vector3 a) => new Vector3(a.x * d, a.y * d, a.z * d);

        public static Vector3 operator /(Vector3 a, float d) => new Vector3(a.x / d, a.y / d, a.z / d);

        public static bool operator ==(Vector3 lhs, Vector3 rhs) => lhs.x == rhs.x && lhs.y == rhs.y && lhs.z == rhs.z;

        public static bool operator !=(Vector3 lhs, Vector3 rhs) => !(lhs == rhs);

        public static float Dot(Vector3 lhs, Vector3 rhs) => (lhs.x * rhs.x) + (lhs.y * rhs.y) + (lhs.z * rhs.z);

        public static Vector3 Cross(Vector3 lhs, Vector3 rhs) =>
            new Vector3(
                (lhs.y * rhs.z) - (lhs.z * rhs.y),
                (lhs.z * rhs.x) - (lhs.x * rhs.z),
                (lhs.x * rhs.y) - (lhs.y * rhs.x));

        public static float Distance(Vector3 a, Vector3 b) => (a - b).magnitude;

        public bool Equals(Vector3 other) => x == other.x && y == other.y && z == other.z;

        public override int GetHashCode() => HashCode.Combine(x, y, z);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "({0:F1}, {1:F1}, {2:F1})", x, y, z);
    }
}
