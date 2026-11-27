using System;
using System.IO;
using System.Runtime.CompilerServices;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    internal static class RigidbodyDepthTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnTargetShard()
        {
            string? shard = Environment.GetEnvironmentVariable("UNITAS_TEST_SHARD");
            if (string.Equals(shard, "RigidbodyDepthTests", StringComparison.OrdinalIgnoreCase))
            {
                RigidbodyDepthTests.Run();
            }
        }
    }

    public static class RigidbodyDepthTests
    {
        private const string ProbeGuid = "99112233445566778899aabbccddeeff";

        public static void Run()
        {
            TestEnumsAndDefaults();
            TestAngularVelocityAndAddTorque();
            TestAddRelativeForceAndTorque();
            TestAddForceAtPosition();
            TestAddExplosionForce();
            TestSleepAndWakeUp();
            TestInterpolationAndConstraints();
            TestNativeBridgeEndToEnd();
        }

        private static void TestEnumsAndDefaults()
        {
            Check.Equal("RigidbodyInterpolation.None value is 0", 0, (int)RigidbodyInterpolation.None);
            Check.Equal("RigidbodyInterpolation.Interpolate value is 1", 1, (int)RigidbodyInterpolation.Interpolate);
            Check.Equal("RigidbodyInterpolation.Extrapolate value is 2", 2, (int)RigidbodyInterpolation.Extrapolate);

            Check.Equal("RigidbodyConstraints.None is 0", 0, (int)RigidbodyConstraints.None);
            Check.Equal("RigidbodyConstraints.FreezePositionX is 2", 2, (int)RigidbodyConstraints.FreezePositionX);
            Check.Equal("RigidbodyConstraints.FreezePositionY is 4", 4, (int)RigidbodyConstraints.FreezePositionY);
            Check.Equal("RigidbodyConstraints.FreezePositionZ is 8", 8, (int)RigidbodyConstraints.FreezePositionZ);
            Check.Equal("RigidbodyConstraints.FreezeRotationX is 16", 16, (int)RigidbodyConstraints.FreezeRotationX);
            Check.Equal("RigidbodyConstraints.FreezeRotationY is 32", 32, (int)RigidbodyConstraints.FreezeRotationY);
            Check.Equal("RigidbodyConstraints.FreezeRotationZ is 64", 64, (int)RigidbodyConstraints.FreezeRotationZ);
            Check.Equal(
                "RigidbodyConstraints.FreezePosition combines X, Y, Z",
                (int)(RigidbodyConstraints.FreezePositionX | RigidbodyConstraints.FreezePositionY | RigidbodyConstraints.FreezePositionZ),
                (int)RigidbodyConstraints.FreezePosition);
            Check.Equal(
                "RigidbodyConstraints.FreezeRotation combines X, Y, Z",
                (int)(RigidbodyConstraints.FreezeRotationX | RigidbodyConstraints.FreezeRotationY | RigidbodyConstraints.FreezeRotationZ),
                (int)RigidbodyConstraints.FreezeRotation);
            Check.Equal(
                "RigidbodyConstraints.FreezeAll combines FreezePosition and FreezeRotation",
                (int)(RigidbodyConstraints.FreezePosition | RigidbodyConstraints.FreezeRotation),
                (int)RigidbodyConstraints.FreezeAll);

            GameObject go = new GameObject("RbDefault");
            Rigidbody rb = go.AddComponent<Rigidbody>();
            Check.Equal("Default angularVelocity X is 0", 0f, rb.angularVelocity.x);
            Check.Equal("Default angularVelocity Y is 0", 0f, rb.angularVelocity.y);
            Check.Equal("Default angularVelocity Z is 0", 0f, rb.angularVelocity.z);
            Check.Equal("Default interpolation is None", (int)RigidbodyInterpolation.None, (int)rb.interpolation);
            Check.Equal("Default constraints is None", (int)RigidbodyConstraints.None, (int)rb.constraints);
            Check.Equal("Default IsSleeping is false", false, rb.IsSleeping());
        }

        private static void TestAngularVelocityAndAddTorque()
        {
            GameObject go = new GameObject("RbTorque");
            Rigidbody rb = go.AddComponent<Rigidbody>();
            rb.mass = 2f;

            rb.angularVelocity = new Vector3(1f, 2f, 3f);
            Check.Equal("angularVelocity setter stores X", 1f, rb.angularVelocity.x);
            Check.Equal("angularVelocity setter stores Y", 2f, rb.angularVelocity.y);
            Check.Equal("angularVelocity setter stores Z", 3f, rb.angularVelocity.z);

            rb.angularVelocity = Vector3.zero;
            rb.AddTorque(new Vector3(0f, 6f, 0f), ForceMode.Impulse);
            Check.That("AddTorque Impulse increases angularVelocity Y", rb.angularVelocity.y > 0f);

            rb.angularVelocity = Vector3.zero;
            rb.AddTorque(4f, 0f, -2f, ForceMode.VelocityChange);
            Check.Equal("AddTorque(x,y,z, VelocityChange) sets X directly", 4f, rb.angularVelocity.x);
            Check.Equal("AddTorque(x,y,z, VelocityChange) sets Z directly", -2f, rb.angularVelocity.z);

            rb.isKinematic = true;
            Vector3 beforeKinematic = rb.angularVelocity;
            rb.AddTorque(new Vector3(10f, 10f, 10f), ForceMode.Impulse);
            Check.Equal("Kinematic AddTorque is a no-op on angularVelocity X", beforeKinematic.x, rb.angularVelocity.x);
        }

        private static void TestAddRelativeForceAndTorque()
        {
            GameObject go = new GameObject("RbRelative");
            go.transform.rotation = Quaternion.Euler(0f, 90f, 0f);
            Rigidbody rb = go.AddComponent<Rigidbody>();
            rb.mass = 1f;

            rb.AddRelativeForce(new Vector3(0f, 0f, 5f), ForceMode.VelocityChange);
            Check.Equal("AddRelativeForce rotates local +Z by 90 deg Y to world +X", 5f, rb.velocity.x);
            Check.That("AddRelativeForce local +Z has near-zero world Z", Mathf.Abs(rb.velocity.z) < 0.001f);

            rb.velocity = Vector3.zero;
            rb.AddRelativeForce(2f, 0f, 0f, ForceMode.VelocityChange);
            Check.Equal("AddRelativeForce(x,y,z) rotates local +X by 90 deg Y to world -Z", -2f, rb.velocity.z);

            rb.angularVelocity = Vector3.zero;
            rb.AddRelativeTorque(new Vector3(0f, 0f, 3f), ForceMode.VelocityChange);
            Check.Equal("AddRelativeTorque rotates local +Z by 90 deg Y to world +X", 3f, rb.angularVelocity.x);

            rb.angularVelocity = Vector3.zero;
            rb.AddRelativeTorque(0f, 4f, 0f, ForceMode.VelocityChange);
            Check.Equal("AddRelativeTorque(x,y,z) preserves Y under Y rotation", 4f, rb.angularVelocity.y);
        }

        private static void TestAddForceAtPosition()
        {
            GameObject go = new GameObject("RbForceAtPos");
            go.transform.position = Vector3.zero;
            Rigidbody rb = go.AddComponent<Rigidbody>();
            rb.mass = 1f;

            rb.AddForceAtPosition(new Vector3(0f, 0f, 4f), new Vector3(1f, 0f, 0f), ForceMode.VelocityChange);
            Check.Equal("AddForceAtPosition adds linear velocity Z", 4f, rb.velocity.z);
            Check.That("AddForceAtPosition off-center produces negative Y torque (r x F)", rb.angularVelocity.y < -0.1f);
        }

        private static void TestAddExplosionForce()
        {
            GameObject go = new GameObject("RbExplosion");
            go.transform.position = new Vector3(2f, 0f, 0f);
            Rigidbody rb = go.AddComponent<Rigidbody>();
            rb.mass = 1f;

            rb.AddExplosionForce(10f, Vector3.zero, 5f, 0f, ForceMode.VelocityChange);
            Check.Equal("AddExplosionForce at d=2, r=5 applies 10*(1 - 2/5)=6 along +X", 6f, rb.velocity.x);
            Check.Equal("AddExplosionForce with upwardsModifier=0 has 0 Y velocity", 0f, rb.velocity.y);

            rb.velocity = Vector3.zero;
            rb.AddExplosionForce(10f, new Vector3(20f, 0f, 0f), 5f, 0f, ForceMode.VelocityChange);
            Check.Equal("AddExplosionForce outside radius applies no force", 0f, rb.velocity.magnitude);

            rb.velocity = Vector3.zero;
            rb.AddExplosionForce(10f, Vector3.zero, 10f, 2f, ForceMode.VelocityChange);
            Check.That("AddExplosionForce with upwardsModifier>0 imparts upward Y velocity", rb.velocity.y > 0f);
        }

        private static void TestSleepAndWakeUp()
        {
            GameObject go = new GameObject("RbSleep");
            Rigidbody rb = go.AddComponent<Rigidbody>();
            rb.velocity = new Vector3(1f, 2f, 3f);
            rb.angularVelocity = new Vector3(1f, 1f, 1f);

            rb.Sleep();
            Check.Equal("IsSleeping is true after Sleep()", true, rb.IsSleeping());
            Check.Equal("Sleep zeroes linear velocity", 0f, rb.velocity.magnitude);
            Check.Equal("Sleep zeroes angular velocity", 0f, rb.angularVelocity.magnitude);

            rb.WakeUp();
            Check.Equal("IsSleeping is false after WakeUp()", false, rb.IsSleeping());

            rb.Sleep();
            Check.Equal("IsSleeping is true again after second Sleep()", true, rb.IsSleeping());
            rb.AddForce(new Vector3(1f, 0f, 0f), ForceMode.Impulse);
            Check.Equal("AddForce wakes sleeping rigidbody", false, rb.IsSleeping());
        }

        private static void TestInterpolationAndConstraints()
        {
            GameObject go = new GameObject("RbInterpConstraints");
            Rigidbody rb = go.AddComponent<Rigidbody>();

            rb.interpolation = RigidbodyInterpolation.Interpolate;
            Check.Equal("interpolation getter returns Interpolate", (int)RigidbodyInterpolation.Interpolate, (int)rb.interpolation);
            rb.interpolation = RigidbodyInterpolation.Extrapolate;
            Check.Equal("interpolation getter returns Extrapolate", (int)RigidbodyInterpolation.Extrapolate, (int)rb.interpolation);

            rb.constraints = RigidbodyConstraints.FreezePositionY | RigidbodyConstraints.FreezeRotationX | RigidbodyConstraints.FreezeRotationZ;
            Check.Equal(
                "constraints getter returns assigned flags",
                (int)(RigidbodyConstraints.FreezePositionY | RigidbodyConstraints.FreezeRotationX | RigidbodyConstraints.FreezeRotationZ),
                (int)rb.constraints);

            rb.velocity = new Vector3(3f, 5f, 7f);
            Check.Equal("FreezePositionY zeroes Y velocity while preserving X", 3f, rb.velocity.x);
            Check.Equal("FreezePositionY zeroes Y velocity component", 0f, rb.velocity.y);
            Check.Equal("FreezePositionY preserves Z velocity", 7f, rb.velocity.z);

            rb.angularVelocity = new Vector3(2f, 4f, 6f);
            Check.Equal("FreezeRotationX zeroes X angularVelocity", 0f, rb.angularVelocity.x);
            Check.Equal("Unfrozen Y angularVelocity is preserved", 4f, rb.angularVelocity.y);
            Check.Equal("FreezeRotationZ zeroes Z angularVelocity", 0f, rb.angularVelocity.z);
        }

        private static void TestNativeBridgeEndToEnd()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_rb_depth_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "rb_depth.unity");
            File.WriteAllText(scenePath, NativeRigidbodyScene);

            try
            {
                ScriptGlueRegistry.Clear();
                RigidbodyDepthProbeScript probe = new RigidbodyDepthProbeScript();
                ScriptGlueRegistry.Register(ProbeGuid, () => probe);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Tick(0.02);
                    host.Shutdown();
                }

                Check.That("Native bridge scene probe Awake executed", probe.RanAwake);
                Check.That("Native bridge AddTorque produced positive angularVelocity Y", probe.AngularVelocityAfterTorque.y > 0.1f);
                Check.That("Native bridge AddRelativeForce produced expected world velocity", probe.VelocityAfterRelativeForce.x > 0.1f);
                Check.That("Native bridge AddForceAtPosition produced both linear and angular velocity", probe.LinearAndAngularFromForceAtPos);
                Check.That("Native bridge AddExplosionForce pushed body away with upward component", probe.ExplosionMovedUpAndAway);
                Check.That("Native bridge Sleep and WakeUp toggled IsSleeping", probe.SleepWakeRoundTripSucceeded);
                Check.That("Native bridge interpolation and constraints round-tripped", probe.InterpolationAndConstraintsSucceeded);
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private sealed class RigidbodyDepthProbeScript : MonoBehaviour
        {
            public bool RanAwake;
            public Vector3 AngularVelocityAfterTorque;
            public Vector3 VelocityAfterRelativeForce;
            public bool LinearAndAngularFromForceAtPos;
            public bool ExplosionMovedUpAndAway;
            public bool SleepWakeRoundTripSucceeded;
            public bool InterpolationAndConstraintsSucceeded;

            public override void Awake()
            {
                RanAwake = true;
                Rigidbody? rb = GetComponent<Rigidbody>();
                if (rb == null)
                {
                    return;
                }

                rb.useGravity = false;
                rb.angularVelocity = Vector3.zero;
                rb.AddTorque(new Vector3(0f, 5f, 0f), ForceMode.VelocityChange);
                AngularVelocityAfterTorque = rb.angularVelocity;

                transform.rotation = Quaternion.Euler(0f, 90f, 0f);
                rb.velocity = Vector3.zero;
                rb.AddRelativeForce(new Vector3(0f, 0f, 4f), ForceMode.VelocityChange);
                VelocityAfterRelativeForce = rb.velocity;

                transform.rotation = Quaternion.identity;
                transform.position = Vector3.zero;
                rb.velocity = Vector3.zero;
                rb.angularVelocity = Vector3.zero;
                rb.AddForceAtPosition(new Vector3(0f, 0f, 3f), new Vector3(1f, 0f, 0f), ForceMode.VelocityChange);
                LinearAndAngularFromForceAtPos = rb.velocity.z > 1f && Mathf.Abs(rb.angularVelocity.y) > 0.1f;

                transform.position = new Vector3(2f, 0f, 0f);
                rb.velocity = Vector3.zero;
                rb.angularVelocity = Vector3.zero;
                rb.AddExplosionForce(10f, Vector3.zero, 5f, 1f, ForceMode.VelocityChange);
                ExplosionMovedUpAndAway = rb.velocity.x > 1f && rb.velocity.y > 0.1f;

                rb.Sleep();
                bool slept = rb.IsSleeping();
                rb.WakeUp();
                bool woke = !rb.IsSleeping();
                SleepWakeRoundTripSucceeded = slept && woke;

                rb.interpolation = RigidbodyInterpolation.Interpolate;
                rb.constraints = RigidbodyConstraints.FreezePositionY | RigidbodyConstraints.FreezeRotationZ;
                rb.velocity = new Vector3(2f, 9f, 3f);
                rb.angularVelocity = new Vector3(1f, 2f, 8f);
                InterpolationAndConstraintsSucceeded =
                    rb.interpolation == RigidbodyInterpolation.Interpolate
                    && rb.constraints == (RigidbodyConstraints.FreezePositionY | RigidbodyConstraints.FreezeRotationZ)
                    && Mathf.Abs(rb.velocity.y) < 0.001f
                    && Mathf.Abs(rb.angularVelocity.z) < 0.001f;
            }
        }

        private const string NativeRigidbodyScene = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &519420021
GameObject:
  m_ObjectHideFlags: 0
  m_Name: ProbeBody
  m_Layer: 0
  m_TagString: Untagged
  m_IsActive: 1
--- !u!4 &1311639322
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 519420021}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!54 &1311639323
Rigidbody:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 519420021}
  m_Mass: 1
  m_UseGravity: 0
--- !u!135 &1311639324
SphereCollider:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 519420021}
  m_Radius: 0.5
  m_Center: {x: 0, y: 0, z: 0}
--- !u!114 &1311639325
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 519420021}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 99112233445566778899aabbccddeeff, type: 3}
";
    }
}
