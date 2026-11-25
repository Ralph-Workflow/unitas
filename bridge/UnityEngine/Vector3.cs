using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Vector3 : IEquatable<Vector3>
    {
        public const float kEpsilon = 0.00001f;
        public const float kEpsilonNormalSqrt = 1e-15f;

        public float x;
        public float y;
        public float z;

        public Vector3(float x, float y, float z)
        {
            this.x = x;
            this.y = y;
            this.z = z;
        }

        public Vector3(float x, float y)
        {
            this.x = x;
            this.y = y;
            z = 0f;
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
                    case 2:
                        return z;
                    default:
                        throw new IndexOutOfRangeException("Invalid Vector3 index!");
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
                    case 2:
                        z = value;
                        break;
                    default:
                        throw new IndexOutOfRangeException("Invalid Vector3 index!");
                }
            }
        }

        public static Vector3 zero => new Vector3(0f, 0f, 0f);
        public static Vector3 one => new Vector3(1f, 1f, 1f);
        public static Vector3 up => new Vector3(0f, 1f, 0f);
        public static Vector3 down => new Vector3(0f, -1f, 0f);
        public static Vector3 left => new Vector3(-1f, 0f, 0f);
        public static Vector3 right => new Vector3(1f, 0f, 0f);
        public static Vector3 forward => new Vector3(0f, 0f, 1f);
        public static Vector3 back => new Vector3(0f, 0f, -1f);
        public static Vector3 positiveInfinity => new Vector3(float.PositiveInfinity, float.PositiveInfinity, float.PositiveInfinity);
        public static Vector3 negativeInfinity => new Vector3(float.NegativeInfinity, float.NegativeInfinity, float.NegativeInfinity);

        public float magnitude => (float)Math.Sqrt((x * x) + (y * y) + (z * z));

        public float sqrMagnitude => (x * x) + (y * y) + (z * z);

        public Vector3 normalized => Normalize(this);

        public void Set(float newX, float newY, float newZ)
        {
            x = newX;
            y = newY;
            z = newZ;
        }

        public static Vector3 Normalize(Vector3 value)
        {
            float mag = Magnitude(value);
            if (mag > kEpsilon)
            {
                return value / mag;
            }
            return zero;
        }

        public void Normalize()
        {
            float mag = Magnitude(this);
            if (mag > kEpsilon)
            {
                this /= mag;
            }
            else
            {
                this = zero;
            }
        }

        public static Vector3 Lerp(Vector3 a, Vector3 b, float t)
        {
            t = Mathf.Clamp01(t);
            return new Vector3(
                a.x + ((b.x - a.x) * t),
                a.y + ((b.y - a.y) * t),
                a.z + ((b.z - a.z) * t));
        }

        public static Vector3 LerpUnclamped(Vector3 a, Vector3 b, float t)
        {
            return new Vector3(
                a.x + ((b.x - a.x) * t),
                a.y + ((b.y - a.y) * t),
                a.z + ((b.z - a.z) * t));
        }

        public static Vector3 MoveTowards(Vector3 current, Vector3 target, float maxDistanceDelta)
        {
            float toVectorX = target.x - current.x;
            float toVectorY = target.y - current.y;
            float toVectorZ = target.z - current.z;
            float sqdist = (toVectorX * toVectorX) + (toVectorY * toVectorY) + (toVectorZ * toVectorZ);
            if (sqdist == 0f || (maxDistanceDelta >= 0f && sqdist <= maxDistanceDelta * maxDistanceDelta))
            {
                return target;
            }
            float dist = (float)Math.Sqrt(sqdist);
            return new Vector3(
                current.x + (toVectorX / dist * maxDistanceDelta),
                current.y + (toVectorY / dist * maxDistanceDelta),
                current.z + (toVectorZ / dist * maxDistanceDelta));
        }

        public static Vector3 Slerp(Vector3 a, Vector3 b, float t) => SlerpUnclamped(a, b, Mathf.Clamp01(t));

        public static Vector3 SlerpUnclamped(Vector3 a, Vector3 b, float t)
        {
            float magA = a.magnitude;
            float magB = b.magnitude;
            if (magA < kEpsilon || magB < kEpsilon)
            {
                return LerpUnclamped(a, b, t);
            }

            Vector3 dirA = a / magA;
            Vector3 dirB = b / magB;
            float lerpedMag = Mathf.LerpUnclamped(magA, magB, t);
            float dot = Mathf.Clamp(Dot(dirA, dirB), -1f, 1f);

            if (dot > 0.999999f)
            {
                return Normalize(LerpUnclamped(dirA, dirB, t)) * lerpedMag;
            }

            if (dot < -0.999999f)
            {
                Vector3 ortho = Cross(right, dirA);
                if (ortho.sqrMagnitude < kEpsilonNormalSqrt)
                {
                    ortho = Cross(up, dirA);
                }
                ortho = Normalize(ortho);
                float angle = Mathf.PI * t;
                Vector3 rotated = (dirA * Mathf.Cos(angle)) + (ortho * Mathf.Sin(angle));
                return rotated * lerpedMag;
            }

            float theta = Mathf.Acos(dot);
            float sinTheta = Mathf.Sin(theta);
            float scaleA = Mathf.Sin((1f - t) * theta) / sinTheta;
            float scaleB = Mathf.Sin(t * theta) / sinTheta;
            return ((dirA * scaleA) + (dirB * scaleB)) * lerpedMag;
        }

        public static Vector3 Scale(Vector3 a, Vector3 b) => new Vector3(a.x * b.x, a.y * b.y, a.z * b.z);

        public void Scale(Vector3 scale)
        {
            x *= scale.x;
            y *= scale.y;
            z *= scale.z;
        }

        public static Vector3 Cross(Vector3 lhs, Vector3 rhs) =>
            new Vector3(
                (lhs.y * rhs.z) - (lhs.z * rhs.y),
                (lhs.z * rhs.x) - (lhs.x * rhs.z),
                (lhs.x * rhs.y) - (lhs.y * rhs.x));

        public static Vector3 Reflect(Vector3 inDirection, Vector3 inNormal)
        {
            float factor = -2f * Dot(inNormal, inDirection);
            return new Vector3(
                (factor * inNormal.x) + inDirection.x,
                (factor * inNormal.y) + inDirection.y,
                (factor * inNormal.z) + inDirection.z);
        }

        public static float Dot(Vector3 lhs, Vector3 rhs) => (lhs.x * rhs.x) + (lhs.y * rhs.y) + (lhs.z * rhs.z);

        public static Vector3 Project(Vector3 vector, Vector3 onNormal)
        {
            float sqrMag = Dot(onNormal, onNormal);
            if (sqrMag < Mathf.Epsilon)
            {
                return zero;
            }
            float dot = Dot(vector, onNormal);
            return new Vector3(
                onNormal.x * dot / sqrMag,
                onNormal.y * dot / sqrMag,
                onNormal.z * dot / sqrMag);
        }

        public static Vector3 ProjectOnPlane(Vector3 vector, Vector3 planeNormal)
        {
            float sqrMag = Dot(planeNormal, planeNormal);
            if (sqrMag < Mathf.Epsilon)
            {
                return vector;
            }
            float dot = Dot(vector, planeNormal);
            return new Vector3(
                vector.x - (planeNormal.x * dot / sqrMag),
                vector.y - (planeNormal.y * dot / sqrMag),
                vector.z - (planeNormal.z * dot / sqrMag));
        }

        public static float Angle(Vector3 from, Vector3 to)
        {
            float denominator = (float)Math.Sqrt(from.sqrMagnitude * to.sqrMagnitude);
            if (denominator < kEpsilonNormalSqrt)
            {
                return 0f;
            }
            float dot = Mathf.Clamp(Dot(from, to) / denominator, -1f, 1f);
            return (float)Math.Acos(dot) * Mathf.Rad2Deg;
        }

        public static float SignedAngle(Vector3 from, Vector3 to, Vector3 axis)
        {
            float unsignedAngle = Angle(from, to);
            float crossX = (from.y * to.z) - (from.z * to.y);
            float crossY = (from.z * to.x) - (from.x * to.z);
            float crossZ = (from.x * to.y) - (from.y * to.x);
            float sign = Mathf.Sign((axis.x * crossX) + (axis.y * crossY) + (axis.z * crossZ));
            return unsignedAngle * sign;
        }

        public static float Distance(Vector3 a, Vector3 b)
        {
            float diffX = a.x - b.x;
            float diffY = a.y - b.y;
            float diffZ = a.z - b.z;
            return (float)Math.Sqrt((diffX * diffX) + (diffY * diffY) + (diffZ * diffZ));
        }

        public static Vector3 ClampMagnitude(Vector3 vector, float maxLength)
        {
            float sqrmag = vector.sqrMagnitude;
            if (sqrmag > maxLength * maxLength)
            {
                float mag = (float)Math.Sqrt(sqrmag);
                float normalizedX = vector.x / mag;
                float normalizedY = vector.y / mag;
                float normalizedZ = vector.z / mag;
                return new Vector3(
                    normalizedX * maxLength,
                    normalizedY * maxLength,
                    normalizedZ * maxLength);
            }
            return vector;
        }

        public static float Magnitude(Vector3 vector) => (float)Math.Sqrt((vector.x * vector.x) + (vector.y * vector.y) + (vector.z * vector.z));

        public static float SqrMagnitude(Vector3 vector) => (vector.x * vector.x) + (vector.y * vector.y) + (vector.z * vector.z);

        public static Vector3 Min(Vector3 lhs, Vector3 rhs) =>
            new Vector3(Mathf.Min(lhs.x, rhs.x), Mathf.Min(lhs.y, rhs.y), Mathf.Min(lhs.z, rhs.z));

        public static Vector3 Max(Vector3 lhs, Vector3 rhs) =>
            new Vector3(Mathf.Max(lhs.x, rhs.x), Mathf.Max(lhs.y, rhs.y), Mathf.Max(lhs.z, rhs.z));

        public static Vector3 operator +(Vector3 a, Vector3 b) => new Vector3(a.x + b.x, a.y + b.y, a.z + b.z);

        public static Vector3 operator -(Vector3 a) => new Vector3(-a.x, -a.y, -a.z);

        public static Vector3 operator -(Vector3 a, Vector3 b) => new Vector3(a.x - b.x, a.y - b.y, a.z - b.z);

        public static Vector3 operator *(Vector3 a, float d) => new Vector3(a.x * d, a.y * d, a.z * d);

        public static Vector3 operator *(float d, Vector3 a) => new Vector3(a.x * d, a.y * d, a.z * d);

        public static Vector3 operator /(Vector3 a, float d) => new Vector3(a.x / d, a.y / d, a.z / d);

        public static bool operator ==(Vector3 lhs, Vector3 rhs)
        {
            float diffX = lhs.x - rhs.x;
            float diffY = lhs.y - rhs.y;
            float diffZ = lhs.z - rhs.z;
            float sqrmag = (diffX * diffX) + (diffY * diffY) + (diffZ * diffZ);
            return sqrmag < 9.99999944E-11f;
        }

        public static bool operator !=(Vector3 lhs, Vector3 rhs) => !(lhs == rhs);

        public bool Equals(Vector3 other) => x == other.x && y == other.y && z == other.z;

        public override bool Equals(object? obj) => obj is Vector3 v && Equals(v);

        public override int GetHashCode() => HashCode.Combine(x, y, z);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "({0:F2}, {1:F2}, {2:F2})", x, y, z);
    }
}
