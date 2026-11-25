using System;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class ValueTypeCompletionTests
    {
        public static void Run()
        {
            TestColorConstructorsAndNamedPresets();
            TestColorPropertiesAndIndexer();
            TestColorLerpAndHsvConversions();
            TestColorOperatorsEqualityAndVector4Conversions();
            TestRaySurface();
            TestVector3StaticShorthandsAndProperties();
            TestVector3MutationAndInterpolation();
            TestVector3GeometricOperationsAndEquality();
        }

        private static void TestColorConstructorsAndNamedPresets()
        {
            Color rgba = new Color(0.1f, 0.2f, 0.3f, 0.4f);
            Check.Equal("Color(r,g,b,a) r", 0.1f, rgba.r);
            Check.Equal("Color(r,g,b,a) g", 0.2f, rgba.g);
            Check.Equal("Color(r,g,b,a) b", 0.3f, rgba.b);
            Check.Equal("Color(r,g,b,a) a", 0.4f, rgba.a);

            Color rgb = new Color(0.25f, 0.5f, 0.75f);
            Check.Equal("Color(r,g,b) r", 0.25f, rgb.r);
            Check.Equal("Color(r,g,b) g", 0.5f, rgb.g);
            Check.Equal("Color(r,g,b) b", 0.75f, rgb.b);
            Check.Equal("Color(r,g,b) defaults a to 1", 1f, rgb.a);

            Check.That("Color.red", Color.red == new Color(1f, 0f, 0f, 1f));
            Check.That("Color.green", Color.green == new Color(0f, 1f, 0f, 1f));
            Check.That("Color.blue", Color.blue == new Color(0f, 0f, 1f, 1f));
            Check.That("Color.white", Color.white == new Color(1f, 1f, 1f, 1f));
            Check.That("Color.black", Color.black == new Color(0f, 0f, 0f, 1f));
            Check.That("Color.yellow matches Unity (1, 0.92, 0.016, 1)", Color.yellow == new Color(1f, 0.92f, 0.016f, 1f));
            Check.That("Color.cyan", Color.cyan == new Color(0f, 1f, 1f, 1f));
            Check.That("Color.magenta", Color.magenta == new Color(1f, 0f, 1f, 1f));
            Check.That("Color.gray", Color.gray == new Color(0.5f, 0.5f, 0.5f, 1f));
            Check.That("Color.grey equals Color.gray", Color.grey == Color.gray);
            Check.That("Color.clear", Color.clear == new Color(0f, 0f, 0f, 0f));
        }

        private static void TestColorPropertiesAndIndexer()
        {
            Color sample = new Color(0.2f, 0.7f, 0.4f, 0.8f);
            float expectedGray = (0.299f * 0.2f) + (0.587f * 0.7f) + (0.114f * 0.4f);
            Check.Equal("Color.grayscale", expectedGray, sample.grayscale);
            Check.Equal("Color.maxColorComponent", 0.7f, sample.maxColorComponent);

            Color linear = sample.linear;
            Check.That("Color.linear darkens mid-range sRGB channels", linear.r < sample.r && linear.g < sample.g && linear.b < sample.b);
            Check.Equal("Color.linear preserves alpha", sample.a, linear.a);
            Color roundTripGamma = linear.gamma;
            Check.Equal("Color.linear.gamma round-trip r", sample.r, roundTripGamma.r);
            Check.Equal("Color.linear.gamma round-trip g", sample.g, roundTripGamma.g);
            Check.Equal("Color.linear.gamma round-trip b", sample.b, roundTripGamma.b);
            Check.Equal("Color.linear.gamma round-trip a", sample.a, roundTripGamma.a);

            Color indexed = Color.clear;
            indexed[0] = 0.1f;
            indexed[1] = 0.2f;
            indexed[2] = 0.3f;
            indexed[3] = 0.4f;
            Check.Equal("Color indexer [0]", 0.1f, indexed[0]);
            Check.Equal("Color indexer [1]", 0.2f, indexed[1]);
            Check.Equal("Color indexer [2]", 0.3f, indexed[2]);
            Check.Equal("Color indexer [3]", 0.4f, indexed[3]);

            bool threwOut = false;
            try
            {
                float unused = indexed[4];
                Check.Equal("Unreachable color index read", 0f, unused);
            }
            catch (IndexOutOfRangeException)
            {
                threwOut = true;
            }
            Check.That("Color indexer [4] throws IndexOutOfRangeException", threwOut);

            Check.Equal("Color.ToString invariant F3 format", "RGBA(0.100, 0.200, 0.300, 0.400)", indexed.ToString());
        }

        private static void TestColorLerpAndHsvConversions()
        {
            Color a = new Color(0f, 0.2f, 0.4f, 0f);
            Color b = new Color(1f, 0.6f, 0.8f, 1f);
            Color mid = Color.Lerp(a, b, 0.5f);
            Check.Equal("Color.Lerp r", 0.5f, mid.r);
            Check.Equal("Color.Lerp g", 0.4f, mid.g);
            Check.Equal("Color.Lerp b", 0.6f, mid.b);
            Check.Equal("Color.Lerp a", 0.5f, mid.a);
            Check.That("Color.Lerp clamps t < 0", Color.Lerp(a, b, -2f) == a);
            Check.That("Color.Lerp clamps t > 1", Color.Lerp(a, b, 2f) == b);

            Color unclamped = Color.LerpUnclamped(a, b, 2f);
            Check.Equal("Color.LerpUnclamped r at t=2", 2f, unclamped.r);
            Check.Equal("Color.LerpUnclamped a at t=2", 2f, unclamped.a);

            Color.RGBToHSV(Color.red, out float hRed, out float sRed, out float vRed);
            Check.Equal("RGBToHSV(red) H", 0f, hRed);
            Check.Equal("RGBToHSV(red) S", 1f, sRed);
            Check.Equal("RGBToHSV(red) V", 1f, vRed);

            Color.RGBToHSV(Color.green, out float hGreen, out float sGreen, out float vGreen);
            Check.Equal("RGBToHSV(green) H", 1f / 3f, hGreen);
            Check.Equal("RGBToHSV(green) S", 1f, sGreen);
            Check.Equal("RGBToHSV(green) V", 1f, vGreen);

            Color.RGBToHSV(Color.blue, out float hBlue, out float sBlue, out float vBlue);
            Check.Equal("RGBToHSV(blue) H", 2f / 3f, hBlue);
            Check.Equal("RGBToHSV(blue) S", 1f, sBlue);
            Check.Equal("RGBToHSV(blue) V", 1f, vBlue);

            Color fromHsv = Color.HSVToRGB(1f / 3f, 1f, 1f);
            Check.That("HSVToRGB(1/3, 1, 1) is Color.green", fromHsv == Color.green);

            Color customRgb = new Color(0.75f, 0.25f, 0.5f, 1f);
            Color.RGBToHSV(customRgb, out float cH, out float cS, out float cV);
            Color roundTripRgb = Color.HSVToRGB(cH, cS, cV);
            Check.That("RGBToHSV -> HSVToRGB round-trip", roundTripRgb == customRgb);

            Color clampedHdr = Color.HSVToRGB(0f, 1f, 2f, false);
            Check.Equal("HSVToRGB with hdr=false clamps V>1 to 1", 1f, clampedHdr.r);
            Color unclampedHdr = Color.HSVToRGB(0f, 1f, 2f, true);
            Check.Equal("HSVToRGB with hdr=true preserves V>1", 2f, unclampedHdr.r);
        }

        private static void TestColorOperatorsEqualityAndVector4Conversions()
        {
            Color c1 = new Color(0.2f, 0.3f, 0.4f, 0.5f);
            Color c2 = new Color(0.1f, 0.2f, 0.3f, 0.4f);

            Check.That("Color + Color", (c1 + c2) == new Color(0.3f, 0.5f, 0.7f, 0.9f));
            Check.That("Color - Color", (c1 - c2) == new Color(0.1f, 0.1f, 0.1f, 0.1f));
            Check.That("Color * Color", (c1 * c2) == new Color(0.02f, 0.06f, 0.12f, 0.2f));
            Check.That("Color * float", (c2 * 2f) == new Color(0.2f, 0.4f, 0.6f, 0.8f));
            Check.That("float * Color", (2f * c2) == new Color(0.2f, 0.4f, 0.6f, 0.8f));
            Check.That("Color / float", (c1 / 2f) == new Color(0.1f, 0.15f, 0.2f, 0.25f));
            Check.That("Color != Color", c1 != c2);
            Check.That("Color.Equals(Color)", c1.Equals(new Color(0.2f, 0.3f, 0.4f, 0.5f)));
            Check.Equal("Color.GetHashCode equal for identical colors", c1.GetHashCode(), new Color(0.2f, 0.3f, 0.4f, 0.5f).GetHashCode());

            Vector4 vec = c1;
            Check.That("Color -> Vector4 implicit conversion", vec == new Vector4(0.2f, 0.3f, 0.4f, 0.5f));
            Color back = vec;
            Check.That("Vector4 -> Color implicit conversion", back == c1);
        }

        private static void TestRaySurface()
        {
            Vector3 origin = new Vector3(1f, 2f, 3f);
            Vector3 dir = new Vector3(0f, 3f, 4f);
            Ray ray = new Ray(origin, dir);
            Check.That("Ray.origin", ray.origin == origin);
            Check.That("Ray.direction", ray.direction == dir);
            Check.That("Ray.GetPoint(2.5)", ray.GetPoint(2.5f) == new Vector3(1f, 9.5f, 13f));
            Check.Equal("Ray.ToString formatting", "Origin: (1.00, 2.00, 3.00), Dir: (0.00, 3.00, 4.00)", ray.ToString());
        }

        private static void TestVector3StaticShorthandsAndProperties()
        {
            Check.That("Vector3.zero", Vector3.zero == new Vector3(0f, 0f, 0f));
            Check.That("Vector3.one", Vector3.one == new Vector3(1f, 1f, 1f));
            Check.That("Vector3.up", Vector3.up == new Vector3(0f, 1f, 0f));
            Check.That("Vector3.down", Vector3.down == new Vector3(0f, -1f, 0f));
            Check.That("Vector3.left", Vector3.left == new Vector3(-1f, 0f, 0f));
            Check.That("Vector3.right", Vector3.right == new Vector3(1f, 0f, 0f));
            Check.That("Vector3.forward", Vector3.forward == new Vector3(0f, 0f, 1f));
            Check.That("Vector3.back", Vector3.back == new Vector3(0f, 0f, -1f));
            Check.That("Vector3.positiveInfinity", float.IsPositiveInfinity(Vector3.positiveInfinity.x) && float.IsPositiveInfinity(Vector3.positiveInfinity.y) && float.IsPositiveInfinity(Vector3.positiveInfinity.z));
            Check.That("Vector3.negativeInfinity", float.IsNegativeInfinity(Vector3.negativeInfinity.x) && float.IsNegativeInfinity(Vector3.negativeInfinity.y) && float.IsNegativeInfinity(Vector3.negativeInfinity.z));

            Vector3 v = new Vector3(2f, 3f, 6f);
            Check.Equal("Vector3.magnitude", 7f, v.magnitude);
            Check.Equal("Vector3.sqrMagnitude", 49f, v.sqrMagnitude);
            Check.That("Vector3.normalized", v.normalized == new Vector3(2f / 7f, 3f / 7f, 6f / 7f));

            Vector3 indexed = Vector3.zero;
            indexed[0] = 4f;
            indexed[1] = 5f;
            indexed[2] = 6f;
            Check.Equal("Vector3 indexer [0]", 4f, indexed[0]);
            Check.Equal("Vector3 indexer [1]", 5f, indexed[1]);
            Check.Equal("Vector3 indexer [2]", 6f, indexed[2]);

            bool threwOut = false;
            try
            {
                float unused = indexed[3];
                Check.Equal("Unreachable Vector3 index read", 0f, unused);
            }
            catch (IndexOutOfRangeException)
            {
                threwOut = true;
            }
            Check.That("Vector3 indexer [3] throws IndexOutOfRangeException", threwOut);
        }

        private static void TestVector3MutationAndInterpolation()
        {
            Vector3 v = Vector3.zero;
            v.Set(3f, 0f, 4f);
            Check.That("Vector3.Set", v == new Vector3(3f, 0f, 4f));
            Check.That("Vector3.Normalize(value) static leaves original unchanged", Vector3.Normalize(v) == new Vector3(0.6f, 0f, 0.8f) && v.magnitude == 5f);
            v.Normalize();
            Check.That("Vector3.Normalize() instance mutates in place", v == new Vector3(0.6f, 0f, 0.8f));

            Vector3 tiny = new Vector3(1e-7f, 0f, 0f);
            tiny.Normalize();
            Check.That("Vector3.Normalize() below kEpsilon becomes zero", tiny == Vector3.zero);

            Vector3 a = new Vector3(0f, 2f, 4f);
            Vector3 b = new Vector3(10f, 6f, 8f);
            Check.That("Vector3.Lerp mid", Vector3.Lerp(a, b, 0.25f) == new Vector3(2.5f, 3f, 5f));
            Check.That("Vector3.Lerp clamps > 1", Vector3.Lerp(a, b, 3f) == b);
            Check.That("Vector3.LerpUnclamped extrapolates", Vector3.LerpUnclamped(a, b, 1.5f) == new Vector3(15f, 8f, 10f));

            Vector3 moved = Vector3.MoveTowards(Vector3.zero, new Vector3(3f, 4f, 0f), 2.5f);
            Check.That("Vector3.MoveTowards partial step", moved == new Vector3(1.5f, 2f, 0f));
            Check.That("Vector3.MoveTowards reaches target", Vector3.MoveTowards(Vector3.zero, new Vector3(3f, 4f, 0f), 10f) == new Vector3(3f, 4f, 0f));

            Vector3 slerpMid = Vector3.Slerp(Vector3.right, Vector3.up, 0.5f);
            float invSqrt2 = Mathf.Sqrt(0.5f);
            Check.Equal("Vector3.Slerp(right, up, 0.5) x", invSqrt2, slerpMid.x);
            Check.Equal("Vector3.Slerp(right, up, 0.5) y", invSqrt2, slerpMid.y);
            Check.Equal("Vector3.Slerp(right, up, 0.5) magnitude", 1f, slerpMid.magnitude);

            Vector3 slerpClamped = Vector3.Slerp(Vector3.right, Vector3.up, 2f);
            Check.That("Vector3.Slerp clamps t > 1", slerpClamped == Vector3.up);

            Vector3 slerpUnclamped = Vector3.SlerpUnclamped(Vector3.right, Vector3.up, 2f);
            Check.That("Vector3.SlerpUnclamped(right, up, 2) reaches -right", slerpUnclamped == Vector3.left);

            Vector3 slerpOpposite = Vector3.Slerp(Vector3.right, Vector3.left, 0.5f);
            Check.Equal("Vector3.Slerp opposite vectors preserves unit length", 1f, slerpOpposite.magnitude);
            Check.Equal("Vector3.Slerp opposite vectors midpoint is orthogonal to right", 0f, Vector3.Dot(slerpOpposite, Vector3.right));
        }

        private static void TestVector3GeometricOperationsAndEquality()
        {
            Vector3 s1 = new Vector3(2f, 3f, 4f);
            Vector3 s2 = new Vector3(5f, 6f, 7f);
            Check.That("Vector3.Scale static", Vector3.Scale(s1, s2) == new Vector3(10f, 18f, 28f));
            s1.Scale(s2);
            Check.That("Vector3.Scale instance", s1 == new Vector3(10f, 18f, 28f));

            Vector3 reflected = Vector3.Reflect(new Vector3(1f, -1f, 2f), Vector3.up);
            Check.That("Vector3.Reflect across up normal", reflected == new Vector3(1f, 1f, 2f));

            Vector3 projected = Vector3.Project(new Vector3(3f, 4f, 5f), new Vector3(0f, 2f, 0f));
            Check.That("Vector3.Project onto non-unit +y axis", projected == new Vector3(0f, 4f, 0f));
            Check.That("Vector3.Project onto zero normal returns zero", Vector3.Project(new Vector3(3f, 4f, 5f), Vector3.zero) == Vector3.zero);

            Vector3 onPlane = Vector3.ProjectOnPlane(new Vector3(3f, 4f, 5f), new Vector3(0f, 2f, 0f));
            Check.That("Vector3.ProjectOnPlane onto XZ plane", onPlane == new Vector3(3f, 0f, 5f));

            Check.Equal("Vector3.Angle(right, up)", 90f, Vector3.Angle(Vector3.right, Vector3.up));
            Check.Equal("Vector3.Angle with zero vector is 0", 0f, Vector3.Angle(Vector3.zero, Vector3.up));
            Check.Equal("Vector3.SignedAngle(right, up, forward) is +90", 90f, Vector3.SignedAngle(Vector3.right, Vector3.up, Vector3.forward));
            Check.Equal("Vector3.SignedAngle(up, right, forward) is -90", -90f, Vector3.SignedAngle(Vector3.up, Vector3.right, Vector3.forward));

            Vector3 clamped = Vector3.ClampMagnitude(new Vector3(0f, 6f, 8f), 5f);
            Check.That("Vector3.ClampMagnitude scales down to maxLength", clamped == new Vector3(0f, 3f, 4f));
            Check.That("Vector3.ClampMagnitude leaves shorter vector unchanged", Vector3.ClampMagnitude(new Vector3(1f, 2f, 2f), 5f) == new Vector3(1f, 2f, 2f));

            Check.That("Vector3.Min", Vector3.Min(new Vector3(1f, 5f, 3f), new Vector3(2f, 4f, 6f)) == new Vector3(1f, 4f, 3f));
            Check.That("Vector3.Max", Vector3.Max(new Vector3(1f, 5f, 3f), new Vector3(2f, 4f, 6f)) == new Vector3(2f, 5f, 6f));

            Vector3 baseVec = new Vector3(1f, 2f, 3f);
            Vector3 nearVec = new Vector3(1f + 1e-7f, 2f - 1e-7f, 3f + 1e-7f);
            Check.That("Vector3 == uses approximate threshold", baseVec == nearVec);
            Check.That("Vector3 != false for approximate match", !(baseVec != nearVec));
            Check.That("Vector3.Equals uses exact component equality", !baseVec.Equals(nearVec) && baseVec.Equals(new Vector3(1f, 2f, 3f)));
            Check.Equal("Vector3.ToString uses invariant F2 format", "(1.00, 2.00, 3.00)", baseVec.ToString());
        }
    }
}
