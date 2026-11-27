using System;
using System.IO;
using System.Runtime.CompilerServices;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static partial class JointTests
    {
        private static bool s_ran;

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestJoint3DComponentDefaultsAndProperties();
            TestFixedAndHingeJoint3DSimulationOverBridge();
            TestSpringAndCharacterJoint3DSimulationOverBridge();
            TestConfigurableJoint3DSimulationOverBridge();
            TestSceneYamlHingeJointMaterializationAndSimulation();
            RunJoint2DTests();
        }

        private static void TestJoint3DComponentDefaultsAndProperties()
        {
            using (GameObject.SuppressRuntimeCreation())
            {
                GameObject go = new GameObject("Joint3DDefaults");
                FixedJoint fj = go.AddComponent<FixedJoint>();
                Check.That("FixedJoint inherits Joint", fj is Joint);
                Check.That("Joint default axis", fj.axis == Vector3.right);
                Check.Equal("Joint default autoConfigureConnectedAnchor", true, fj.autoConfigureConnectedAnchor);
                Check.That("Joint default breakForce", float.IsPositiveInfinity(fj.breakForce));
                Check.That("Joint default breakTorque", float.IsPositiveInfinity(fj.breakTorque));
                Check.Equal("Joint default massScale", 1f, fj.massScale);
                Check.Equal("Joint default connectedMassScale", 1f, fj.connectedMassScale);

                HingeJoint hj = go.AddComponent<HingeJoint>();
                hj.useLimits = true;
                hj.limits = new JointLimits { min = -30f, max = 45f };
                hj.useMotor = true;
                hj.motor = new JointMotor { targetVelocity = 90f, force = 50f, freeSpin = true };
                hj.useSpring = true;
                hj.spring = new JointSpring { spring = 20f, damper = 2f, targetPosition = 15f };
                Check.Equal("HingeJoint limits min", -30f, hj.limits.min);
                Check.Equal("HingeJoint limits max", 45f, hj.limits.max);
                Check.Equal("HingeJoint motor targetVelocity", 90f, hj.motor.targetVelocity);
                Check.Equal("HingeJoint spring targetPosition", 15f, hj.spring.targetPosition);

                SpringJoint sj = go.AddComponent<SpringJoint>();
                Check.Equal("SpringJoint default spring", 10f, sj.spring);
                Check.Equal("SpringJoint default damper", 0.2f, sj.damper);
                Check.Equal("SpringJoint default tolerance", 0.025f, sj.tolerance);

                CharacterJoint cj = go.AddComponent<CharacterJoint>();
                Check.That("CharacterJoint default swingAxis", cj.swingAxis == Vector3.up);
                Check.Equal("CharacterJoint default lowTwistLimit", -20f, cj.lowTwistLimit.limit);
                Check.Equal("CharacterJoint default highTwistLimit", 70f, cj.highTwistLimit.limit);

                ConfigurableJoint cfg = go.AddComponent<ConfigurableJoint>();
                Check.Equal("ConfigurableJoint default xMotion", (int)ConfigurableJointMotion.Free, (int)cfg.xMotion);
                Check.Equal("ConfigurableJoint default xDrive maximumForce", float.MaxValue, cfg.xDrive.maximumForce);
            }
        }

        private static void TestFixedAndHingeJoint3DSimulationOverBridge()
        {
            using (BridgeHost host = new BridgeHost())
            {
                GameObject baseGo = CreateBody3D(host, "Base3D", new Vector3(0f, 5f, 0f), Quaternion.identity, true);
                GameObject fixedChild = CreateBody3D(host, "FixedChild3D", new Vector3(3f, 1f, -2f), Quaternion.Euler(0f, 0f, 45f), false);
                FixedJoint fj = fixedChild.AddComponent<FixedJoint>();
                fj.connectedBody = baseGo.GetComponent<Rigidbody>();
                fj.connectedAnchor = new Vector3(1f, 0f, 0f);
                fj.anchor = new Vector3(-0.5f, 0f, 0f);
                fj.SyncToNative();

                GameObject hingeBase = CreateBody3D(host, "HingeBase3D", Vector3.zero, Quaternion.identity, true);
                GameObject doorGo = CreateBody3D(host, "Door3D", new Vector3(1f, 0f, 0f), Quaternion.identity, false);
                HingeJoint hj = doorGo.AddComponent<HingeJoint>();
                hj.connectedBody = hingeBase.GetComponent<Rigidbody>();
                hj.axis = new Vector3(0f, 0f, 1f);
                hj.connectedAnchor = Vector3.zero;
                hj.anchor = new Vector3(-1f, 0f, 0f);
                hj.useMotor = true;
                hj.motor = new JointMotor { targetVelocity = 6f, force = 100f, freeSpin = false };
                hj.useLimits = true;
                hj.limits = new JointLimits { min = -0.4f, max = 0.5f };
                hj.SyncToNative();

                Step3D(host, 30, 0.02);

                Check.That("FixedJoint holds child at world anchor offset [1.5, 5.0, 0.0]",
                    host.TryGetBodyPosition(fixedChild.NativeEntityId, out Vector3 fixedPos)
                    && Mathf.Abs(fixedPos.x - 1.5f) < 1e-2f
                    && Mathf.Abs(fixedPos.y - 5.0f) < 1e-2f);
                Check.That("HingeJoint motor and upper limit swing door along arc to angle 0.5 rad",
                    host.TryGetBodyPosition(doorGo.NativeEntityId, out Vector3 doorPos)
                    && Mathf.Abs(doorPos.x - Mathf.Cos(0.5f)) < 2e-2f
                    && Mathf.Abs(doorPos.y - Mathf.Sin(0.5f)) < 2e-2f);
            }
        }

        private static void TestSpringAndCharacterJoint3DSimulationOverBridge()
        {
            using (BridgeHost host = new BridgeHost())
            {
                GameObject springAnchor = CreateBody3D(host, "SpringAnchor3D", Vector3.zero, Quaternion.identity, true);
                GameObject springBob = CreateBody3D(host, "SpringBob3D", new Vector3(5f, 0f, 0f), Quaternion.identity, false);
                SpringJoint sj = springBob.AddComponent<SpringJoint>();
                sj.connectedBody = springAnchor.GetComponent<Rigidbody>();
                sj.spring = 40f;
                sj.damper = 8f;
                sj.minDistance = 1f;
                sj.maxDistance = 2f;
                sj.tolerance = 0.01f;
                sj.SyncToNative();

                GameObject pelvis = CreateBody3D(host, "Pelvis3D", new Vector3(0f, 2f, 0f), Quaternion.identity, true);
                GameObject thigh = CreateBody3D(host, "Thigh3D", new Vector3(2f, 0f, 0f), Quaternion.Euler(0f, 0f, 68f), false);
                CharacterJoint cj = thigh.AddComponent<CharacterJoint>();
                cj.connectedBody = pelvis.GetComponent<Rigidbody>();
                cj.connectedAnchor = new Vector3(0f, -0.5f, 0f);
                cj.anchor = new Vector3(0f, 0.5f, 0f);
                cj.axis = Vector3.right;
                cj.swingAxis = Vector3.up;
                cj.lowTwistLimit = new SoftJointLimit { limit = -0.25f };
                cj.highTwistLimit = new SoftJointLimit { limit = 0.25f };
                cj.swing1Limit = new SoftJointLimit { limit = 0.3f };
                cj.swing2Limit = new SoftJointLimit { limit = 0.35f };
                cj.SyncToNative();

                Step3D(host, 80, 0.02);

                Check.That("SpringJoint pulls stretched body from x=5 into [1.0, 2.0] distance band",
                    host.TryGetBodyPosition(springBob.NativeEntityId, out Vector3 bobPos)
                    && bobPos.x >= 0.95f && bobPos.x <= 2.15f);
                Check.That("CharacterJoint keeps thigh anchor at distance 0.5 from pelvis socket [0, 1.5, 0]",
                    host.TryGetBodyPosition(thigh.NativeEntityId, out Vector3 thighPos)
                    && Mathf.Abs((thighPos - new Vector3(0f, 1.5f, 0f)).magnitude - 0.5f) < 1e-2f);
                Check.That("CharacterJoint clamps initial swing angle to swing2Limit 0.35 rad",
                    host.TryGetBodyRotation3D(thigh.NativeEntityId, out Quaternion thighRot)
                    && 2f * Mathf.Acos(Mathf.Clamp(thighRot.w, -1f, 1f)) <= 0.36f);
            }
        }

        private static void TestConfigurableJoint3DSimulationOverBridge()
        {
            using (BridgeHost host = new BridgeHost())
            {
                GameObject frame = CreateBody3D(host, "Frame3D", Vector3.zero, Quaternion.identity, true);
                GameObject carriage = CreateBody3D(host, "Carriage3D", new Vector3(0f, 3f, 4f), Quaternion.Euler(0f, 0f, 50f), false);
                ConfigurableJoint cfg = carriage.AddComponent<ConfigurableJoint>();
                cfg.connectedBody = frame.GetComponent<Rigidbody>();
                cfg.xMotion = ConfigurableJointMotion.Free;
                cfg.yMotion = ConfigurableJointMotion.Locked;
                cfg.zMotion = ConfigurableJointMotion.Limited;
                cfg.angularXMotion = ConfigurableJointMotion.Locked;
                cfg.angularYMotion = ConfigurableJointMotion.Locked;
                cfg.angularZMotion = ConfigurableJointMotion.Limited;
                cfg.linearLimit = new SoftJointLimit { limit = 1.25f };
                cfg.lowAngularXLimit = new SoftJointLimit { limit = -0.1f };
                cfg.highAngularXLimit = new SoftJointLimit { limit = 0.1f };
                cfg.angularYLimit = new SoftJointLimit { limit = 0.1f };
                cfg.angularZLimit = new SoftJointLimit { limit = 0.3f };
                cfg.xDrive = new JointDrive { positionSpring = 50f, positionDamper = 10f, maximumForce = 500f };
                cfg.targetPosition = new Vector3(2f, 0f, 0f);
                cfg.SyncToNative();

                Step3D(host, 60, 0.02);

                Check.That("ConfigurableJoint drives Free X to 2.0, locks Y to 0.0, and clamps Limited Z to 1.25",
                    host.TryGetBodyPosition(carriage.NativeEntityId, out Vector3 cPos)
                    && Mathf.Abs(cPos.x - 2.0f) < 0.1f
                    && Mathf.Abs(cPos.y) < 1e-3f
                    && Mathf.Abs(cPos.z - 1.25f) < 1e-2f);
            }
        }

        private static void TestSceneYamlHingeJointMaterializationAndSimulation()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_joint_scene_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "HingeJointScene.unity");
            File.WriteAllText(scenePath, AuthoredHingeJointSceneYaml);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.EnsureLoadedSceneJointsMaterialized();
                    GameObject? frameGo = GameObject.Find("SceneHingeBase");
                    GameObject? doorGo = GameObject.Find("SceneHingeDoor");
                    Check.That("SceneHingeBase materialized", frameGo != null);
                    Check.That("SceneHingeDoor materialized", doorGo != null);
                    if (frameGo == null || doorGo == null)
                    {
                        return;
                    }
                    HingeJoint? hj = doorGo.GetComponent<HingeJoint>();
                    Check.That("Authored HingeJoint component materialized on SceneHingeDoor", hj != null);
                    if (hj == null)
                    {
                        return;
                    }
                    Check.That("Authored HingeJoint connectedBody resolves to SceneHingeBase Rigidbody",
                        hj.connectedBody != null && ReferenceEquals(hj.connectedBody.gameObject, frameGo));
                    Check.Equal("Authored HingeJoint useMotor", true, hj.useMotor);
                    Check.Equal("Authored HingeJoint useLimits", true, hj.useLimits);

                    Step3D(host, 30, 0.02);

                    Check.That("Authored HingeJoint simulates motor and limit in native 3D world",
                        host.TryGetBodyPosition(doorGo.NativeEntityId, out Vector3 doorPos)
                        && Mathf.Abs(doorPos.x - Mathf.Cos(0.5f)) < 2e-2f
                        && Mathf.Abs(doorPos.y - Mathf.Sin(0.5f)) < 2e-2f);
                }
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static GameObject CreateBody3D(BridgeHost host, string name, Vector3 pos, Quaternion rot, bool isKinematic)
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_j3d_" + Guid.NewGuid().ToString("N"));
            if (host.SceneCount == 0)
            {
                Directory.CreateDirectory(tempDir);
                string scenePath = Path.Combine(tempDir, "Empty3D.unity");
                File.WriteAllText(scenePath, EmptyZeroGravity3DSceneYaml);
                host.LoadScene(scenePath);
                Directory.Delete(tempDir, true);
            }
            GameObject go = new GameObject(name);
            go.transform.position = pos;
            go.transform.rotation = rot;
            Rigidbody rb = go.AddComponent<Rigidbody>();
            rb.useGravity = false;
            rb.isKinematic = isKinematic;
            return go;
        }

        private static void Step3D(BridgeHost host, int steps, double dt)
        {
            for (int i = 0; i < steps; i++)
            {
                host.Tick(dt);
                host.SyncJointTransformsAfterStep();
            }
        }

        private const string EmptyZeroGravity3DSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: Root
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
";

        private const string AuthoredHingeJointSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Name: SceneHingeBase
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Father: {fileID: 0}
--- !u!54 &12
Rigidbody:
  m_GameObject: {fileID: 10}
  m_UseGravity: 0
  m_IsKinematic: 1
