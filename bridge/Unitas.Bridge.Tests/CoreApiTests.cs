using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public interface ICoreProbeMarker
    {
        int MarkerValue { get; }
    }

    public class CoreBaseProbeComponent : MonoBehaviour, ICoreProbeMarker
    {
        public int ProbeId { get; set; }

        public int MarkerValue => ProbeId;
    }

    public sealed class CoreDerivedProbeComponent : CoreBaseProbeComponent
    {
        public string TagLabel { get; set; } = string.Empty;
    }

    public sealed class CoreOtherProbeComponent : MonoBehaviour
    {
        public float Weight { get; set; }
    }

    public static class CoreApiTests
    {
        public static void Run()
        {
            TestMathfSurface();
            TestVector2Surface();
            TestVector4Surface();
            TestQuaternionSurface();
            TestRectSurface();
            TestBoundsSurface();
            TestMatrix4x4Surface();
            TestTransformRotationEulerAndAxes();
            TestTransformScaleAndTrsSpaces();
            TestTransformRotateAndLookAt();
            TestTransformSetParentWorldPositionStaysAndNegativeScale();
            TestTransformSiblingIndexAndFind();
            TestGetComponentsInChildrenAndParent();
            TestTryGetComponentAndListOverloads();
            ObjectModelTests.RunAll();
        }

        private static void TestMathfSurface()
        {
            Check.That("Mathf.PI is approximately pi", Mathf.Abs(Mathf.PI - 3.14159274f) < 1e-6f);
            Check.That("Mathf.Deg2Rad * 180 == PI", Mathf.Approximately(180f * Mathf.Deg2Rad, Mathf.PI));
            Check.That("Mathf.Rad2Deg * PI == 180", Mathf.Approximately(Mathf.PI * Mathf.Rad2Deg, 180f));
            Check.That("Mathf.Epsilon > 0", Mathf.Epsilon > 0f);
            Check.That("Mathf.Infinity is positive infinity", float.IsPositiveInfinity(Mathf.Infinity));
            Check.That("Mathf.NegativeInfinity is negative infinity", float.IsNegativeInfinity(Mathf.NegativeInfinity));

            Check.Equal("Mathf.Sin(PI/2)", 1f, Mathf.Sin(Mathf.PI * 0.5f));
            Check.Equal("Mathf.Cos(0)", 1f, Mathf.Cos(0f));
            Check.Equal("Mathf.Tan(PI/4)", 1f, Mathf.Tan(Mathf.PI * 0.25f));
            Check.Equal("Mathf.Asin(1)", Mathf.PI * 0.5f, Mathf.Asin(1f));
            Check.Equal("Mathf.Acos(1)", 0f, Mathf.Acos(1f));
            Check.Equal("Mathf.Atan(1)", Mathf.PI * 0.25f, Mathf.Atan(1f));
            Check.Equal("Mathf.Atan2(1, 1)", Mathf.PI * 0.25f, Mathf.Atan2(1f, 1f));
            Check.Equal("Mathf.Sqrt(9)", 3f, Mathf.Sqrt(9f));
            Check.Equal("Mathf.Abs(-4.5f)", 4.5f, Mathf.Abs(-4.5f));
            Check.Equal("Mathf.Abs(-7)", 7, Mathf.Abs(-7));

            Check.Equal("Mathf.Min(3f, -2f)", -2f, Mathf.Min(3f, -2f));
            Check.Equal("Mathf.Min(params float)", -5f, Mathf.Min(2f, -5f, 9f));
            Check.Equal("Mathf.Min(4, 1)", 1, Mathf.Min(4, 1));
            Check.Equal("Mathf.Min(params int)", -3, Mathf.Min(4, -3, 10));
            Check.Equal("Mathf.Max(3f, -2f)", 3f, Mathf.Max(3f, -2f));
            Check.Equal("Mathf.Max(params float)", 9f, Mathf.Max(2f, -5f, 9f));
            Check.Equal("Mathf.Max(4, 1)", 4, Mathf.Max(4, 1));
            Check.Equal("Mathf.Max(params int)", 10, Mathf.Max(4, -3, 10));

            Check.Equal("Mathf.Pow(2, 3)", 8f, Mathf.Pow(2f, 3f));
            Check.Equal("Mathf.Exp(0)", 1f, Mathf.Exp(0f));
            Check.Equal("Mathf.Log(8, 2)", 3f, Mathf.Log(8f, 2f));
            Check.Equal("Mathf.Log(e)", 1f, Mathf.Log((float)Math.E));
            Check.Equal("Mathf.Log10(100)", 2f, Mathf.Log10(100f));

            Check.Equal("Mathf.Ceil(1.2f)", 2f, Mathf.Ceil(1.2f));
            Check.Equal("Mathf.Floor(1.8f)", 1f, Mathf.Floor(1.8f));
            Check.Equal("Mathf.Round(2.5f) banker's rounding to even", 2f, Mathf.Round(2.5f));
            Check.Equal("Mathf.Round(3.5f) banker's rounding to even", 4f, Mathf.Round(3.5f));
            Check.Equal("Mathf.CeilToInt(1.2f)", 2, Mathf.CeilToInt(1.2f));
            Check.Equal("Mathf.FloorToInt(1.8f)", 1, Mathf.FloorToInt(1.8f));
            Check.Equal("Mathf.RoundToInt(2.5f)", 2, Mathf.RoundToInt(2.5f));
            Check.Equal("Mathf.RoundToInt(3.5f)", 4, Mathf.RoundToInt(3.5f));

            Check.Equal("Mathf.Sign(0f) is +1", 1f, Mathf.Sign(0f));
            Check.Equal("Mathf.Sign(5f) is +1", 1f, Mathf.Sign(5f));
            Check.Equal("Mathf.Sign(-0.1f) is -1", -1f, Mathf.Sign(-0.1f));

            Check.Equal("Mathf.Clamp float low", 2f, Mathf.Clamp(-1f, 2f, 5f));
            Check.Equal("Mathf.Clamp float high", 5f, Mathf.Clamp(10f, 2f, 5f));
            Check.Equal("Mathf.Clamp int", 4, Mathf.Clamp(10, 1, 4));
            Check.Equal("Mathf.Clamp01 low", 0f, Mathf.Clamp01(-3f));
            Check.Equal("Mathf.Clamp01 high", 1f, Mathf.Clamp01(3f));
            Check.Equal("Mathf.Clamp01 mid", 0.4f, Mathf.Clamp01(0.4f));

            Check.Equal("Mathf.Lerp clamped", 20f, Mathf.Lerp(10f, 20f, 1.5f));
            Check.Equal("Mathf.Lerp mid", 15f, Mathf.Lerp(10f, 20f, 0.5f));
            Check.Equal("Mathf.LerpUnclamped", 25f, Mathf.LerpUnclamped(10f, 20f, 1.5f));
            Check.Equal("Mathf.InverseLerp", 0.5f, Mathf.InverseLerp(10f, 20f, 15f));
            Check.Equal("Mathf.InverseLerp equal bounds", 0f, Mathf.InverseLerp(5f, 5f, 5f));

            Check.Equal("Mathf.DeltaAngle(350, 10)", 20f, Mathf.DeltaAngle(350f, 10f));
            Check.Equal("Mathf.DeltaAngle(10, 350)", -20f, Mathf.DeltaAngle(10f, 350f));
            Check.Equal("Mathf.LerpAngle(350, 10, 0.5)", 360f, Mathf.LerpAngle(350f, 10f, 0.5f));
            Check.Equal("Mathf.MoveTowards step", 3f, Mathf.MoveTowards(1f, 10f, 2f));
            Check.Equal("Mathf.MoveTowards reach", 10f, Mathf.MoveTowards(9f, 10f, 2f));
            Check.Equal("Mathf.MoveTowardsAngle wrap", 360f, Mathf.MoveTowardsAngle(350f, 10f, 10f));
            Check.Equal("Mathf.SmoothStep mid", 0.5f, Mathf.SmoothStep(0f, 1f, 0.5f));
            Check.Equal("Mathf.Repeat(3.5, 2)", 1.5f, Mathf.Repeat(3.5f, 2f));
            Check.Equal("Mathf.Repeat(-0.5, 2)", 1.5f, Mathf.Repeat(-0.5f, 2f));
            Check.Equal("Mathf.PingPong(2.5, 2)", 1.5f, Mathf.PingPong(2.5f, 2f));

            Check.That("Mathf.Approximately near values", Mathf.Approximately(1.0f, 1.0f + 1e-7f));
            Check.That("Mathf.Approximately distinct values", !Mathf.Approximately(1.0f, 1.01f));

            float vel = 0f;
            float damped = Mathf.SmoothDamp(0f, 10f, ref vel, 0.3f, Mathf.Infinity, 0.02f);
            Check.That("Mathf.SmoothDamp advances toward target", damped > 0f && damped < 10f && vel > 0f);

            float angleVel = 0f;
            float dampedAngle = Mathf.SmoothDampAngle(350f, 10f, ref angleVel, 0.1f, Mathf.Infinity, 0.02f);
            Check.That("Mathf.SmoothDampAngle moves across 360 seam", dampedAngle > 350f);
        }

        private static void TestVector2Surface()
        {
            Check.That("Vector2.zero", Vector2.zero == new Vector2(0f, 0f));
            Check.That("Vector2.one", Vector2.one == new Vector2(1f, 1f));
            Check.That("Vector2.up", Vector2.up == new Vector2(0f, 1f));
            Check.That("Vector2.down", Vector2.down == new Vector2(0f, -1f));
            Check.That("Vector2.left", Vector2.left == new Vector2(-1f, 0f));
            Check.That("Vector2.right", Vector2.right == new Vector2(1f, 0f));

            Vector2 v = new Vector2(3f, 4f);
            Check.Equal("Vector2.magnitude", 5f, v.magnitude);
            Check.Equal("Vector2.sqrMagnitude", 25f, v.sqrMagnitude);
            Check.Equal("Vector2.normalized x", 0.6f, v.normalized.x);
            Check.Equal("Vector2.normalized y", 0.8f, v.normalized.y);
            Check.That("Vector2.zero.normalized is zero", Vector2.zero.normalized == Vector2.zero);

            Vector2 idxVec = Vector2.zero;
            idxVec[0] = 7f;
            idxVec[1] = 8f;
            Check.Equal("Vector2 indexer [0]", 7f, idxVec[0]);
            Check.Equal("Vector2 indexer [1]", 8f, idxVec[1]);

            Vector2 a = new Vector2(1f, 2f);
            Vector2 b = new Vector2(3f, 4f);
            Check.That("Vector2 +", (a + b) == new Vector2(4f, 6f));
            Check.That("Vector2 -", (b - a) == new Vector2(2f, 2f));
            Check.That("Vector2 unary -", (-a) == new Vector2(-1f, -2f));
            Check.That("Vector2 * scalar", (a * 2f) == new Vector2(2f, 4f));
            Check.That("scalar * Vector2", (2f * a) == new Vector2(2f, 4f));
            Check.That("Vector2 / scalar", (b / 2f) == new Vector2(1.5f, 2f));
            Check.That("Vector2 * Vector2", (a * b) == new Vector2(3f, 8f));
            Check.That("Vector2 / Vector2", (b / a) == new Vector2(3f, 2f));

            Check.Equal("Vector2.Dot", 11f, Vector2.Dot(a, b));
            Check.Equal("Vector2.Distance", 5f, Vector2.Distance(Vector2.zero, v));
            Check.Equal("Vector2.Angle(right, up)", 90f, Vector2.Angle(Vector2.right, Vector2.up));
            Check.Equal("Vector2.SignedAngle(right, up)", 90f, Vector2.SignedAngle(Vector2.right, Vector2.up));
            Check.Equal("Vector2.SignedAngle(up, right)", -90f, Vector2.SignedAngle(Vector2.up, Vector2.right));
            Check.That("Vector2.Perpendicular(right) is up", Vector2.Perpendicular(Vector2.right) == Vector2.up);
            Check.That("Vector2.Reflect", Vector2.Reflect(new Vector2(1f, -1f), Vector2.up) == new Vector2(1f, 1f));
            Check.That("Vector2.Lerp", Vector2.Lerp(Vector2.zero, new Vector2(10f, 20f), 0.25f) == new Vector2(2.5f, 5f));
            Check.That("Vector2.MoveTowards", Vector2.MoveTowards(Vector2.zero, new Vector2(3f, 4f), 2.5f) == new Vector2(1.5f, 2f));
            Check.Equal("Vector2.ClampMagnitude", 2f, Vector2.ClampMagnitude(v, 2f).magnitude);
            Check.That("Vector2.Min", Vector2.Min(new Vector2(1f, 5f), new Vector2(3f, 2f)) == new Vector2(1f, 2f));
            Check.That("Vector2.Max", Vector2.Max(new Vector2(1f, 5f), new Vector2(3f, 2f)) == new Vector2(3f, 5f));

            Vector3 asV3 = a;
            Check.Equal("Vector2 -> Vector3 implicit x", 1f, asV3.x);
            Check.Equal("Vector2 -> Vector3 implicit y", 2f, asV3.y);
            Check.Equal("Vector2 -> Vector3 implicit z", 0f, asV3.z);
            Vector2 backToV2 = new Vector3(9f, 8f, 7f);
            Check.That("Vector3 -> Vector2 implicit drops z", backToV2 == new Vector2(9f, 8f));
        }

        private static void TestVector4Surface()
        {
            Check.That("Vector4.zero", Vector4.zero == new Vector4(0f, 0f, 0f, 0f));
            Check.That("Vector4.one", Vector4.one == new Vector4(1f, 1f, 1f, 1f));

            Vector4 v = new Vector4(1f, 2f, 2f, 4f);
            Check.Equal("Vector4.sqrMagnitude", 25f, v.sqrMagnitude);
            Check.Equal("Vector4.magnitude", 5f, v.magnitude);
            Check.Equal("Vector4.normalized magnitude", 1f, v.normalized.magnitude);
            Check.That("Vector4.zero.normalized is zero", Vector4.zero.normalized == Vector4.zero);

            Vector4 a = new Vector4(1f, 2f, 3f, 4f);
            Vector4 b = new Vector4(2f, 3f, 4f, 5f);
            Check.That("Vector4 +", (a + b) == new Vector4(3f, 5f, 7f, 9f));
            Check.That("Vector4 -", (b - a) == Vector4.one);
            Check.That("Vector4 * scalar", (a * 2f) == new Vector4(2f, 4f, 6f, 8f));
            Check.That("Vector4 / scalar", (a / 2f) == new Vector4(0.5f, 1f, 1.5f, 2f));
            Check.Equal("Vector4.Dot", 40f, Vector4.Dot(a, b));
            Check.Equal("Vector4.Distance", 2f, Vector4.Distance(a, b));
            Check.That("Vector4.Scale", Vector4.Scale(a, b) == new Vector4(2f, 6f, 12f, 20f));
            Check.That("Vector4.Lerp", Vector4.Lerp(Vector4.zero, a, 0.5f) == new Vector4(0.5f, 1f, 1.5f, 2f));

            Vector4 fromV3 = new Vector3(3f, 4f, 5f);
            Check.That("Vector3 -> Vector4 sets w = 0", fromV3 == new Vector4(3f, 4f, 5f, 0f));
            Vector3 toV3 = a;
            Check.That("Vector4 -> Vector3 keeps xyz", toV3 == new Vector3(1f, 2f, 3f));
        }

        private static void TestQuaternionSurface()
        {
            Quaternion id = Quaternion.identity;
            Check.Equal("Quaternion.identity x", 0f, id.x);
            Check.Equal("Quaternion.identity y", 0f, id.y);
            Check.Equal("Quaternion.identity z", 0f, id.z);
            Check.Equal("Quaternion.identity w", 1f, id.w);

            Quaternion rotY90 = Quaternion.Euler(0f, 90f, 0f);
            Vector3 rotatedForward = rotY90 * Vector3.forward;
            Check.Equal("Euler(0,90,0) * forward -> +x", 1f, rotatedForward.x);
            Check.Equal("Euler(0,90,0) * forward -> 0y", 0f, rotatedForward.y);
            Check.Equal("Euler(0,90,0) * forward -> 0z", 0f, rotatedForward.z);

            Vector3 eulerRoundTrip = rotY90.eulerAngles;
            Check.Equal("Euler(0,90,0) round-trip x", 0f, eulerRoundTrip.x);
            Check.Equal("Euler(0,90,0) round-trip y", 90f, eulerRoundTrip.y);
            Check.Equal("Euler(0,90,0) round-trip z", 0f, eulerRoundTrip.z);

            Quaternion angleAxis = Quaternion.AngleAxis(90f, Vector3.up);
            Check.That("AngleAxis(90, up) matches Euler(0,90,0)", angleAxis == rotY90);
            angleAxis.ToAngleAxis(out float outAngle, out Vector3 outAxis);
            Check.Equal("ToAngleAxis angle", 90f, outAngle);
            Check.Equal("ToAngleAxis axis y", 1f, outAxis.y);

            Quaternion inv = Quaternion.Inverse(rotY90);
            Vector3 restored = inv * rotatedForward;
            Check.Equal("Inverse rotation restores forward x", 0f, restored.x);
            Check.Equal("Inverse rotation restores forward y", 0f, restored.y);
            Check.Equal("Inverse rotation restores forward z", 1f, restored.z);

            Quaternion fromTo = Quaternion.FromToRotation(Vector3.forward, Vector3.right);
            Vector3 fromToVec = fromTo * Vector3.forward;
            Check.Equal("FromToRotation(forward, right) * forward x", 1f, fromToVec.x);
            Check.Equal("FromToRotation(forward, right) * forward z", 0f, fromToVec.z);

            Quaternion fromToOpposite = Quaternion.FromToRotation(Vector3.forward, -Vector3.forward);
            Vector3 oppositeVec = fromToOpposite * Vector3.forward;
            Check.Equal("FromToRotation 180 degrees flips forward z", -1f, oppositeVec.z);

            Quaternion lookRight = Quaternion.LookRotation(Vector3.right, Vector3.up);
            Vector3 lookForward = lookRight * Vector3.forward;
            Check.Equal("LookRotation(right) * forward x", 1f, lookForward.x);
            Check.Equal("LookRotation(right) * forward z", 0f, lookForward.z);

            Check.Equal("Quaternion.Angle(identity, rotY90)", 90f, Quaternion.Angle(id, rotY90));
            Quaternion slerpHalf = Quaternion.Slerp(id, rotY90, 0.5f);
            Check.Equal("Slerp(identity, rotY90, 0.5) angle is 45", 45f, Quaternion.Angle(id, slerpHalf));

            Quaternion stepped = Quaternion.RotateTowards(id, rotY90, 30f);
            Check.Equal("RotateTowards steps by maxDegreesDelta", 30f, Quaternion.Angle(id, stepped));
            Quaternion reached = Quaternion.RotateTowards(id, rotY90, 120f);
            Check.That("RotateTowards clamps at target", reached == rotY90);
        }

        private static void TestRectSurface()
        {
            Rect r = new Rect(10f, 20f, 100f, 50f);
            Check.Equal("Rect.x", 10f, r.x);
            Check.Equal("Rect.y", 20f, r.y);
            Check.Equal("Rect.width", 100f, r.width);
            Check.Equal("Rect.height", 50f, r.height);
            Check.Equal("Rect.xMin", 10f, r.xMin);
            Check.Equal("Rect.yMin", 20f, r.yMin);
            Check.Equal("Rect.xMax", 110f, r.xMax);
            Check.Equal("Rect.yMax", 70f, r.yMax);
            Check.That("Rect.center", r.center == new Vector2(60f, 45f));
            Check.That("Rect.position", r.position == new Vector2(10f, 20f));
            Check.That("Rect.size", r.size == new Vector2(100f, 50f));

            Check.That("Rect.Contains inside point", r.Contains(new Vector2(50f, 40f)));
            Check.That("Rect.Contains min corner inclusive", r.Contains(new Vector2(10f, 20f)));
            Check.That("Rect.Contains max corner exclusive", !r.Contains(new Vector2(110f, 70f)));
            Check.That("Rect.Contains outside point", !r.Contains(new Vector2(5f, 40f)));

            Rect overlapping = Rect.MinMaxRect(50f, 30f, 150f, 90f);
            Rect disjoint = new Rect(200f, 200f, 10f, 10f);
            Check.That("Rect.Overlaps true", r.Overlaps(overlapping));
            Check.That("Rect.Overlaps false", !r.Overlaps(disjoint));

            Vector2 norm = Rect.PointToNormalized(r, new Vector2(60f, 45f));
            Check.That("Rect.PointToNormalized center is (0.5, 0.5)", norm == new Vector2(0.5f, 0.5f));
            Vector2 pt = Rect.NormalizedToPoint(r, norm);
            Check.That("Rect.NormalizedToPoint round-trip", pt == new Vector2(60f, 45f));
        }

        private static void TestBoundsSurface()
        {
            Bounds b = new Bounds(new Vector3(0f, 0f, 0f), new Vector3(2f, 4f, 6f));
            Check.That("Bounds.extents", b.extents == new Vector3(1f, 2f, 3f));
            Check.That("Bounds.min", b.min == new Vector3(-1f, -2f, -3f));
            Check.That("Bounds.max", b.max == new Vector3(1f, 2f, 3f));
            Check.That("Bounds.Contains center", b.Contains(Vector3.zero));
            Check.That("Bounds.Contains surface point", b.Contains(new Vector3(1f, 2f, 3f)));
            Check.That("Bounds.Contains outside point false", !b.Contains(new Vector3(2f, 0f, 0f)));

            Vector3 closest = b.ClosestPoint(new Vector3(5f, 0f, -10f));
            Check.That("Bounds.ClosestPoint clamps to box surface", closest == new Vector3(1f, 0f, -3f));
            Check.Equal("Bounds.SqrDistance from (3,0,0)", 4f, b.SqrDistance(new Vector3(3f, 0f, 0f)));

            Bounds other = new Bounds(new Vector3(1.5f, 0f, 0f), new Vector3(2f, 2f, 2f));
            Check.That("Bounds.Intersects overlapping", b.Intersects(other));
            Bounds far = new Bounds(new Vector3(10f, 0f, 0f), Vector3.one);
            Check.That("Bounds.Intersects disjoint", !b.Intersects(far));

            Ray hitRay = new Ray(new Vector3(-5f, 0f, 0f), Vector3.right);
            bool hit = b.IntersectRay(hitRay, out float hitDist);
            Check.That("Bounds.IntersectRay hits along +x", hit);
            Check.Equal("Bounds.IntersectRay distance", 4f, hitDist);

            Ray missRay = new Ray(new Vector3(-5f, 10f, 0f), Vector3.right);
            Check.That("Bounds.IntersectRay misses offset ray", !b.IntersectRay(missRay));

            b.Encapsulate(new Vector3(3f, 0f, 0f));
            Check.That("Bounds.Encapsulate expands max.x to 3", b.max == new Vector3(3f, 2f, 3f));
            Check.That("Bounds.Encapsulate preserves min.x at -1", b.min == new Vector3(-1f, -2f, -3f));
        }

        private static void TestMatrix4x4Surface()
        {
            Matrix4x4 id = Matrix4x4.identity;
            Check.That("Matrix4x4.identity.isIdentity", id.isIdentity);
            Check.Equal("Matrix4x4.identity.determinant", 1f, id.determinant);

            Vector3 pos = new Vector3(3f, -4f, 5f);
            Quaternion rot = Quaternion.Euler(0f, 90f, 0f);
            Vector3 scale = new Vector3(2f, 3f, 4f);
            Matrix4x4 trs = Matrix4x4.TRS(pos, rot, scale);

            Check.That("Matrix4x4.TRS.ValidTRS", trs.ValidTRS());
            Check.That("Matrix4x4.TRS.GetPosition", trs.GetPosition() == pos);
            Check.Equal("Matrix4x4.TRS.lossyScale x", 2f, trs.lossyScale.x);
            Check.Equal("Matrix4x4.TRS.lossyScale y", 3f, trs.lossyScale.y);
            Check.Equal("Matrix4x4.TRS.lossyScale z", 4f, trs.lossyScale.z);
            Check.Equal("Matrix4x4.TRS.rotation angle from rot", 0f, Quaternion.Angle(rot, trs.rotation));

            Vector3 localPt = new Vector3(1f, 2f, 3f);
            Vector3 worldPt = trs.MultiplyPoint3x4(localPt);
            Check.Equal("TRS.MultiplyPoint3x4 x (pos.x + rot(scale*pt).x = 3 + 12)", 15f, worldPt.x);
            Check.Equal("TRS.MultiplyPoint3x4 y (pos.y + rot(scale*pt).y = -4 + 6)", 2f, worldPt.y);
            Check.Equal("TRS.MultiplyPoint3x4 z (pos.z + rot(scale*pt).z = 5 - 2)", 3f, worldPt.z);

            Matrix4x4 inv = trs.inverse;
            Vector3 roundTripPt = inv.MultiplyPoint3x4(worldPt);
            Check.Equal("TRS.inverse.MultiplyPoint3x4 round-trip x", localPt.x, roundTripPt.x);
            Check.Equal("TRS.inverse.MultiplyPoint3x4 round-trip y", localPt.y, roundTripPt.y);
            Check.Equal("TRS.inverse.MultiplyPoint3x4 round-trip z", localPt.z, roundTripPt.z);

            Matrix4x4 product = trs * inv;
            Check.That("TRS * TRS.inverse is identity", product.isIdentity);
        }

        private static void TestTransformRotationEulerAndAxes()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject parent = Track(created, new GameObject("RotParent"));
                GameObject child = Track(created, new GameObject("RotChild"));
                child.transform.SetParent(parent.transform, false);

                parent.transform.rotation = Quaternion.Euler(0f, 90f, 0f);
                Check.Equal("Parent eulerAngles.y is 90", 90f, parent.transform.eulerAngles.y);
                Check.Equal("Parent forward is +x", 1f, parent.transform.forward.x);
                Check.Equal("Parent forward.z is 0", 0f, parent.transform.forward.z);
                Check.Equal("Parent right is -z", -1f, parent.transform.right.z);
                Check.Equal("Parent up is +y", 1f, parent.transform.up.y);

                child.transform.localEulerAngles = new Vector3(0f, 90f, 0f);
                Check.Equal("Child localEulerAngles.y is 90", 90f, child.transform.localEulerAngles.y);
                Check.Equal("Child world eulerAngles.y is 180", 180f, child.transform.eulerAngles.y);
                Check.Equal("Child world forward is -z", -1f, child.transform.forward.z);

                child.transform.eulerAngles = new Vector3(0f, 45f, 0f);
                Check.Equal("Setting child world eulerAngles updates world eulerAngles.y", 45f, child.transform.eulerAngles.y);
                Check.Equal("Setting child world eulerAngles updates localEulerAngles.y to 315", 315f, child.transform.localEulerAngles.y);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestTransformScaleAndTrsSpaces()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject parent = Track(created, new GameObject("TrsParent"));
                GameObject child = Track(created, new GameObject("TrsChild"));
                child.transform.SetParent(parent.transform, false);

                parent.transform.position = new Vector3(10f, 0f, 0f);
                parent.transform.rotation = Quaternion.Euler(0f, 90f, 0f);
                parent.transform.localScale = new Vector3(2f, 2f, 2f);

                child.transform.localPosition = new Vector3(0f, 0f, 3f);
                child.transform.localScale = new Vector3(1.5f, 1.5f, 1.5f);

                Check.Equal("Child world position x under rotated/scaled parent", 16f, child.transform.position.x);
                Check.Equal("Child world position y", 0f, child.transform.position.y);
                Check.Equal("Child world position z", 0f, child.transform.position.z);

                Check.Equal("Child lossyScale x", 3f, child.transform.lossyScale.x);
                Check.Equal("Child lossyScale y", 3f, child.transform.lossyScale.y);
                Check.Equal("Child lossyScale z", 3f, child.transform.lossyScale.z);

                Vector3 localPt = new Vector3(1f, 2f, 3f);
                Vector3 worldPt = child.transform.TransformPoint(localPt);
                Vector3 backLocalPt = child.transform.InverseTransformPoint(worldPt);
                Check.Equal("TransformPoint -> InverseTransformPoint x", localPt.x, backLocalPt.x);
                Check.Equal("TransformPoint -> InverseTransformPoint y", localPt.y, backLocalPt.y);
                Check.Equal("TransformPoint -> InverseTransformPoint z", localPt.z, backLocalPt.z);

                Vector3 localVec = new Vector3(1f, 0f, 0f);
                Vector3 worldVec = child.transform.TransformVector(localVec);
                Check.Equal("TransformVector applies scale and rotation (z = -3)", -3f, worldVec.z);
                Vector3 backVec = child.transform.InverseTransformVector(worldVec);
                Check.Equal("InverseTransformVector restores x", 1f, backVec.x);

                Vector3 worldDir = child.transform.TransformDirection(localVec);
                Check.Equal("TransformDirection ignores scale and applies rotation (z = -1)", -1f, worldDir.z);
                Check.Equal("TransformDirection preserves unit length", 1f, worldDir.magnitude);
                Vector3 backDir = child.transform.InverseTransformDirection(worldDir);
                Check.Equal("InverseTransformDirection restores x", 1f, backDir.x);

                child.transform.position = new Vector3(10f, 4f, -2f);
                Check.Equal("Setting child world position updates localPosition.x", 1f, child.transform.localPosition.x);
                Check.Equal("Setting child world position updates localPosition.y", 2f, child.transform.localPosition.y);
                Check.Equal("Setting child world position updates localPosition.z", 0f, child.transform.localPosition.z);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestTransformRotateAndLookAt()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject actor = Track(created, new GameObject("Actor"));
                actor.transform.Rotate(0f, 90f, 0f);
                Check.Equal("Rotate(0, 90, 0) turns forward to +x", 1f, actor.transform.forward.x);
                Check.Equal("Rotate(0, 90, 0) forward.z is 0", 0f, actor.transform.forward.z);

                actor.transform.Rotate(Vector3.up, 90f, Space.World);
                Check.Equal("Rotate(up, 90, World) turns forward to -z", -1f, actor.transform.forward.z);

                GameObject target = Track(created, new GameObject("LookTarget"));
                target.transform.position = new Vector3(0f, 0f, 10f);
                actor.transform.position = new Vector3(0f, 0f, 0f);
                actor.transform.LookAt(target.transform);
                Check.Equal("LookAt(target) aligns forward with +z", 1f, actor.transform.forward.z);
                Check.Equal("LookAt(target) forward.x is 0", 0f, actor.transform.forward.x);

                actor.transform.LookAt(new Vector3(-5f, 0f, 0f));
                Check.Equal("LookAt(worldPos) aligns forward with -x", -1f, actor.transform.forward.x);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestTransformSetParentWorldPositionStaysAndNegativeScale()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject negParent = Track(created, new GameObject("NegativelyScaledParent"));
                negParent.transform.position = new Vector3(4f, -2f, 6f);
                negParent.transform.rotation = Quaternion.Euler(0f, 90f, 0f);
                negParent.transform.localScale = new Vector3(-2f, 2f, 2f);

                GameObject child = Track(created, new GameObject("PoseChild"));
                Vector3 initialWorldPos = new Vector3(10f, 4f, -2f);
                Quaternion initialWorldRot = Quaternion.Euler(0f, 30f, 0f);
                Vector3 initialWorldScale = new Vector3(1f, 3f, 5f);
                child.transform.position = initialWorldPos;
                child.transform.rotation = initialWorldRot;
                child.transform.localScale = initialWorldScale;

                Vector3 expectedPtWorld = child.transform.TransformPoint(new Vector3(1f, 2f, 3f));

                child.transform.SetParent(negParent.transform, true);

                Check.Equal("SetParent(true) under negative-scale parent preserves world position x", initialWorldPos.x, child.transform.position.x);
                Check.Equal("SetParent(true) under negative-scale parent preserves world position y", initialWorldPos.y, child.transform.position.y);
                Check.Equal("SetParent(true) under negative-scale parent preserves world position z", initialWorldPos.z, child.transform.position.z);
                Check.Equal("SetParent(true) under negative-scale parent preserves world rotation", 0f, Quaternion.Angle(initialWorldRot, child.transform.rotation));
                Check.Equal("SetParent(true) under negative-scale parent preserves localScale x sign (-0.5)", -0.5f, child.transform.localScale.x);
                Check.Equal("SetParent(true) under negative-scale parent preserves localScale y (1.5)", 1.5f, child.transform.localScale.y);
                Check.Equal("SetParent(true) under negative-scale parent preserves localScale z (2.5)", 2.5f, child.transform.localScale.z);

                Vector3 actualPtWorld = child.transform.TransformPoint(new Vector3(1f, 2f, 3f));
                Check.Equal("SetParent(true) under negative-scale parent preserves world TransformPoint x", expectedPtWorld.x, actualPtWorld.x);
                Check.Equal("SetParent(true) under negative-scale parent preserves world TransformPoint y", expectedPtWorld.y, actualPtWorld.y);
                Check.Equal("SetParent(true) under negative-scale parent preserves world TransformPoint z", expectedPtWorld.z, actualPtWorld.z);

                child.transform.SetParent(null, true);
                Check.Equal("Unparenting with worldPositionStays=true preserves world position x", initialWorldPos.x, child.transform.position.x);
                Check.Equal("Unparenting with worldPositionStays=true preserves world position y", initialWorldPos.y, child.transform.position.y);
                Check.Equal("Unparenting with worldPositionStays=true preserves world position z", initialWorldPos.z, child.transform.position.z);

                GameObject localChild = Track(created, new GameObject("LocalChild"));
                localChild.transform.localPosition = new Vector3(1f, 2f, 3f);
                localChild.transform.localRotation = Quaternion.Euler(10f, 20f, 30f);
                localChild.transform.localScale = new Vector3(2f, 2f, 2f);
                localChild.transform.SetParent(negParent.transform, false);
                Check.That("SetParent(false) keeps localPosition unchanged", localChild.transform.localPosition == new Vector3(1f, 2f, 3f));
                Check.That("SetParent(false) keeps localScale unchanged", localChild.transform.localScale == new Vector3(2f, 2f, 2f));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestTransformSiblingIndexAndFind()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject parent = Track(created, new GameObject("SiblingParent"));
                GameObject c0 = Track(created, new GameObject("ChildA"));
                GameObject c1 = Track(created, new GameObject("ChildB"));
                GameObject c2 = Track(created, new GameObject("ChildC"));
                c0.transform.SetParent(parent.transform, false);
                c1.transform.SetParent(parent.transform, false);
                c2.transform.SetParent(parent.transform, false);

                Check.Equal("Initial sibling index of ChildA is 0", 0, c0.transform.GetSiblingIndex());
                Check.Equal("Initial sibling index of ChildB is 1", 1, c1.transform.GetSiblingIndex());
                Check.Equal("Initial sibling index of ChildC is 2", 2, c2.transform.GetSiblingIndex());

                c2.transform.SetSiblingIndex(0);
                Check.Equal("After SetSiblingIndex(0), ChildC is at 0", 0, c2.transform.GetSiblingIndex());
                Check.Equal("After SetSiblingIndex(0), ChildA shifts to 1", 1, c0.transform.GetSiblingIndex());
                Check.Equal("After SetSiblingIndex(0), ChildB shifts to 2", 2, c1.transform.GetSiblingIndex());
                Check.That("GetChild(0) returns ChildC", ReferenceEquals(parent.transform.GetChild(0), c2.transform));

                c2.transform.SetAsLastSibling();
                Check.Equal("SetAsLastSibling moves ChildC to index 2", 2, c2.transform.GetSiblingIndex());

                c1.transform.SetAsFirstSibling();
                Check.Equal("SetAsFirstSibling moves ChildB to index 0", 0, c1.transform.GetSiblingIndex());

                GameObject deep = Track(created, new GameObject("DeepLeaf"));
                deep.transform.SetParent(c0.transform, false);
                Check.That("Transform.Find direct child", ReferenceEquals(parent.transform.Find("ChildA"), c0.transform));
                Check.That("Transform.Find relative path", ReferenceEquals(parent.transform.Find("ChildA/DeepLeaf"), deep.transform));
                Check.That("Transform.Find missing returns null", parent.transform.Find("NonExistent") == null);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestGetComponentsInChildrenAndParent()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject root = Track(created, new GameObject("QueryRoot"));
                GameObject branchA = Track(created, new GameObject("BranchA"));
                GameObject leafA1 = Track(created, new GameObject("LeafA1"));
                GameObject branchB = Track(created, new GameObject("BranchB"));

                branchA.transform.SetParent(root.transform, false);
                leafA1.transform.SetParent(branchA.transform, false);
                branchB.transform.SetParent(root.transform, false);

                CoreBaseProbeComponent pRoot = root.AddComponent<CoreBaseProbeComponent>();
                pRoot.ProbeId = 1;
                CoreDerivedProbeComponent pBranchA = branchA.AddComponent<CoreDerivedProbeComponent>();
                pBranchA.ProbeId = 2;
                CoreBaseProbeComponent pLeafA1 = leafA1.AddComponent<CoreBaseProbeComponent>();
                pLeafA1.ProbeId = 3;
                CoreBaseProbeComponent pBranchB = branchB.AddComponent<CoreBaseProbeComponent>();
                pBranchB.ProbeId = 4;

                CoreBaseProbeComponent[] allActive = root.GetComponentsInChildren<CoreBaseProbeComponent>();
                Check.Equal("GetComponentsInChildren<T>() returns all 4 active in pre-order", 4, allActive.Length);
                if (allActive.Length == 4)
                {
                    Check.Equal("GetComponentsInChildren index 0 is root (1)", 1, allActive[0].ProbeId);
                    Check.Equal("GetComponentsInChildren index 1 is branchA (2)", 2, allActive[1].ProbeId);
                    Check.Equal("GetComponentsInChildren index 2 is leafA1 (3)", 3, allActive[2].ProbeId);
                    Check.Equal("GetComponentsInChildren index 3 is branchB (4)", 4, allActive[3].ProbeId);
                }

                ICoreProbeMarker[] interfaceMatches = root.GetComponentsInChildren<ICoreProbeMarker>();
                Check.Equal("GetComponentsInChildren<ICoreProbeMarker> supports interface lookup", 4, interfaceMatches.Length);

                branchA.SetActive(false);
                CoreBaseProbeComponent[] activeOnly = root.GetComponentsInChildren<CoreBaseProbeComponent>(false);
                Check.Equal("GetComponentsInChildren(false) skips inactive branchA and its active child leafA1", 2, activeOnly.Length);
                if (activeOnly.Length == 2)
                {
                    Check.Equal("Active-only index 0 is root (1)", 1, activeOnly[0].ProbeId);
                    Check.Equal("Active-only index 1 is branchB (4)", 4, activeOnly[1].ProbeId);
                }

                CoreBaseProbeComponent[] withInactive = root.transform.GetComponentsInChildren<CoreBaseProbeComponent>(true);
                Check.Equal("Component.GetComponentsInChildren(true) includes inactive subtree", 4, withInactive.Length);

                CoreBaseProbeComponent[] upwardsActive = leafA1.GetComponentsInParent<CoreBaseProbeComponent>(false);
                Check.Equal("GetComponentsInParent(false) from leaf under inactive branchA returns 0 because leaf & branchA are inactive in hierarchy, except root is active!", 1, upwardsActive.Length);
                if (upwardsActive.Length == 1)
                {
                    Check.Equal("GetComponentsInParent(false) finds active root (1)", 1, upwardsActive[0].ProbeId);
                }

                CoreBaseProbeComponent[] upwardsAll = leafA1.transform.GetComponentsInParent<CoreBaseProbeComponent>(true);
                Check.Equal("Component.GetComponentsInParent(true) finds leaf, branchA, and root in upward order", 3, upwardsAll.Length);
                if (upwardsAll.Length == 3)
                {
                    Check.Equal("GetComponentsInParent(true) index 0 is leafA1 (3)", 3, upwardsAll[0].ProbeId);
                    Check.Equal("GetComponentsInParent(true) index 1 is branchA (2)", 2, upwardsAll[1].ProbeId);
                    Check.Equal("GetComponentsInParent(true) index 2 is root (1)", 1, upwardsAll[2].ProbeId);
                }
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestTryGetComponentAndListOverloads()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject go = Track(created, new GameObject("TryGetHost"));
                CoreDerivedProbeComponent derived = go.AddComponent<CoreDerivedProbeComponent>();
                derived.ProbeId = 77;

                bool foundBase = go.TryGetComponent<CoreBaseProbeComponent>(out CoreBaseProbeComponent baseComp);
                Check.That("GameObject.TryGetComponent<Base> returns true", foundBase);
                Check.That("GameObject.TryGetComponent<Base> outputs component", ReferenceEquals(baseComp, derived));

                bool foundViaComp = go.transform.TryGetComponent<ICoreProbeMarker>(out ICoreProbeMarker markerComp);
                Check.That("Component.TryGetComponent<Interface> returns true", foundViaComp);
                Check.Equal("Component.TryGetComponent<Interface> marker value", 77, markerComp.MarkerValue);

                bool foundMissing = go.TryGetComponent<CoreOtherProbeComponent>(out CoreOtherProbeComponent? missingComp);
                Check.That("GameObject.TryGetComponent<Missing> returns false", !foundMissing);
                Check.That("GameObject.TryGetComponent<Missing> sets out to null", missingComp == null);

                bool foundByType = go.transform.TryGetComponent(typeof(CoreDerivedProbeComponent), out Component? byTypeComp);
                Check.That("Component.TryGetComponent(Type) returns true", foundByType);
                Check.That("Component.TryGetComponent(Type) outputs component", ReferenceEquals(byTypeComp, derived));

                List<CoreBaseProbeComponent> buffer = new List<CoreBaseProbeComponent>();
                go.GetComponents<CoreBaseProbeComponent>(buffer);
                Check.Equal("GameObject.GetComponents into buffer populates count", 1, buffer.Count);

                go.transform.GetComponentsInChildren<CoreBaseProbeComponent>(buffer);
                Check.Equal("Component.GetComponentsInChildren into buffer populates count", 1, buffer.Count);

                go.transform.GetComponentsInParent<CoreBaseProbeComponent>(false, buffer);
                Check.Equal("Component.GetComponentsInParent into buffer populates count", 1, buffer.Count);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static GameObject Track(List<UnityEngine.Object> list, GameObject instance)
        {
            list.Add(instance);
            return instance;
        }

        private static void DestroyAll(List<UnityEngine.Object> list)
        {
            for (int i = list.Count - 1; i >= 0; i--)
            {
                UnityEngine.Object.Destroy(list[i]);
            }
        }
    }
}
