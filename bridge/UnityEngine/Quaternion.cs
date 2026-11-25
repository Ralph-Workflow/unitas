using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Quaternion : IEquatable<Quaternion>
    {
        public const float kEpsilon = 0.000001f;

        public float x;
        public float y;
        public float z;
        public float w;

        public Quaternion(float x, float y, float z, float w)
        {
            this.x = x;
            this.y = y;
            this.z = z;
            this.w = w;
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
                    case 3:
                        return w;
                    default:
                        throw new IndexOutOfRangeException("Invalid Quaternion index!");
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
                    case 3:
                        w = value;
                        break;
                    default:
                        throw new IndexOutOfRangeException("Invalid Quaternion index!");
                }
            }
        }

        public static Quaternion identity => new Quaternion(0f, 0f, 0f, 1f);

        public Quaternion normalized => Normalize(this);

        public Vector3 eulerAngles
        {
            get => ToEulerAnglesDegrees(this);
            set => this = Euler(value);
        }

        public void Set(float newX, float newY, float newZ, float newW)
        {
            x = newX;
            y = newY;
            z = newZ;
            w = newW;
        }

        public static Quaternion operator *(Quaternion lhs, Quaternion rhs)
        {
            return new Quaternion(
                (lhs.w * rhs.x) + (lhs.x * rhs.w) + (lhs.y * rhs.z) - (lhs.z * rhs.y),
                (lhs.w * rhs.y) + (lhs.y * rhs.w) + (lhs.z * rhs.x) - (lhs.x * rhs.z),
                (lhs.w * rhs.z) + (lhs.z * rhs.w) + (lhs.x * rhs.y) - (lhs.y * rhs.x),
                (lhs.w * rhs.w) - (lhs.x * rhs.x) - (lhs.y * rhs.y) - (lhs.z * rhs.z));
        }

        public static Vector3 operator *(Quaternion rotation, Vector3 point)
        {
            float x2 = rotation.x * 2f;
            float y2 = rotation.y * 2f;
            float z2 = rotation.z * 2f;
            float xx2 = rotation.x * x2;
            float yy2 = rotation.y * y2;
            float zz2 = rotation.z * z2;
            float xy2 = rotation.x * y2;
            float xz2 = rotation.x * z2;
            float yz2 = rotation.y * z2;
            float wx2 = rotation.w * x2;
            float wy2 = rotation.w * y2;
            float wz2 = rotation.w * z2;

            return new Vector3(
                ((1f - (yy2 + zz2)) * point.x) + ((xy2 - wz2) * point.y) + ((xz2 + wy2) * point.z),
                ((xy2 + wz2) * point.x) + ((1f - (xx2 + zz2)) * point.y) + ((yz2 - wx2) * point.z),
                ((xz2 - wy2) * point.x) + ((yz2 + wx2) * point.y) + ((1f - (xx2 + yy2)) * point.z));
        }

        private static bool IsEqualUsingDot(float dot) => dot > 1.0f - kEpsilon;

        public static bool operator ==(Quaternion lhs, Quaternion rhs) => IsEqualUsingDot(Dot(lhs, rhs));

        public static bool operator !=(Quaternion lhs, Quaternion rhs) => !(lhs == rhs);

        public static float Dot(Quaternion a, Quaternion b) => (a.x * b.x) + (a.y * b.y) + (a.z * b.z) + (a.w * b.w);

        public static float Angle(Quaternion a, Quaternion b)
        {
            float dot = Mathf.Min(Mathf.Abs(Dot(a, b)), 1.0f);
            return IsEqualUsingDot(dot) ? 0.0f : Mathf.Acos(dot) * 2.0f * Mathf.Rad2Deg;
        }

        public static Quaternion Euler(float x, float y, float z)
        {
            float halfX = x * Mathf.Deg2Rad * 0.5f;
            float halfY = y * Mathf.Deg2Rad * 0.5f;
            float halfZ = z * Mathf.Deg2Rad * 0.5f;

            float sx = Mathf.Sin(halfX);
            float cx = Mathf.Cos(halfX);
            float sy = Mathf.Sin(halfY);
            float cy = Mathf.Cos(halfY);
            float sz = Mathf.Sin(halfZ);
            float cz = Mathf.Cos(halfZ);

            Quaternion qx = new Quaternion(sx, 0f, 0f, cx);
            Quaternion qy = new Quaternion(0f, sy, 0f, cy);
            Quaternion qz = new Quaternion(0f, 0f, sz, cz);

            return qy * qx * qz;
        }

        public static Quaternion Euler(Vector3 euler) => Euler(euler.x, euler.y, euler.z);

        public static Quaternion AngleAxis(float angle, Vector3 axis)
        {
            Vector3 norm = axis.normalized;
            if (norm.magnitude < 1e-6f)
            {
                return identity;
            }
            float halfAngle = angle * Mathf.Deg2Rad * 0.5f;
            float s = Mathf.Sin(halfAngle);
            float c = Mathf.Cos(halfAngle);
            return new Quaternion(norm.x * s, norm.y * s, norm.z * s, c);
        }

        public void ToAngleAxis(out float angle, out Vector3 axis)
        {
            Quaternion q = Normalize(this);
            if (q.w < 0f)
            {
                q = new Quaternion(-q.x, -q.y, -q.z, -q.w);
            }
            angle = 2.0f * Mathf.Acos(Mathf.Clamp(q.w, -1f, 1f)) * Mathf.Rad2Deg;
            float s = Mathf.Sqrt(Mathf.Max(0f, 1.0f - (q.w * q.w)));
            if (s < 0.0001f)
            {
                axis = new Vector3(1f, 0f, 0f);
            }
            else
            {
                axis = new Vector3(q.x / s, q.y / s, q.z / s);
            }
        }

        public static Quaternion FromToRotation(Vector3 fromDirection, Vector3 toDirection)
        {
            Vector3 from = fromDirection.normalized;
            Vector3 to = toDirection.normalized;
            if (from.magnitude < 1e-6f || to.magnitude < 1e-6f)
            {
                return identity;
            }
            float dot = Vector3.Dot(from, to);
            if (dot >= 1f - 1e-6f)
            {
                return identity;
            }
            if (dot <= -1f + 1e-6f)
            {
                Vector3 ortho = Vector3.Cross(Vector3.right, from);
                if (ortho.magnitude < 1e-5f)
                {
                    ortho = Vector3.Cross(Vector3.up, from);
                }
                ortho = ortho.normalized;
                return new Quaternion(ortho.x, ortho.y, ortho.z, 0f);
            }
            Vector3 cross = Vector3.Cross(from, to);
            Quaternion result = new Quaternion(cross.x, cross.y, cross.z, 1f + dot);
            return Normalize(result);
        }

        public void SetFromToRotation(Vector3 fromDirection, Vector3 toDirection)
        {
            this = FromToRotation(fromDirection, toDirection);
        }

        public static Quaternion LookRotation(Vector3 forward) => LookRotation(forward, Vector3.up);

        public static Quaternion LookRotation(Vector3 forward, Vector3 upwards)
        {
            Vector3 zAxis = forward.normalized;
            if (zAxis.magnitude < 1e-6f)
            {
                return identity;
            }

            Vector3 xAxis = Vector3.Cross(upwards, zAxis).normalized;
            if (xAxis.magnitude < 1e-6f)
            {
                Vector3 fallbackUp = Mathf.Abs(zAxis.y) < 0.999f ? Vector3.up : Vector3.right;
                xAxis = Vector3.Cross(fallbackUp, zAxis).normalized;
            }

            Vector3 yAxis = Vector3.Cross(zAxis, xAxis);

            return FromOrthonormalBasis(xAxis, yAxis, zAxis);
        }

        public void SetLookRotation(Vector3 view) => SetLookRotation(view, Vector3.up);

        public void SetLookRotation(Vector3 view, Vector3 up)
        {
            this = LookRotation(view, up);
        }

        internal static Quaternion FromOrthonormalBasis(Vector3 right, Vector3 up, Vector3 forward)
        {
            float m00 = right.x;
            float m01 = up.x;
            float m02 = forward.x;
            float m10 = right.y;
            float m11 = up.y;
            float m12 = forward.y;
            float m20 = right.z;
            float m21 = up.z;
            float m22 = forward.z;

            float trace = m00 + m11 + m22;
            Quaternion q;
            if (trace > 0f)
            {
                float s = Mathf.Sqrt(trace + 1.0f) * 2f;
                q = new Quaternion(
                    (m21 - m12) / s,
                    (m02 - m20) / s,
                    (m10 - m01) / s,
                    0.25f * s);
            }
            else if (m00 > m11 && m00 > m22)
            {
                float s = Mathf.Sqrt(1.0f + m00 - m11 - m22) * 2f;
                q = new Quaternion(
                    0.25f * s,
                    (m01 + m10) / s,
                    (m02 + m20) / s,
                    (m21 - m12) / s);
            }
            else if (m11 > m22)
            {
                float s = Mathf.Sqrt(1.0f + m11 - m00 - m22) * 2f;
                q = new Quaternion(
                    (m01 + m10) / s,
                    0.25f * s,
                    (m12 + m21) / s,
                    (m02 - m20) / s);
            }
            else
            {
                float s = Mathf.Sqrt(1.0f + m22 - m00 - m11) * 2f;
                q = new Quaternion(
                    (m02 + m20) / s,
                    (m12 + m21) / s,
                    0.25f * s,
                    (m10 - m01) / s);
            }
            return Normalize(q);
        }

        public static Quaternion Slerp(Quaternion a, Quaternion b, float t) => SlerpUnclamped(a, b, Mathf.Clamp01(t));

        public static Quaternion SlerpUnclamped(Quaternion a, Quaternion b, float t)
        {
            Quaternion q1 = Normalize(a);
            Quaternion q2 = Normalize(b);
            float dot = Dot(q1, q2);

            if (dot < 0.0f)
            {
                dot = -dot;
                q2 = new Quaternion(-q2.x, -q2.y, -q2.z, -q2.w);
            }

            if (dot > 0.9995f)
            {
                return Normalize(new Quaternion(
                    q1.x + ((q2.x - q1.x) * t),
                    q1.y + ((q2.y - q1.y) * t),
                    q1.z + ((q2.z - q1.z) * t),
                    q1.w + ((q2.w - q1.w) * t)));
            }

            float theta0 = Mathf.Acos(Mathf.Clamp(dot, -1f, 1f));
            float theta = theta0 * t;
            float sinTheta = Mathf.Sin(theta);
            float sinTheta0 = Mathf.Sin(theta0);

            float s0 = Mathf.Cos(theta) - (dot * sinTheta / sinTheta0);
            float s1 = sinTheta / sinTheta0;

            return new Quaternion(
                (s0 * q1.x) + (s1 * q2.x),
                (s0 * q1.y) + (s1 * q2.y),
                (s0 * q1.z) + (s1 * q2.z),
                (s0 * q1.w) + (s1 * q2.w));
        }

        public static Quaternion Lerp(Quaternion a, Quaternion b, float t) => LerpUnclamped(a, b, Mathf.Clamp01(t));

        public static Quaternion LerpUnclamped(Quaternion a, Quaternion b, float t)
        {
            Quaternion q1 = Normalize(a);
            Quaternion q2 = Normalize(b);
            if (Dot(q1, q2) < 0f)
            {
                q2 = new Quaternion(-q2.x, -q2.y, -q2.z, -q2.w);
            }
            return Normalize(new Quaternion(
                q1.x + ((q2.x - q1.x) * t),
                q1.y + ((q2.y - q1.y) * t),
                q1.z + ((q2.z - q1.z) * t),
                q1.w + ((q2.w - q1.w) * t)));
        }

        public static Quaternion RotateTowards(Quaternion from, Quaternion to, float maxDegreesDelta)
        {
            float angle = Angle(from, to);
            if (angle == 0.0f)
            {
                return to;
            }
            return SlerpUnclamped(from, to, Mathf.Min(1.0f, maxDegreesDelta / angle));
        }

        public static Quaternion Inverse(Quaternion rotation)
        {
            float normSq = Dot(rotation, rotation);
            if (normSq < 1e-12f)
            {
                return identity;
            }
            float inv = 1.0f / normSq;
            return new Quaternion(-rotation.x * inv, -rotation.y * inv, -rotation.z * inv, rotation.w * inv);
        }

        public static Quaternion Normalize(Quaternion q)
        {
            float mag = Mathf.Sqrt(Dot(q, q));
            if (mag < Mathf.Epsilon)
            {
                return identity;
            }
            return new Quaternion(q.x / mag, q.y / mag, q.z / mag, q.w / mag);
        }

        public void Normalize()
        {
            this = Normalize(this);
        }

        private static Vector3 ToEulerAnglesDegrees(Quaternion rotation)
        {
            Quaternion q = Normalize(rotation);
            float xx = q.x * q.x;
            float yy = q.y * q.y;
            float zz = q.z * q.z;

            float m02 = 2f * ((q.x * q.z) + (q.w * q.y));
            float m10 = 2f * ((q.x * q.y) + (q.w * q.z));
            float m11 = 1f - (2f * (xx + zz));
            float m12 = 2f * ((q.y * q.z) - (q.w * q.x));
            float m22 = 1f - (2f * (xx + yy));

            float sinX = Mathf.Clamp(-m12, -1f, 1f);
            float xRad;
            float yRad;
            float zRad;

            if (Mathf.Abs(sinX) < 0.999999f)
            {
                xRad = Mathf.Asin(sinX);
                yRad = Mathf.Atan2(m02, m22);
                zRad = Mathf.Atan2(m10, m11);
            }
            else
            {
                float m01 = 2f * ((q.x * q.y) - (q.w * q.z));
                float m00 = 1f - (2f * (yy + zz));
                xRad = Mathf.Asin(sinX);
                yRad = 0f;
                zRad = Mathf.Atan2(-m01, m00);
            }

            return NormalizeAngles(new Vector3(xRad * Mathf.Rad2Deg, yRad * Mathf.Rad2Deg, zRad * Mathf.Rad2Deg));
        }

        private static Vector3 NormalizeAngles(Vector3 angles)
        {
            return new Vector3(
                NormalizeAngle(angles.x),
                NormalizeAngle(angles.y),
                NormalizeAngle(angles.z));
        }

        private static float NormalizeAngle(float angle)
        {
            float wrapped = Mathf.Repeat(angle, 360f);
            if (wrapped >= 360f - 1e-4f || wrapped < 1e-4f)
            {
                return 0f;
            }
            return wrapped;
        }

        public bool Equals(Quaternion other) =>
            x.Equals(other.x) && y.Equals(other.y) && z.Equals(other.z) && w.Equals(other.w);

        public override bool Equals(object? obj) => obj is Quaternion q && Equals(q);

        public override int GetHashCode() => HashCode.Combine(x, y, z, w);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "({0:F5}, {1:F5}, {2:F5}, {3:F5})", x, y, z, w);
    }
}