--- !u!135 &13
SphereCollider:
  m_GameObject: {fileID: 10}
  m_Radius: 0.25
  m_IsTrigger: 1
--- !u!1 &20
GameObject:
  m_Name: SceneHingeDoor
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalPosition: {x: 1, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Father: {fileID: 0}
--- !u!54 &22
Rigidbody:
  m_GameObject: {fileID: 20}
  m_Mass: 1
  m_UseGravity: 0
  m_IsKinematic: 0
--- !u!135 &23
SphereCollider:
  m_GameObject: {fileID: 20}
  m_Radius: 0.25
  m_IsTrigger: 1
--- !u!59 &24
HingeJoint:
  m_GameObject: {fileID: 20}
  m_ConnectedBody: {fileID: 12}
  m_Anchor: {x: -1, y: 0, z: 0}
  m_Axis: {x: 0, y: 0, z: 1}
  m_ConnectedAnchor: {x: 0, y: 0, z: 0}
  m_UseMotor: 1
  m_Motor:
    targetVelocity: 6
    force: 100
    freeSpin: 0
  m_UseLimits: 1
  m_Limits:
    min: -0.4
    max: 0.5
";
    }

    internal static class JointTestEntry
    {
        [ModuleInitializer]
        internal static void Register()
        {
            if (Environment.GetEnvironmentVariable("UNITAS_BRIDGE_SHARD") == "physics")
            {
                JointTests.Run();
            }
        }
    }
}
