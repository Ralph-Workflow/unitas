using System;
using System.IO;
using System.Runtime.CompilerServices;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class Effector2DTests
    {
        private static bool s_ran;

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestEffector2DComponentDefaultsAndPropertyRoundTrips();
            TestAreaEffector2DAppliesDirectionalForceAndDragOverAbi();
            TestPointEffector2DAttractsAndRepelsWithColliderMaskOverAbi();
            TestBuoyancyEffector2DFloatsSinksAndAppliesFlowAndDragOverAbi();
        }

        private static void TestEffector2DComponentDefaultsAndPropertyRoundTrips()
        {
            using (GameObject.SuppressRuntimeCreation())
            {
                GameObject go = new GameObject("Effector2DDefaults");

                AreaEffector2D area = go.AddComponent<AreaEffector2D>();
                Check.That("AreaEffector2D is a Component", area is Component);
                Check.Equal("AreaEffector2D default forceAngle", 0f, area.forceAngle);
                Check.Equal("AreaEffector2D default forceMagnitude", 10f, area.forceMagnitude);
                Check.Equal("AreaEffector2D default forceVariation", 0f, area.forceVariation);
                Check.Equal("AreaEffector2D default drag", 0f, area.drag);
                Check.Equal("AreaEffector2D default angularDrag", 0f, area.angularDrag);
                Check.Equal("AreaEffector2D default useGlobalAngle", false, area.useGlobalAngle);
                Check.Equal("AreaEffector2D default useColliderMask", true, area.useColliderMask);
                Check.Equal("AreaEffector2D default colliderMask", -1, area.colliderMask);

                area.forceAngle = 45f;
                area.forceMagnitude = 25f;
                area.forceVariation = 2.5f;
                area.drag = 0.75f;
                area.angularDrag = 1.25f;
                area.useGlobalAngle = true;
                area.useColliderMask = false;
                area.colliderMask = 1 << 2;
                Check.Equal("AreaEffector2D forceAngle round-trip", 45f, area.forceAngle);
                Check.Equal("AreaEffector2D forceMagnitude round-trip", 25f, area.forceMagnitude);
                Check.Equal("AreaEffector2D forceVariation round-trip", 2.5f, area.forceVariation);
                Check.Equal("AreaEffector2D drag round-trip", 0.75f, area.drag);
                Check.Equal("AreaEffector2D angularDrag round-trip", 1.25f, area.angularDrag);
                Check.Equal("AreaEffector2D useGlobalAngle round-trip", true, area.useGlobalAngle);
                Check.Equal("AreaEffector2D useColliderMask round-trip", false, area.useColliderMask);
                Check.Equal("AreaEffector2D colliderMask round-trip", 1 << 2, area.colliderMask);

                PointEffector2D point = go.AddComponent<PointEffector2D>();
                Check.That("PointEffector2D is a Component", point is Component);
                Check.Equal("PointEffector2D default forceMagnitude", -10f, point.forceMagnitude);
                Check.Equal("PointEffector2D default forceVariation", 0f, point.forceVariation);
                Check.Equal("PointEffector2D default distanceScale", 1f, point.distanceScale);
                Check.Equal("PointEffector2D default drag", 0f, point.drag);
                Check.Equal("PointEffector2D default angularDrag", 0f, point.angularDrag);
                Check.Equal("PointEffector2D default useColliderMask", true, point.useColliderMask);
                Check.Equal("PointEffector2D default colliderMask", -1, point.colliderMask);

                point.forceMagnitude = 30f;
                point.forceVariation = 1.5f;
                point.distanceScale = 2f;
                point.drag = 0.4f;
                point.angularDrag = 0.8f;
                point.useColliderMask = false;
                point.colliderMask = 1 << 3;
                Check.Equal("PointEffector2D forceMagnitude round-trip", 30f, point.forceMagnitude);
                Check.Equal("PointEffector2D forceVariation round-trip", 1.5f, point.forceVariation);
                Check.Equal("PointEffector2D distanceScale round-trip", 2f, point.distanceScale);
                Check.Equal("PointEffector2D drag round-trip", 0.4f, point.drag);
                Check.Equal("PointEffector2D angularDrag round-trip", 0.8f, point.angularDrag);
                Check.Equal("PointEffector2D useColliderMask round-trip", false, point.useColliderMask);
                Check.Equal("PointEffector2D colliderMask round-trip", 1 << 3, point.colliderMask);

                BuoyancyEffector2D buoyancy = go.AddComponent<BuoyancyEffector2D>();
                Check.That("BuoyancyEffector2D is a Component", buoyancy is Component);
                Check.Equal("BuoyancyEffector2D default surfaceLevel", 0f, buoyancy.surfaceLevel);
                Check.Equal("BuoyancyEffector2D default density", 2f, buoyancy.density);
                Check.Equal("BuoyancyEffector2D default linearDrag", 5f, buoyancy.linearDrag);
                Check.Equal("BuoyancyEffector2D default angularDrag", 5f, buoyancy.angularDrag);
                Check.Equal("BuoyancyEffector2D default flowAngle", 0f, buoyancy.flowAngle);
                Check.Equal("BuoyancyEffector2D default flowMagnitude", 0f, buoyancy.flowMagnitude);
                Check.Equal("BuoyancyEffector2D default flowVariation", 0f, buoyancy.flowVariation);
                Check.Equal("BuoyancyEffector2D default useColliderMask", true, buoyancy.useColliderMask);
                Check.Equal("BuoyancyEffector2D default colliderMask", -1, buoyancy.colliderMask);

                buoyancy.surfaceLevel = 1.5f;
                buoyancy.density = 3.2f;
                buoyancy.linearDrag = 2.5f;
                buoyancy.angularDrag = 4f;
                buoyancy.flowAngle = 15f;
                buoyancy.flowMagnitude = 6f;
                buoyancy.flowVariation = 0.5f;
                buoyancy.useColliderMask = false;
                buoyancy.colliderMask = 1 << 4;
                Check.Equal("BuoyancyEffector2D surfaceLevel round-trip", 1.5f, buoyancy.surfaceLevel);
                Check.Equal("BuoyancyEffector2D density round-trip", 3.2f, buoyancy.density);
                Check.Equal("BuoyancyEffector2D linearDrag round-trip", 2.5f, buoyancy.linearDrag);
                Check.Equal("BuoyancyEffector2D angularDrag round-trip", 4f, buoyancy.angularDrag);
                Check.Equal("BuoyancyEffector2D flowAngle round-trip", 15f, buoyancy.flowAngle);
                Check.Equal("BuoyancyEffector2D flowMagnitude round-trip", 6f, buoyancy.flowMagnitude);
                Check.Equal("BuoyancyEffector2D flowVariation round-trip", 0.5f, buoyancy.flowVariation);
                Check.Equal("BuoyancyEffector2D useColliderMask round-trip", false, buoyancy.useColliderMask);
                Check.Equal("BuoyancyEffector2D colliderMask round-trip", 1 << 4, buoyancy.colliderMask);
            }
        }

        private static void TestAreaEffector2DAppliesDirectionalForceAndDragOverAbi()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_eff2d_area_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "AreaEffectorScene.unity");
            File.WriteAllText(scenePath, AreaEffectorSceneYaml);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    Check.That("BridgeHost implements IPhysicsEffector2DProvider", Physics2D.Provider is IPhysicsEffector2DProvider);
                    host.LoadScene(scenePath);
                    Physics2D.gravity = Vector2.zero;

                    GameObject? windGo = GameObject.Find("WindZone2D");
                    GameObject? ballGo = GameObject.Find("WindBall2D");
                    Check.That("WindZone2D materialized", windGo != null);
                    Check.That("WindBall2D materialized", ballGo != null);
                    if (windGo == null || ballGo == null)
                    {
                        return;
                    }

                    AreaEffector2D area = windGo.AddComponent<AreaEffector2D>();
                    area.forceAngle = 90f;
                    area.forceMagnitude = 20f;
                    area.drag = 0.5f;
                    area.colliderMask = 1 << 1;

                    Rigidbody2D? ballBody = ballGo.GetComponent<Rigidbody2D>();
                    Check.That("WindBall2D has Rigidbody2D", ballBody != null);
                    if (ballBody == null)
                    {
                        return;
                    }

                    ballBody.velocity = new Vector2(4f, 0f);
                    for (int step = 0; step < 25; step++)
                    {
                        host.Tick(0.02);
                    }

                    Vector2 finalVelocity = ballBody.velocity;
                    Check.That("AreaEffector2D at 90 deg pushes ball upward (vy > 5)", finalVelocity.y > 5f);
                    Check.That("AreaEffector2D drag damps horizontal velocity (vx < 4)", finalVelocity.x < 4f);
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

        private static void TestPointEffector2DAttractsAndRepelsWithColliderMaskOverAbi()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_eff2d_point_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "PointEffectorScene.unity");
            File.WriteAllText(scenePath, PointEffectorSceneYaml);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    Physics2D.gravity = Vector2.zero;

                    GameObject? magnetGo = GameObject.Find("Magnet2D");
                    GameObject? metalGo = GameObject.Find("Metal2D");
                    GameObject? woodGo = GameObject.Find("Wood2D");
                    Check.That("Magnet2D materialized", magnetGo != null);
                    Check.That("Metal2D materialized", metalGo != null);
                    Check.That("Wood2D materialized", woodGo != null);
                    if (magnetGo == null || metalGo == null || woodGo == null)
                    {
                        return;
                    }

                    PointEffector2D point = magnetGo.AddComponent<PointEffector2D>();
                    point.forceMagnitude = -30f;
                    point.distanceScale = 1f;
                    point.colliderMask = 1 << 1;

                    Rigidbody2D? metalBody = metalGo.GetComponent<Rigidbody2D>();
                    Rigidbody2D? woodBody = woodGo.GetComponent<Rigidbody2D>();
                    Check.That("Metal2D has Rigidbody2D", metalBody != null);
                    Check.That("Wood2D has Rigidbody2D", woodBody != null);
                    if (metalBody == null || woodBody == null)
                    {
                        return;
                    }

                    for (int step = 0; step < 25; step++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That("Negative PointEffector2D attracts metal toward origin (vx < -1)", metalBody.velocity.x < -1f);
                    Check.That("Excluded layer Wood2D is unaffected by PointEffector2D", Math.Abs(woodBody.velocity.x) < 1e-4f);

                    point.forceMagnitude = 50f;

                    metalBody.MovePosition(new Vector2(2f, 0f));
                    metalBody.velocity = Vector2.zero;
                    for (int step = 0; step < 25; step++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That("Positive PointEffector2D repels metal away from origin (vx > 1)", metalBody.velocity.x > 1f);
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

        private static void TestBuoyancyEffector2DFloatsSinksAndAppliesFlowAndDragOverAbi()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_eff2d_buoyancy_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "BuoyancyEffectorScene.unity");
            File.WriteAllText(scenePath, BuoyancyEffectorSceneYaml);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    Physics2D.gravity = new Vector2(0f, -10f);

                    GameObject? waterGo = GameObject.Find("Water2D");
                    GameObject? corkGo = GameObject.Find("Cork2D");
                    GameObject? stoneGo = GameObject.Find("Stone2D");
                    Check.That("Water2D materialized", waterGo != null);
                    Check.That("Cork2D materialized", corkGo != null);
                    Check.That("Stone2D materialized", stoneGo != null);
                    if (waterGo == null || corkGo == null || stoneGo == null)
                    {
                        return;
                    }

                    BuoyancyEffector2D buoyancy = waterGo.AddComponent<BuoyancyEffector2D>();
                    buoyancy.surfaceLevel = 2f;
                    buoyancy.density = 2f;
                    buoyancy.linearDrag = 2f;
                    buoyancy.angularDrag = 3f;
                    buoyancy.flowAngle = 0f;
                    buoyancy.flowMagnitude = 8f;
                    buoyancy.flowVariation = 0f;
                    buoyancy.colliderMask = 1 << 0;

                    Rigidbody2D? corkBody = corkGo.GetComponent<Rigidbody2D>();
                    Rigidbody2D? stoneBody = stoneGo.GetComponent<Rigidbody2D>();
                    Check.That("Cork2D has Rigidbody2D", corkBody != null);
                    Check.That("Stone2D has Rigidbody2D", stoneBody != null);
                    if (corkBody == null || stoneBody == null)
                    {
                        return;
                    }

                    corkBody.velocity = new Vector2(0f, -2f);
                    for (int step = 0; step < 20; step++)
                    {
                        host.Tick(0.02);
                    }

                    Vector2 corkVel = corkBody.velocity;
                    Vector2 stoneVel = stoneBody.velocity;
                    Check.That("Submerged low-mass cork floats upward (vy > 0)", corkVel.y > 0f);
                    Check.That("Submerged cork receives rightward flow velocity (vx > 0.5)", corkVel.x > 0.5f);
                    Check.That("Heavy stone sinks downward (vy < -1)", stoneVel.y < -1f);
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

        private const string AreaEffectorSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: WindZone2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!61 &102
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 8, y: 8}
--- !u!1 &200
GameObject:
  m_Name: WindBall2D
  m_Layer: 1
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!50 &202
Rigidbody2D:
  m_GameObject: {fileID: 200}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
