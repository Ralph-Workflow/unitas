using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Matrix4x4 : IEquatable<Matrix4x4>
    {
        public float m00;
        public float m10;
        public float m20;
        public float m30;

        public float m01;
        public float m11;
        public float m21;
        public float m31;

        public float m02;
        public float m12;
        public float m22;
        public float m32;

        public float m03;
        public float m13;
        public float m23;
        public float m33;

        public Matrix4x4(Vector4 column0, Vector4 column1, Vector4 column2, Vector4 column3)
        {
            m00 = column0.x;
            m10 = column0.y;
            m20 = column0.z;
            m30 = column0.w;

            m01 = column1.x;
            m11 = column1.y;
            m21 = column1.z;
            m31 = column1.w;

            m02 = column2.x;
            m12 = column2.y;
            m22 = column2.z;
            m32 = column2.w;

            m03 = column3.x;
            m13 = column3.y;
            m23 = column3.z;
            m33 = column3.w;
        }

        public float this[int row, int column]
        {
            get => this[row + (column * 4)];
            set => this[row + (column * 4)] = value;
        }

        public float this[int index]
        {
            get
            {
                switch (index)
                {
                    case 0: return m00;
                    case 1: return m10;
                    case 2: return m20;
                    case 3: return m30;
                    case 4: return m01;
                    case 5: return m11;
                    case 6: return m21;
                    case 7: return m31;
                    case 8: return m02;
                    case 9: return m12;
                    case 10: return m22;
                    case 11: return m32;
                    case 12: return m03;
                    case 13: return m13;
                    case 14: return m23;
                    case 15: return m33;
                    default:
                        throw new IndexOutOfRangeException("Invalid matrix index!");
                }
            }
            set
            {
                switch (index)
                {
                    case 0: m00 = value; break;
                    case 1: m10 = value; break;
                    case 2: m20 = value; break;
                    case 3: m30 = value; break;
                    case 4: m01 = value; break;
                    case 5: m11 = value; break;
                    case 6: m21 = value; break;
                    case 7: m31 = value; break;
                    case 8: m02 = value; break;
                    case 9: m12 = value; break;
                    case 10: m22 = value; break;
                    case 11: m32 = value; break;
                    case 12: m03 = value; break;
                    case 13: m13 = value; break;
                    case 14: m23 = value; break;
                    case 15: m33 = value; break;
                    default:
                        throw new IndexOutOfRangeException("Invalid matrix index!");
                }
            }
        }

        public static Matrix4x4 zero => new Matrix4x4(
            new Vector4(0f, 0f, 0f, 0f),
            new Vector4(0f, 0f, 0f, 0f),
            new Vector4(0f, 0f, 0f, 0f),
            new Vector4(0f, 0f, 0f, 0f));

        public static Matrix4x4 identity => new Matrix4x4(
            new Vector4(1f, 0f, 0f, 0f),
            new Vector4(0f, 1f, 0f, 0f),
            new Vector4(0f, 0f, 1f, 0f),
            new Vector4(0f, 0f, 0f, 1f));

        public Matrix4x4 inverse => Inverse(this);

        public Matrix4x4 transpose => Transpose(this);

        public float determinant => Determinant(this);

        public bool isIdentity =>
            Mathf.Abs(m00 - 1f) < 1e-5f && Mathf.Abs(m10) < 1e-5f && Mathf.Abs(m20) < 1e-5f && Mathf.Abs(m30) < 1e-5f &&
            Mathf.Abs(m01) < 1e-5f && Mathf.Abs(m11 - 1f) < 1e-5f && Mathf.Abs(m21) < 1e-5f && Mathf.Abs(m31) < 1e-5f &&
            Mathf.Abs(m02) < 1e-5f && Mathf.Abs(m12) < 1e-5f && Mathf.Abs(m22 - 1f) < 1e-5f && Mathf.Abs(m32) < 1e-5f &&
            Mathf.Abs(m03) < 1e-5f && Mathf.Abs(m13) < 1e-5f && Mathf.Abs(m23) < 1e-5f && Mathf.Abs(m33 - 1f) < 1e-5f;

        public Quaternion rotation
        {
            get
            {
                Vector3 col0 = new Vector3(m00, m10, m20);
                Vector3 col1 = new Vector3(m01, m11, m21);
                Vector3 col2 = new Vector3(m02, m12, m22);

                float sx = col0.magnitude;
                float sy = col1.magnitude;
                float sz = col2.magnitude;
                if (sx < 1e-6f || sy < 1e-6f || sz < 1e-6f)
                {
                    return Quaternion.identity;
                }

                float det3 = Vector3.Dot(col0, Vector3.Cross(col1, col2));
                if (det3 < 0f)
                {
                    sx = -sx;
                }

                Vector3 r0 = col0 / sx;
                Vector3 r1 = col1 / sy;
                Vector3 r2 = col2 / sz;
                return Quaternion.FromOrthonormalBasis(r0, r1, r2);
            }
        }

        public Vector3 lossyScale
        {
            get
            {
                Vector3 col0 = new Vector3(m00, m10, m20);
                Vector3 col1 = new Vector3(m01, m11, m21);
                Vector3 col2 = new Vector3(m02, m12, m22);

                float sx = col0.magnitude;
                float sy = col1.magnitude;
                float sz = col2.magnitude;

                float det3 = Vector3.Dot(col0, Vector3.Cross(col1, col2));
                if (det3 < 0f)
                {
                    sx = -sx;
                }

                return new Vector3(sx, sy, sz);
            }
        }

        public Vector3 GetPosition() => new Vector3(m03, m13, m23);

        public Vector4 GetColumn(int index)
        {
            switch (index)
            {
                case 0: return new Vector4(m00, m10, m20, m30);
                case 1: return new Vector4(m01, m11, m21, m31);
                case 2: return new Vector4(m02, m12, m22, m32);
                case 3: return new Vector4(m03, m13, m23, m33);
                default:
                    throw new IndexOutOfRangeException("Invalid column index!");
            }
        }

        public Vector4 GetRow(int index)
        {
            switch (index)
            {
                case 0: return new Vector4(m00, m01, m02, m03);
                case 1: return new Vector4(m10, m11, m12, m13);
                case 2: return new Vector4(m20, m21, m22, m23);
                case 3: return new Vector4(m30, m31, m32, m33);
                default:
                    throw new IndexOutOfRangeException("Invalid row index!");
            }
        }

        public void SetColumn(int index, Vector4 column)
        {
            this[0, index] = column.x;
            this[1, index] = column.y;
            this[2, index] = column.z;
            this[3, index] = column.w;
        }

        public void SetRow(int index, Vector4 row)
        {
            this[index, 0] = row.x;
            this[index, 1] = row.y;
            this[index, 2] = row.z;
            this[index, 3] = row.w;
        }

        public Vector3 MultiplyPoint(Vector3 point)
        {
            Vector3 res;
            res.x = (m00 * point.x) + (m01 * point.y) + (m02 * point.z) + m03;
            res.y = (m10 * point.x) + (m11 * point.y) + (m12 * point.z) + m13;
            res.z = (m20 * point.x) + (m21 * point.y) + (m22 * point.z) + m23;
            float w = (m30 * point.x) + (m31 * point.y) + (m32 * point.z) + m33;
            w = 1f / w;
            res.x *= w;
            res.y *= w;
            res.z *= w;
            return res;
        }

        public Vector3 MultiplyPoint3x4(Vector3 point)
        {
            Vector3 res;
            res.x = (m00 * point.x) + (m01 * point.y) + (m02 * point.z) + m03;
            res.y = (m10 * point.x) + (m11 * point.y) + (m12 * point.z) + m13;
            res.z = (m20 * point.x) + (m21 * point.y) + (m22 * point.z) + m23;
            return res;
        }

        public Vector3 MultiplyVector(Vector3 vector)
        {
            Vector3 res;
            res.x = (m00 * vector.x) + (m01 * vector.y) + (m02 * vector.z);
            res.y = (m10 * vector.x) + (m11 * vector.y) + (m12 * vector.z);
            res.z = (m20 * vector.x) + (m21 * vector.y) + (m22 * vector.z);
            return res;
        }

        public static Matrix4x4 Scale(Vector3 vector)
        {
            Matrix4x4 m = zero;
            m.m00 = vector.x;
            m.m11 = vector.y;
            m.m22 = vector.z;
            m.m33 = 1f;
            return m;
        }

        public static Matrix4x4 Translate(Vector3 vector)
        {
            Matrix4x4 m = identity;
            m.m03 = vector.x;
            m.m13 = vector.y;
            m.m23 = vector.z;
            return m;
        }

        public static Matrix4x4 Rotate(Quaternion q)
        {
            float x = q.x * 2.0f;
            float y = q.y * 2.0f;
            float z = q.z * 2.0f;
            float xx = q.x * x;
            float yy = q.y * y;
            float zz = q.z * z;
            float xy = q.x * y;
            float xz = q.x * z;
            float yz = q.y * z;
            float wx = q.w * x;
            float wy = q.w * y;
            float wz = q.w * z;

            Matrix4x4 m;
            m.m00 = 1.0f - (yy + zz);
            m.m10 = xy + wz;
            m.m20 = xz - wy;
            m.m30 = 0.0f;

            m.m01 = xy - wz;
            m.m11 = 1.0f - (xx + zz);
            m.m21 = yz + wx;
            m.m31 = 0.0f;

            m.m02 = xz + wy;
            m.m12 = yz - wx;
            m.m22 = 1.0f - (xx + yy);
            m.m32 = 0.0f;

            m.m03 = 0.0f;
            m.m13 = 0.0f;
            m.m23 = 0.0f;
            m.m33 = 1.0f;
            return m;
        }

        public static Matrix4x4 TRS(Vector3 pos, Quaternion q, Vector3 s)
        {
            Matrix4x4 m = Rotate(q);

            m.m00 *= s.x;
            m.m10 *= s.x;
            m.m20 *= s.x;

            m.m01 *= s.y;
            m.m11 *= s.y;
            m.m21 *= s.y;

            m.m02 *= s.z;
            m.m12 *= s.z;
            m.m22 *= s.z;

            m.m03 = pos.x;
            m.m13 = pos.y;
            m.m23 = pos.z;
            m.m33 = 1.0f;
            return m;
        }

        public void SetTRS(Vector3 pos, Quaternion q, Vector3 s)
        {
            this = TRS(pos, q, s);
        }

        public bool ValidTRS()
        {
            return m30 == 0f && m31 == 0f && m32 == 0f && Mathf.Abs(m33 - 1f) < 1e-5f;
        }

        public static Matrix4x4 Transpose(Matrix4x4 m)
        {
            return new Matrix4x4(
                new Vector4(m.m00, m.m01, m.m02, m.m03),
                new Vector4(m.m10, m.m11, m.m12, m.m13),
                new Vector4(m.m20, m.m21, m.m22, m.m23),
                new Vector4(m.m30, m.m31, m.m32, m.m33));
        }

        public static float Determinant(Matrix4x4 m)
        {
            float s0 = (m.m00 * m.m11) - (m.m10 * m.m01);
            float s1 = (m.m00 * m.m12) - (m.m10 * m.m02);
            float s2 = (m.m00 * m.m13) - (m.m10 * m.m03);
            float s3 = (m.m01 * m.m12) - (m.m11 * m.m02);
            float s4 = (m.m01 * m.m13) - (m.m11 * m.m03);
            float s5 = (m.m02 * m.m13) - (m.m12 * m.m03);

            float c5 = (m.m22 * m.m33) - (m.m32 * m.m23);
            float c4 = (m.m21 * m.m33) - (m.m31 * m.m23);
            float c3 = (m.m21 * m.m32) - (m.m31 * m.m22);
            float c2 = (m.m20 * m.m33) - (m.m30 * m.m23);
            float c1 = (m.m20 * m.m32) - (m.m30 * m.m22);
            float c0 = (m.m20 * m.m31) - (m.m30 * m.m21);

            return (s0 * c5) - (s1 * c4) + (s2 * c3) + (s3 * c2) - (s4 * c1) + (s5 * c0);
        }

        public static Matrix4x4 Inverse(Matrix4x4 m)
        {
            float s0 = (m.m00 * m.m11) - (m.m10 * m.m01);
            float s1 = (m.m00 * m.m12) - (m.m10 * m.m02);
            float s2 = (m.m00 * m.m13) - (m.m10 * m.m03);
            float s3 = (m.m01 * m.m12) - (m.m11 * m.m02);
            float s4 = (m.m01 * m.m13) - (m.m11 * m.m03);
            float s5 = (m.m02 * m.m13) - (m.m12 * m.m03);

            float c5 = (m.m22 * m.m33) - (m.m32 * m.m23);
            float c4 = (m.m21 * m.m33) - (m.m31 * m.m23);
            float c3 = (m.m21 * m.m32) - (m.m31 * m.m22);
            float c2 = (m.m20 * m.m33) - (m.m30 * m.m23);
            float c1 = (m.m20 * m.m32) - (m.m30 * m.m22);
            float c0 = (m.m20 * m.m31) - (m.m30 * m.m21);

            float det = (s0 * c5) - (s1 * c4) + (s2 * c3) + (s3 * c2) - (s4 * c1) + (s5 * c0);
            if (Mathf.Abs(det) <= 1e-14f)
            {
                return zero;
            }

            float invDet = 1.0f / det;
            Matrix4x4 res;

            res.m00 = ((m.m11 * c5) - (m.m12 * c4) + (m.m13 * c3)) * invDet;
            res.m01 = ((-m.m01 * c5) + (m.m02 * c4) - (m.m03 * c3)) * invDet;
            res.m02 = ((m.m31 * s5) - (m.m32 * s4) + (m.m33 * s3)) * invDet;
            res.m03 = ((-m.m21 * s5) + (m.m22 * s4) - (m.m23 * s3)) * invDet;

            res.m10 = ((-m.m10 * c5) + (m.m12 * c2) - (m.m13 * c1)) * invDet;
            res.m11 = ((m.m00 * c5) - (m.m02 * c2) + (m.m03 * c1)) * invDet;
            res.m12 = ((-m.m30 * s5) + (m.m32 * s2) - (m.m33 * s1)) * invDet;
            res.m13 = ((m.m20 * s5) - (m.m22 * s2) + (m.m23 * s1)) * invDet;

            res.m20 = ((m.m10 * c4) - (m.m11 * c2) + (m.m13 * c0)) * invDet;
            res.m21 = ((-m.m00 * c4) + (m.m01 * c2) - (m.m03 * c0)) * invDet;
            res.m22 = ((m.m30 * s4) - (m.m31 * s2) + (m.m33 * s0)) * invDet;
            res.m23 = ((-m.m20 * s4) + (m.m21 * s2) - (m.m23 * s0)) * invDet;

            res.m30 = ((-m.m10 * c3) + (m.m11 * c1) - (m.m12 * c0)) * invDet;
            res.m31 = ((m.m00 * c3) - (m.m01 * c1) + (m.m02 * c0)) * invDet;
            res.m32 = ((-m.m30 * s3) + (m.m31 * s1) - (m.m32 * s0)) * invDet;
            res.m33 = ((m.m20 * s3) - (m.m21 * s1) + (m.m22 * s0)) * invDet;

            return res;
        }

        public static Matrix4x4 Ortho(float left, float right, float bottom, float top, float zNear, float zFar)
        {
            Matrix4x4 m = identity;
            float deltax = right - left;
            float deltay = top - bottom;
            float deltaz = zFar - zNear;

            m.m00 = 2.0f / deltax;
            m.m03 = -(right + left) / deltax;
            m.m11 = 2.0f / deltay;
            m.m13 = -(top + bottom) / deltay;
            m.m22 = -2.0f / deltaz;
            m.m23 = -(zFar + zNear) / deltaz;
            return m;
        }

        public static Matrix4x4 Perspective(float fov, float aspect, float zNear, float zFar)
        {
            Matrix4x4 m = zero;
            float rad = fov * Mathf.Deg2Rad * 0.5f;
            float cotan = Mathf.Cos(rad) / Mathf.Sin(rad);
            float deltaz = zNear - zFar;

            m.m00 = cotan / aspect;
            m.m11 = cotan;
            m.m22 = (zFar + zNear) / deltaz;
            m.m23 = (2.0f * zFar * zNear) / deltaz;
            m.m32 = -1.0f;
            return m;
        }

        public static Matrix4x4 LookAt(Vector3 from, Vector3 to, Vector3 up)
        {
            Quaternion q = Quaternion.LookRotation(to - from, up);
            return TRS(from, q, Vector3.one);
        }

        public static Matrix4x4 operator *(Matrix4x4 lhs, Matrix4x4 rhs)
        {
            Matrix4x4 res;
            res.m00 = (lhs.m00 * rhs.m00) + (lhs.m01 * rhs.m10) + (lhs.m02 * rhs.m20) + (lhs.m03 * rhs.m30);
            res.m01 = (lhs.m00 * rhs.m01) + (lhs.m01 * rhs.m11) + (lhs.m02 * rhs.m21) + (lhs.m03 * rhs.m31);
            res.m02 = (lhs.m00 * rhs.m02) + (lhs.m01 * rhs.m12) + (lhs.m02 * rhs.m22) + (lhs.m03 * rhs.m32);
            res.m03 = (lhs.m00 * rhs.m03) + (lhs.m01 * rhs.m13) + (lhs.m02 * rhs.m23) + (lhs.m03 * rhs.m33);

            res.m10 = (lhs.m10 * rhs.m00) + (lhs.m11 * rhs.m10) + (lhs.m12 * rhs.m20) + (lhs.m13 * rhs.m30);
            res.m11 = (lhs.m10 * rhs.m01) + (lhs.m11 * rhs.m11) + (lhs.m12 * rhs.m21) + (lhs.m13 * rhs.m31);
            res.m12 = (lhs.m10 * rhs.m02) + (lhs.m11 * rhs.m12) + (lhs.m12 * rhs.m22) + (lhs.m13 * rhs.m32);
            res.m13 = (lhs.m10 * rhs.m03) + (lhs.m11 * rhs.m13) + (lhs.m12 * rhs.m23) + (lhs.m13 * rhs.m33);

            res.m20 = (lhs.m20 * rhs.m00) + (lhs.m21 * rhs.m10) + (lhs.m22 * rhs.m20) + (lhs.m23 * rhs.m30);
            res.m21 = (lhs.m20 * rhs.m01) + (lhs.m21 * rhs.m11) + (lhs.m22 * rhs.m21) + (lhs.m23 * rhs.m31);
            res.m22 = (lhs.m20 * rhs.m02) + (lhs.m21 * rhs.m12) + (lhs.m22 * rhs.m22) + (lhs.m23 * rhs.m32);
            res.m23 = (lhs.m20 * rhs.m03) + (lhs.m21 * rhs.m13) + (lhs.m22 * rhs.m23) + (lhs.m23 * rhs.m33);

            res.m30 = (lhs.m30 * rhs.m00) + (lhs.m31 * rhs.m10) + (lhs.m32 * rhs.m20) + (lhs.m33 * rhs.m30);
            res.m31 = (lhs.m30 * rhs.m01) + (lhs.m31 * rhs.m11) + (lhs.m32 * rhs.m21) + (lhs.m33 * rhs.m31);
            res.m32 = (lhs.m30 * rhs.m02) + (lhs.m31 * rhs.m12) + (lhs.m32 * rhs.m22) + (lhs.m33 * rhs.m32);
            res.m33 = (lhs.m30 * rhs.m03) + (lhs.m31 * rhs.m13) + (lhs.m32 * rhs.m23) + (lhs.m33 * rhs.m33);

            return res;
        }

        public static Vector4 operator *(Matrix4x4 lhs, Vector4 vector)
        {
            Vector4 res;
            res.x = (lhs.m00 * vector.x) + (lhs.m01 * vector.y) + (lhs.m02 * vector.z) + (lhs.m03 * vector.w);
            res.y = (lhs.m10 * vector.x) + (lhs.m11 * vector.y) + (lhs.m12 * vector.z) + (lhs.m13 * vector.w);
            res.z = (lhs.m20 * vector.x) + (lhs.m21 * vector.y) + (lhs.m22 * vector.z) + (lhs.m23 * vector.w);
            res.w = (lhs.m30 * vector.x) + (lhs.m31 * vector.y) + (lhs.m32 * vector.z) + (lhs.m33 * vector.w);
            return res;
        }

        public static bool operator ==(Matrix4x4 lhs, Matrix4x4 rhs)
        {
            return lhs.GetColumn(0) == rhs.GetColumn(0)
                && lhs.GetColumn(1) == rhs.GetColumn(1)
                && lhs.GetColumn(2) == rhs.GetColumn(2)
                && lhs.GetColumn(3) == rhs.GetColumn(3);
        }

        public static bool operator !=(Matrix4x4 lhs, Matrix4x4 rhs) => !(lhs == rhs);

        public bool Equals(Matrix4x4 other)
        {
            return GetColumn(0).Equals(other.GetColumn(0))
                && GetColumn(1).Equals(other.GetColumn(1))
                && GetColumn(2).Equals(other.GetColumn(2))
                && GetColumn(3).Equals(other.GetColumn(3));
        }

        public override bool Equals(object? obj) => obj is Matrix4x4 m && Equals(m);

        public override int GetHashCode() =>
            HashCode.Combine(GetColumn(0), GetColumn(1), GetColumn(2), GetColumn(3));

        public override string ToString() =>
            string.Format(
                CultureInfo.InvariantCulture,
                "{0:F5}\t{1:F5}\t{2:F5}\t{3:F5}\n{4:F5}\t{5:F5}\t{6:F5}\t{7:F5}\n{8:F5}\t{9:F5}\t{10:F5}\t{11:F5}\n{12:F5}\t{13:F5}\t{14:F5}\t{15:F5}\n",
                m00, m01, m02, m03,
                m10, m11, m12, m13,
                m20, m21, m22, m23,
                m30, m31, m32, m33);
    }
}
