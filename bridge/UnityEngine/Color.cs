using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Color : IEquatable<Color>
    {
        public float r;
        public float g;
        public float b;
        public float a;

        public Color(float r, float g, float b, float a)
        {
            this.r = r;
            this.g = g;
            this.b = b;
            this.a = a;
        }

        public Color(float r, float g, float b)
        {
            this.r = r;
            this.g = g;
            this.b = b;
            a = 1f;
        }

        public float this[int index]
        {
            get
            {
                switch (index)
                {
                    case 0:
                        return r;
                    case 1:
                        return g;
                    case 2:
                        return b;
                    case 3:
                        return a;
                    default:
                        throw new IndexOutOfRangeException("Invalid Color index(" + index + ")!");
                }
            }
            set
            {
                switch (index)
                {
                    case 0:
                        r = value;
                        break;
                    case 1:
                        g = value;
                        break;
                    case 2:
                        b = value;
                        break;
                    case 3:
                        a = value;
                        break;
                    default:
                        throw new IndexOutOfRangeException("Invalid Color index(" + index + ")!");
                }
            }
        }

        public static Color red => new Color(1f, 0f, 0f, 1f);
        public static Color green => new Color(0f, 1f, 0f, 1f);
        public static Color blue => new Color(0f, 0f, 1f, 1f);
        public static Color white => new Color(1f, 1f, 1f, 1f);
        public static Color black => new Color(0f, 0f, 0f, 1f);
        public static Color yellow => new Color(1f, 0.92f, 0.016f, 1f);
        public static Color cyan => new Color(0f, 1f, 1f, 1f);
        public static Color magenta => new Color(1f, 0f, 1f, 1f);
        public static Color gray => new Color(0.5f, 0.5f, 0.5f, 1f);
        public static Color grey => new Color(0.5f, 0.5f, 0.5f, 1f);
        public static Color clear => new Color(0f, 0f, 0f, 0f);

        public float grayscale => (0.299f * r) + (0.587f * g) + (0.114f * b);

        public Color linear => new Color(GammaToLinearSpace(r), GammaToLinearSpace(g), GammaToLinearSpace(b), a);

        public Color gamma => new Color(LinearToGammaSpace(r), LinearToGammaSpace(g), LinearToGammaSpace(b), a);

        public float maxColorComponent => Mathf.Max(Mathf.Max(r, g), b);

        public static Color Lerp(Color a, Color b, float t)
        {
            t = Mathf.Clamp01(t);
            return new Color(
                a.r + ((b.r - a.r) * t),
                a.g + ((b.g - a.g) * t),
                a.b + ((b.b - a.b) * t),
                a.a + ((b.a - a.a) * t));
        }

        public static Color LerpUnclamped(Color a, Color b, float t)
        {
            return new Color(
                a.r + ((b.r - a.r) * t),
                a.g + ((b.g - a.g) * t),
                a.b + ((b.b - a.b) * t),
                a.a + ((b.a - a.a) * t));
        }

        public static void RGBToHSV(Color rgbColor, out float H, out float S, out float V)
        {
            if (rgbColor.b > rgbColor.g && rgbColor.b > rgbColor.r)
            {
                RGBToHSVHelper(4f, rgbColor.b, rgbColor.r, rgbColor.g, out H, out S, out V);
            }
            else if (rgbColor.g > rgbColor.r)
            {
                RGBToHSVHelper(2f, rgbColor.g, rgbColor.b, rgbColor.r, out H, out S, out V);
            }
            else
            {
                RGBToHSVHelper(0f, rgbColor.r, rgbColor.g, rgbColor.b, out H, out S, out V);
            }
        }

        private static void RGBToHSVHelper(float offset, float dominantcolor, float colorone, float colortwo, out float H, out float S, out float V)
        {
            V = dominantcolor;
            if (V != 0f)
            {
                float num = colorone > colortwo ? colortwo : colorone;
                float num2 = V - num;
                if (num2 != 0f)
                {
                    S = num2 / V;
                    H = offset + ((colorone - colortwo) / num2);
                }
                else
                {
                    S = 0f;
                    H = offset + (colorone - colortwo);
                }
                H /= 6f;
                if (H < 0f)
                {
                    H += 1f;
                }
            }
            else
            {
                S = 0f;
                H = 0f;
            }
        }

        public static Color HSVToRGB(float H, float S, float V) => HSVToRGB(H, S, V, true);

        public static Color HSVToRGB(float H, float S, float V, bool hdr)
        {
            Color retval = white;
            if (S == 0f)
            {
                retval.r = V;
                retval.g = V;
                retval.b = V;
            }
            else if (V == 0f)
            {
                retval.r = 0f;
                retval.g = 0f;
                retval.b = 0f;
            }
            else
            {
                retval.r = 0f;
                retval.g = 0f;
                retval.b = 0f;

                float tS = S;
                float tV = V;
                float hToFloor = H * 6f;

                int temp = (int)Mathf.Floor(hToFloor);
                float t = hToFloor - temp;
                float var1 = tV * (1f - tS);
                float var2 = tV * (1f - (tS * t));
                float var3 = tV * (1f - (tS * (1f - t)));

                switch (temp)
                {
                    case 0:
                        retval.r = tV;
                        retval.g = var3;
                        retval.b = var1;
                        break;
                    case 1:
                        retval.r = var2;
                        retval.g = tV;
                        retval.b = var1;
                        break;
                    case 2:
                        retval.r = var1;
                        retval.g = tV;
                        retval.b = var3;
                        break;
                    case 3:
                        retval.r = var1;
                        retval.g = var2;
                        retval.b = tV;
                        break;
                    case 4:
                        retval.r = var3;
                        retval.g = var1;
                        retval.b = tV;
                        break;
                    case 5:
                        retval.r = tV;
                        retval.g = var1;
                        retval.b = var2;
                        break;
                    case 6:
                        retval.r = tV;
                        retval.g = var3;
                        retval.b = var1;
                        break;
                    case -1:
                        retval.r = tV;
                        retval.g = var1;
                        retval.b = var2;
                        break;
                }

                if (!hdr)
                {
                    retval.r = Mathf.Clamp(retval.r, 0f, 1f);
                    retval.g = Mathf.Clamp(retval.g, 0f, 1f);
                    retval.b = Mathf.Clamp(retval.b, 0f, 1f);
                }
            }
            return retval;
        }

        private static float GammaToLinearSpace(float value)
        {
            if (value <= 0.04045f)
            {
                return value / 12.92f;
            }
            if (value < 1.0f)
            {
                return Mathf.Pow((value + 0.055f) / 1.055f, 2.4f);
            }
            return Mathf.Pow(value, 2.2f);
        }

        private static float LinearToGammaSpace(float value)
        {
            if (value <= 0.0f)
            {
                return 0.0f;
            }
            if (value <= 0.0031308f)
            {
                return 12.92f * value;
            }
            if (value < 1.0f)
            {
                return (1.055f * Mathf.Pow(value, 0.4166667f)) - 0.055f;
            }
            return Mathf.Pow(value, 0.45454545f);
        }

        public static Color operator +(Color a, Color b) => new Color(a.r + b.r, a.g + b.g, a.b + b.b, a.a + b.a);

        public static Color operator -(Color a, Color b) => new Color(a.r - b.r, a.g - b.g, a.b - b.b, a.a - b.a);

        public static Color operator *(Color a, Color b) => new Color(a.r * b.r, a.g * b.g, a.b * b.b, a.a * b.a);

        public static Color operator *(Color a, float b) => new Color(a.r * b, a.g * b, a.b * b, a.a * b);

        public static Color operator *(float b, Color a) => new Color(a.r * b, a.g * b, a.b * b, a.a * b);

        public static Color operator /(Color a, float b) => new Color(a.r / b, a.g / b, a.b / b, a.a / b);

        public static bool operator ==(Color lhs, Color rhs) => (Vector4)lhs == (Vector4)rhs;

        public static bool operator !=(Color lhs, Color rhs) => !(lhs == rhs);

        public static implicit operator Vector4(Color c) => new Vector4(c.r, c.g, c.b, c.a);

        public static implicit operator Color(Vector4 v) => new Color(v.x, v.y, v.z, v.w);

        public bool Equals(Color other) =>
            r.Equals(other.r) && g.Equals(other.g) && b.Equals(other.b) && a.Equals(other.a);

        public override bool Equals(object? obj) => obj is Color c && Equals(c);

        public override int GetHashCode() => HashCode.Combine(r, g, b, a);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "RGBA({0:F3}, {1:F3}, {2:F3}, {3:F3})", r, g, b, a);
    }
}