--- !u!58 &203
CircleCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Radius: 0.4
";

        private const string PointEffectorSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: Magnet2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!58 &102
CircleCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Radius: 5
--- !u!1 &200
GameObject:
  m_Name: Metal2D
  m_Layer: 1
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalPosition: {x: 3, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!50 &202
Rigidbody2D:
  m_GameObject: {fileID: 200}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
--- !u!58 &203
CircleCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Radius: 0.3
--- !u!1 &300
GameObject:
  m_Name: Wood2D
  m_Layer: 2
  m_IsActive: 1
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalPosition: {x: 3, y: 1, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!50 &302
Rigidbody2D:
  m_GameObject: {fileID: 300}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
--- !u!58 &303
CircleCollider2D:
  m_GameObject: {fileID: 300}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Radius: 0.3
";

        private const string BuoyancyEffectorSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: Water2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!61 &102
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 20, y: 8}
--- !u!1 &200
GameObject:
  m_Name: Cork2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalPosition: {x: -2, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!50 &202
Rigidbody2D:
  m_GameObject: {fileID: 200}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 1
--- !u!61 &203
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 1, y: 1}
--- !u!1 &300
GameObject:
  m_Name: Stone2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalPosition: {x: 2, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!50 &302
Rigidbody2D:
  m_GameObject: {fileID: 300}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 8
  m_GravityScale: 1
--- !u!61 &303
BoxCollider2D:
  m_GameObject: {fileID: 300}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 1, y: 1}
";
    }

    internal static class Effector2DTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnTargetShard()
        {
            string[] args = Environment.GetCommandLineArgs();
            if (IsPerfMode(args) || !IsTargetShard(args))
            {
                return;
            }
            Effector2DTests.Run();
        }

        private static bool IsPerfMode(string[] args)
        {
            for (int i = 0; i < args.Length; i++)
            {
                if (string.Equals(args[i], "--perf", StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static bool IsTargetShard(string[] args)
        {
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (!string.Equals(args[i], "--shard", StringComparison.Ordinal))
                {
                    continue;
                }
                string[] parts = args[i + 1].Split('/');
                return parts.Length == 2 && string.Equals(parts[0], "4", StringComparison.Ordinal);
            }
            return true;
        }
    }
}
