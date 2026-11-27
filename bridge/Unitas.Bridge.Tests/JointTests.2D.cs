using System;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static partial class JointTests
    {
        private static void RunJoint2DTests()
        {
            TestJoint2DComponentDefaultsAndProperties();
            TestDistanceSpringAndTargetJoint2DSimulationOverBridge();
            TestFixedRelativeAndFrictionJoint2DSimulationOverBridge();
            TestHingeSliderAndWheelJoint2DSimulationOverBridge();
        }

        private static void TestJoint2DComponentDefaultsAndProperties()
        {
            using (GameObject.SuppressRuntimeCreation())
            {
                GameObject go = new GameObject("Joint2DDefaults");
                DistanceJoint2D dj = go.AddComponent<DistanceJoint2D>();
                Check.That("DistanceJoint2D inherits AnchoredJoint2D and Joint2D", dj is AnchoredJoint2D && dj is Joint2D);
                Check.Equal("DistanceJoint2D default distance", 1f, dj.distance);
                Check.Equal("DistanceJoint2D default autoConfigureDistance", true, dj.autoConfigureDistance);
                Check.That("Joint2D default breakForce", float.IsPositiveInfinity(dj.breakForce));
                Check.That("Joint2D default breakTorque", float.IsPositiveInfinity(dj.breakTorque));

                FixedJoint2D fj = go.AddComponent<FixedJoint2D>();
                Check.Equal("FixedJoint2D default frequency", 2f, fj.frequency);

                FrictionJoint2D frj = go.AddComponent<FrictionJoint2D>();
                Check.Equal("FrictionJoint2D default maxForce", 1f, frj.maxForce);
                Check.Equal("FrictionJoint2D default maxTorque", 1f, frj.maxTorque);

                HingeJoint2D hj = go.AddComponent<HingeJoint2D>();
                Check.Equal("HingeJoint2D default motor maxMotorTorque", 10000f, hj.motor.maxMotorTorque);

                RelativeJoint2D rj = go.AddComponent<RelativeJoint2D>();
                Check.Equal("RelativeJoint2D default correctionScale", 0.3f, rj.correctionScale);

                SliderJoint2D slj = go.AddComponent<SliderJoint2D>();
                Check.Equal("SliderJoint2D default autoConfigureAngle", true, slj.autoConfigureAngle);

                SpringJoint2D sj = go.AddComponent<SpringJoint2D>();
                Check.Equal("SpringJoint2D default distance", 1f, sj.distance);
                Check.Equal("SpringJoint2D default frequency", 2f, sj.frequency);

                TargetJoint2D tj = go.AddComponent<TargetJoint2D>();
                Check.Equal("TargetJoint2D default maxForce", 10000f, tj.maxForce);
                Check.Equal("TargetJoint2D default frequency", 5f, tj.frequency);
                Check.Equal("TargetJoint2D default dampingRatio", 1f, tj.dampingRatio);

                WheelJoint2D wj = go.AddComponent<WheelJoint2D>();
                Check.Equal("WheelJoint2D default suspension angle", 90f, wj.suspension.angle);
                Check.Equal("WheelJoint2D default suspension dampingRatio", 0.7f, wj.suspension.dampingRatio);
                Check.Equal("WheelJoint2D default suspension frequency", 2f, wj.suspension.frequency);
            }
        }

        private static void TestDistanceSpringAndTargetJoint2DSimulationOverBridge()
        {
            using (BridgeHost host = new BridgeHost())
            {
                Init2DScene(host);
                Physics2D.gravity = Vector2.zero;

                GameObject anchorGo = CreateBody2D("Anchor2D", Vector2.zero, 0f, RigidbodyType2D.Static);
                Rigidbody2D anchorRb = anchorGo.GetComponent<Rigidbody2D>()!;

                GameObject distGo = CreateBody2D("DistBody2D", new Vector2(5f, 0f), 0f, RigidbodyType2D.Dynamic);
                DistanceJoint2D dj = distGo.AddComponent<DistanceJoint2D>();
                dj.connectedBody = anchorRb;
                dj.distance = 2f;
                dj.maxDistanceOnly = false;
                dj.SyncToNative();

                GameObject sprGo = CreateBody2D("SpringBody2D", new Vector2(4f, 0f), 0f, RigidbodyType2D.Dynamic);
                SpringJoint2D sj = sprGo.AddComponent<SpringJoint2D>();
                sj.connectedBody = anchorRb;
                sj.distance = 1f;
                sj.dampingRatio = 0.5f;
                sj.frequency = 4f;
                sj.SyncToNative();

                GameObject tgtGo = CreateBody2D("TargetBody2D", Vector2.zero, 0f, RigidbodyType2D.Dynamic);
                TargetJoint2D tj = tgtGo.AddComponent<TargetJoint2D>();
                tj.target = new Vector2(3f, 4f);
                tj.maxForce = 100f;
                tj.frequency = 5f;
                tj.dampingRatio = 0.7f;
                tj.SyncToNative();

                Step2D(host, 5, 0.02);

                Check.That("DistanceJoint2D pulls body to target distance 2.0",
                    host.TryGetBodyPose2D(distGo.NativeEntityId, out Vector2 distPos, out _)
                    && Mathf.Abs(distPos.x - 2f) < 0.1f);
                Check.That("SpringJoint2D accelerates stretched body toward rest distance",
                    host.TryGetBodyPose2D(sprGo.NativeEntityId, out Vector2 sprPos, out _)
                    && sprPos.x < 3.9f);
                Check.That("TargetJoint2D pulls body toward world target [3, 4]",
                    host.TryGetBodyPose2D(tgtGo.NativeEntityId, out Vector2 tgtPos, out _)
                    && tgtPos.x > 0.1f && tgtPos.y > 0.1f);
            }
        }

        private static void TestFixedRelativeAndFrictionJoint2DSimulationOverBridge()
        {
            using (BridgeHost host = new BridgeHost())
            {
                Init2DScene(host);
                Physics2D.gravity = Vector2.zero;

                GameObject anchorGo = CreateBody2D("AnchorFix2D", Vector2.zero, 0f, RigidbodyType2D.Static);
                Rigidbody2D anchorRb = anchorGo.GetComponent<Rigidbody2D>()!;

                GameObject fixGo = CreateBody2D("FixedBody2D", new Vector2(3f, -2f), 45f, RigidbodyType2D.Dynamic);
                FixedJoint2D fj = fixGo.AddComponent<FixedJoint2D>();
                fj.connectedBody = anchorRb;
                fj.connectedAnchor = new Vector2(1f, 2f);
                fj.referenceAngle = 0.25f;
                fj.SyncToNative();

                GameObject relGo = CreateBody2D("RelativeBody2D", new Vector2(5f, 5f), 0f, RigidbodyType2D.Dynamic);
                RelativeJoint2D rj = relGo.AddComponent<RelativeJoint2D>();
                rj.connectedBody = anchorRb;
                rj.linearOffset = new Vector2(-1f, 0.5f);
                rj.angularOffset = -0.5f;
                rj.maxForce = 500f;
                rj.maxTorque = 500f;
                rj.correctionScale = 1f;
                rj.SyncToNative();

                GameObject fricGo = CreateBody2D("FrictionBody2D", Vector2.zero, 0f, RigidbodyType2D.Dynamic);
                FrictionJoint2D frj = fricGo.AddComponent<FrictionJoint2D>();
                frj.connectedBody = anchorRb;
                frj.maxForce = 10f;
                frj.maxTorque = 6f;
                frj.SyncToNative();
                fricGo.GetComponent<Rigidbody2D>()!.velocity = new Vector2(10f, 0f);

                Step2D(host, 6, 0.02);

                Check.That("FixedJoint2D locks relative offset [1, 2] and reference angle 0.25",
                    host.TryGetBodyPose2D(fixGo.NativeEntityId, out Vector2 fixPos, out float fixRot)
                    && Mathf.Abs(fixPos.x - 1f) < 1e-2f
                    && Mathf.Abs(fixPos.y - 2f) < 1e-2f
                    && Mathf.Abs(fixRot - 0.25f) < 1e-2f);
                Check.That("RelativeJoint2D drives body toward linear and angular offsets",
                    host.TryGetBodyPose2D(relGo.NativeEntityId, out Vector2 relPos, out float relRot)
                    && relPos.x < 4.5f && relRot < 0f);
                Check.That("FrictionJoint2D decelerates linear velocity below initial 10.0",
                    fricGo.GetComponent<Rigidbody2D>()!.velocity.x < 9.5f);
            }
        }

        private static void TestHingeSliderAndWheelJoint2DSimulationOverBridge()
        {
            using (BridgeHost host = new BridgeHost())
            {
                Init2DScene(host);
                Physics2D.gravity = Vector2.zero;

                GameObject anchorGo = CreateBody2D("AnchorMotor2D", Vector2.zero, 0f, RigidbodyType2D.Static);
                Rigidbody2D anchorRb = anchorGo.GetComponent<Rigidbody2D>()!;

                GameObject hingeGo = CreateBody2D("HingeBody2D", Vector2.zero, 0f, RigidbodyType2D.Dynamic);
                HingeJoint2D hj = hingeGo.AddComponent<HingeJoint2D>();
                hj.connectedBody = anchorRb;
                hj.useMotor = true;
                hj.motor = new JointMotor2D { motorSpeed = 10f, maxMotorTorque = 100f };
                hj.useLimits = true;
                hj.limits = new JointAngleLimits2D { min = -0.2f, max = 0.3f };
                hj.SyncToNative();

                GameObject slideGo = CreateBody2D("SliderBody2D", new Vector2(0f, 3f), 0f, RigidbodyType2D.Dynamic);
                SliderJoint2D slj = slideGo.AddComponent<SliderJoint2D>();
                slj.connectedBody = anchorRb;
                slj.angle = 0f;
                slj.useMotor = true;
                slj.motor = new JointMotor2D { motorSpeed = 20f, maxMotorTorque = 500f };
                slj.useLimits = true;
                slj.limits = new JointTranslationLimits2D { min = -1f, max = 1.5f };
                slj.SyncToNative();

                GameObject wheelGo = CreateBody2D("WheelBody2D", new Vector2(0f, -2f), 0f, RigidbodyType2D.Dynamic);
                WheelJoint2D wj = wheelGo.AddComponent<WheelJoint2D>();
                wj.connectedBody = anchorRb;
                wj.suspension = new JointSuspension2D { angle = 90f, frequency = 6f, dampingRatio = 0.7f };
                wj.useMotor = true;
                wj.motor = new JointMotor2D { motorSpeed = 6f, maxMotorTorque = 50f };
                wj.SyncToNative();

                Step2D(host, 8, 0.02);

                Check.That("HingeJoint2D motor and upper angular limit clamp rotation to 0.3",
                    host.TryGetBodyPose2D(hingeGo.NativeEntityId, out _, out float hingeRot)
                    && Mathf.Abs(hingeRot - 0.3f) < 1e-2f);
                Check.That("SliderJoint2D projects body onto slide axis and clamps translation to limit 1.5",
                    host.TryGetBodyPose2D(slideGo.NativeEntityId, out Vector2 slidePos, out _)
                    && Mathf.Abs(slidePos.y) < 1e-2f && Mathf.Abs(slidePos.x - 1.5f) < 2e-2f);
                Check.That("WheelJoint2D suspension and motor pull wheel toward anchor and spin it",
                    host.TryGetBodyPose2D(wheelGo.NativeEntityId, out Vector2 wheelPos, out float wheelRot)
                    && wheelPos.y > -2f && wheelRot > 0.05f);
            }
        }

        private static void Init2DScene(BridgeHost host)
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_j2d_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "Empty2D.unity");
            File.WriteAllText(scenePath, EmptyZeroGravity3DSceneYaml);
            host.LoadScene(scenePath);
            Directory.Delete(tempDir, true);
        }

        private static GameObject CreateBody2D(string name, Vector2 pos, float angleDeg, RigidbodyType2D bodyType)
        {
            GameObject go = new GameObject(name);
            go.transform.position = new Vector3(pos.x, pos.y, 0f);
            go.transform.rotation = Quaternion.Euler(0f, 0f, angleDeg);
            Rigidbody2D rb = go.AddComponent<Rigidbody2D>();
            rb.bodyType = bodyType;
            rb.gravityScale = 0f;
            return go;
        }

        private static void Step2D(BridgeHost host, int steps, double dt)
        {
            for (int i = 0; i < steps; i++)
            {
                host.Tick(dt);
                host.SyncJointTransformsAfterStep();
            }
        }
    }
}
