using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Vector2 : IEquatable<Vector2>
    {
        public const float kEpsilon = 0.00001f;
        public const float kEpsilonNormalSqrt = 1e-15f;

        public float x;
        public float y;

        public Vector2(float x, float y)
        {
            this.x = x;
            this.y = y;
        }

        public float this[int index]
        {
            get
            {
                switch (index)
                {
                    case 0:
                        return x;
                    case 1:
                        return y;
                    default:
                        throw new IndexOutOfRangeException("Invalid Vector2 index!");
                }
            }
            set
            {
                switch (index)
                {
                    case 0:
                        x = value;
                        break;
                    case 1:
                        y = value;
                        break;
                    default:
                        throw new IndexOutOfRangeException("Invalid Vector2 index!");
                }
            }
        }

        public static Vector2 zero => new Vector2(0f, 0f);
        public static Vector2 one => new Vector2(1f, 1f);
        public static Vector2 up => new Vector2(0f, 1f);
        public static Vector2 down => new Vector2(0f, -1f);
        public static Vector2 left => new Vector2(-1f, 0f);
        public static Vector2 right => new Vector2(1f, 0f);
        public static Vector2 positiveInfinity => new Vector2(float.PositiveInfinity, float.PositiveInfinity);
        public static Vector2 negativeInfinity => new Vector2(float.NegativeInfinity, float.NegativeInfinity);

        public float magnitude => (float)Math.Sqrt((x * x) + (y * y));

        public float sqrMagnitude => (x * x) + (y * y);

        public Vector2 normalized
        {
            get
            {
                Vector2 v = new Vector2(x, y);
                v.Normalize();
                return v;
            }
        }

        public void Set(float newX, float newY)
        {
            x = newX;
            y = newY;
        }

        public void Normalize()
        {
            float mag = magnitude;
            if (mag > kEpsilon)
            {
                x /= mag;
                y /= mag;
            }
            else
            {
                x = 0f;
                y = 0f;
            }
        }

        public static Vector2 Lerp(Vector2 a, Vector2 b, float t)
        {
            t = Mathf.Clamp01(t);
            return new Vector2(a.x + ((b.x - a.x) * t), a.y + ((b.y - a.y) * t));
        }

        public static Vector2 LerpUnclamped(Vector2 a, Vector2 b, float t)
        {
            return new Vector2(a.x + ((b.x - a.x) * t), a.y + ((b.y - a.y) * t));
        }

        public static Vector2 MoveTowards(Vector2 current, Vector2 target, float maxDistanceDelta)
        {
            float toVectorX = target.x - current.x;
            float toVectorY = target.y - current.y;
            float sqDist = (toVectorX * toVectorX) + (toVectorY * toVectorY);
            if (sqDist == 0f || (maxDistanceDelta >= 0f && sqDist <= maxDistanceDelta * maxDistanceDelta))
            {
                return target;
            }
            float dist = (float)Math.Sqrt(sqDist);
            return new Vector2(
                current.x + (toVectorX / dist * maxDistanceDelta),
                current.y + (toVectorY / dist * maxDistanceDelta));
        }

        public static Vector2 Scale(Vector2 a, Vector2 b) => new Vector2(a.x * b.x, a.y * b.y);

        public void Scale(Vector2 scale)
        {
            x *= scale.x;
            y *= scale.y;
        }

        public static Vector2 Reflect(Vector2 inDirection, Vector2 inNormal)
        {
            float factor = -2f * Dot(inNormal, inDirection);
            return new Vector2((factor * inNormal.x) + inDirection.x, (factor * inNormal.y) + inDirection.y);
        }

        public static Vector2 Perpendicular(Vector2 inDirection) => new Vector2(-inDirection.y, inDirection.x);

        public static float Dot(Vector2 lhs, Vector2 rhs) => (lhs.x * rhs.x) + (lhs.y * rhs.y);

        public static float Angle(Vector2 from, Vector2 to)
        {
            float denominator = (float)Math.Sqrt(from.sqrMagnitude * to.sqrMagnitude);
            if (denominator < kEpsilonNormalSqrt)
            {
                return 0f;
            }
            float dot = Mathf.Clamp(Dot(from, to) / denominator, -1f, 1f);
            return (float)Math.Acos(dot) * Mathf.Rad2Deg;
        }

        public static float SignedAngle(Vector2 from, Vector2 to)
        {
            float unsignedAngle = Angle(from, to);
            float sign = Mathf.Sign((from.x * to.y) - (from.y * to.x));
            return unsignedAngle * sign;
        }

        public static float Distance(Vector2 a, Vector2 b)
        {
            float diffX = a.x - b.x;
            float diffY = a.y - b.y;
            return (float)Math.Sqrt((diffX * diffX) + (diffY * diffY));
        }

        public static Vector2 ClampMagnitude(Vector2 vector, float maxLength)
        {
            float sqrMag = vector.sqrMagnitude;
            if (sqrMag > maxLength * maxLength)
            {
                float mag = (float)Math.Sqrt(sqrMag);
                float normalizedX = vector.x / mag;
                float normalizedY = vector.y / mag;
                return new Vector2(normalizedX * maxLength, normalizedY * maxLength);
            }
            return vector;
        }

        public static float SqrMagnitude(Vector2 a) => (a.x * a.x) + (a.y * a.y);

        public float SqrMagnitude() => (x * x) + (y * y);

        public static Vector2 Min(Vector2 lhs, Vector2 rhs) => new Vector2(Mathf.Min(lhs.x, rhs.x), Mathf.Min(lhs.y, rhs.y));

        public static Vector2 Max(Vector2 lhs, Vector2 rhs) => new Vector2(Mathf.Max(lhs.x, rhs.x), Mathf.Max(lhs.y, rhs.y));

        public static Vector2 operator +(Vector2 a, Vector2 b) => new Vector2(a.x + b.x, a.y + b.y);

        public static Vector2 operator -(Vector2 a, Vector2 b) => new Vector2(a.x - b.x, a.y - b.y);

        public static Vector2 operator *(Vector2 a, Vector2 b) => new Vector2(a.x * b.x, a.y * b.y);

        public static Vector2 operator /(Vector2 a, Vector2 b) => new Vector2(a.x / b.x, a.y / b.y);

        public static Vector2 operator -(Vector2 a) => new Vector2(-a.x, -a.y);

        public static Vector2 operator *(Vector2 a, float d) => new Vector2(a.x * d, a.y * d);

        public static Vector2 operator *(float d, Vector2 a) => new Vector2(a.x * d, a.y * d);

        public static Vector2 operator /(Vector2 a, float d) => new Vector2(a.x / d, a.y / d);

        public static bool operator ==(Vector2 lhs, Vector2 rhs)
        {
            float diffX = lhs.x - rhs.x;
            float diffY = lhs.y - rhs.y;
            return (diffX * diffX) + (diffY * diffY) < 9.99999944E-11f;
        }

        public static bool operator !=(Vector2 lhs, Vector2 rhs) => !(lhs == rhs);

        public static implicit operator Vector2(Vector3 v) => new Vector2(v.x, v.y);

        public static implicit operator Vector3(Vector2 v) => new Vector3(v.x, v.y, 0f);

        public bool Equals(Vector2 other) => x == other.x && y == other.y;

        public override bool Equals(object? obj) => obj is Vector2 v && Equals(v);

        public override int GetHashCode() => HashCode.Combine(x, y);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "({0:F2}, {1:F2})", x, y);
    }
}
