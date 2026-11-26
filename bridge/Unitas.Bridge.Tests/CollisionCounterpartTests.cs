using System;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class CollisionCounterpartTests
    {
        private const string BuiltInCubeMeshIdentity = "0000000000000000e000000000000000:10202";

        public static void Run()
        {
            TestCollisionCounterpartResolvesScriptlessGameObject();
            TestTriggerCounterpartResolvesScriptlessCollider();
            MeshColliderIsMaterializedFromTheScene();
            MeshColliderDoesNotSurviveTheNextScene();
        }

        private static void TestCollisionCounterpartResolvesScriptlessGameObject()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");

            Check.That("Collision counterpart scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            CollisionRecorderScript recordedScript = new CollisionRecorderScript();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            Check.That(
                "Scriptless collision counterpart delivers a gameObject to the script",
                !recordedScript.OtherGameObjectAlwaysNull);
            Check.Equal(
                "Scriptless collision counterpart gameObject is the scene crate",
                "Crate",
                recordedScript.ObservedOtherNames);
        }

        private static void TestTriggerCounterpartResolvesScriptlessCollider()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_trigger.unity");

            Check.That("Trigger counterpart scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            TriggerRecorderScript recordedScript = new TriggerRecorderScript();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            Check.That(
                "Scriptless trigger counterpart delivers a collider to the script",
                !recordedScript.OtherColliderAlwaysNull);
            Check.Equal(
                "Scriptless trigger counterpart collider belongs to the scene crate",
                "Crate",
                recordedScript.ObservedOtherNames);
        }

        public static void MeshColliderIsMaterializedFromTheScene()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_mesh_collider_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "mesh_collider.unity");
            File.WriteAllText(scenePath, MeshColliderScene);

            try
            {
                ScriptGlueRegistry.Clear();
                MeshColliderProbe? convexProbe = null;
                MeshColliderProbe? sharedProbe = null;
                MeshColliderProbe? emptyProbe = null;
                ScriptGlueRegistry.Register(
                    0x1111222233334444UL,
                    0x5555666677778888UL,
                    () => convexProbe = new MeshColliderProbe());
                ScriptGlueRegistry.Register(
                    0x2222222233334444UL,
                    0x5555666677778888UL,
                    () => sharedProbe = new MeshColliderProbe());
                ScriptGlueRegistry.Register(
                    0x3333333333334444UL,
                    0x5555666677778888UL,
                    () => emptyProbe = new MeshColliderProbe());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    for (int tick = 0; tick < 4; tick++)
                    {
                        host.Tick(0.02);
                    }
                    host.Shutdown();
                }

                MeshCollider? convex = convexProbe?.Collider;
                MeshCollider? shared = sharedProbe?.Collider;
                MeshCollider? empty = emptyProbe?.Collider;

                Check.That("MeshColliderIsMaterializedFromTheScene: scene mesh collider is materialised", convex != null);
                Check.Equal("MeshColliderIsMaterializedFromTheScene: scene mesh collider keeps the scene convex flag", true, convex != null && convex.convex);
                Check.That("MeshColliderIsMaterializedFromTheScene: scene mesh collider exposes the mesh it names", convex != null && convex.sharedMesh != null);
                Check.Equal(
                    "MeshColliderIsMaterializedFromTheScene: scene mesh collider names the mesh the scene names",
                    BuiltInCubeMeshIdentity,
                    convex != null && convex.sharedMesh != null ? convex.sharedMesh.name : string.Empty);

                Check.That("MeshColliderIsMaterializedFromTheScene: second scene mesh collider is materialised", shared != null);
                Check.Equal("MeshColliderIsMaterializedFromTheScene: second scene mesh collider keeps the scene convex flag", false, shared != null && shared.convex);
                Check.That(
                    "MeshColliderIsMaterializedFromTheScene: two colliders naming the same mesh share one mesh reference",
                    convex != null && shared != null && ReferenceEquals(convex.sharedMesh, shared.sharedMesh));

                Check.That("MeshColliderIsMaterializedFromTheScene: scene mesh collider without a mesh is materialised", empty != null);
                Check.That("MeshColliderIsMaterializedFromTheScene: scene mesh collider without a mesh has no shared mesh", empty != null && empty.sharedMesh == null);
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        public static void MeshColliderDoesNotSurviveTheNextScene()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_mesh_collider_reload_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string firstScenePath = Path.Combine(tempDir, "first.unity");
            string secondScenePath = Path.Combine(tempDir, "second.unity");
            File.WriteAllText(firstScenePath, MeshColliderFirstScene);
            File.WriteAllText(secondScenePath, MeshColliderSecondScene);

            try
            {
                ScriptGlueRegistry.Clear();
                MeshColliderProbe? firstProbe = null;
                MeshColliderProbe? secondProbe = null;
                ScriptGlueRegistry.Register(
                    0x4444444433334444UL,
                    0x5555666677778888UL,
                    () => firstProbe = new MeshColliderProbe());
                ScriptGlueRegistry.Register(
                    0x6666666633334444UL,
                    0x5555666677778888UL,
                    () => secondProbe = new MeshColliderProbe());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(firstScenePath);
                    for (int tick = 0; tick < 4; tick++)
                    {
                        host.Tick(0.02);
                    }
                    host.LoadScene(secondScenePath);
                    for (int tick = 0; tick < 4; tick++)
                    {
                        host.Tick(0.02);
                    }
                    host.Shutdown();
                }

                Check.That("MeshColliderDoesNotSurviveTheNextScene: first scene mesh collider is materialised", firstProbe?.Collider != null);
                Check.That("MeshColliderDoesNotSurviveTheNextScene: second scene script runs", secondProbe != null);
                Check.That(
                    "MeshColliderDoesNotSurviveTheNextScene: second scene does not inherit the first scene mesh collider",
                    secondProbe != null && secondProbe.Collider == null);
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private sealed class MeshColliderProbe : MonoBehaviour
        {
            public MeshCollider? Collider { get; private set; }

            public override void Awake()
            {
                Collider = GetComponent<MeshCollider>();
            }
        }

        private const string MeshColliderScene = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: ConvexMesh
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!64 &1002
MeshCollider:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}
  m_CookedOptDataCollisionMesh: {fileID: 0}
  m_Convex: 1
  m_IsTrigger: 0
  m_Center: {x: 0, y: 0, z: 0}
  m_Material: {fileID: 0}
--- !u!114 &1003
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 11112222333344445555666677778888, type: 3}
--- !u!1 &2000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: SharedMesh
--- !u!4 &2001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 2, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!64 &2002
MeshCollider:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}
  m_CookedOptDataCollisionMesh: {fileID: 0}
  m_Convex: 0
  m_IsTrigger: 0
  m_Center: {x: 0, y: 0, z: 0}
  m_Material: {fileID: 0}
--- !u!114 &2003
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 22222222333344445555666677778888, type: 3}
--- !u!1 &3000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: EmptyMesh
--- !u!4 &3001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 4, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!64 &3002
MeshCollider:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_Mesh: {fileID: 0}
  m_CookedOptDataCollisionMesh: {fileID: 0}
  m_Convex: 0
  m_IsTrigger: 0
  m_Center: {x: 0, y: 0, z: 0}
  m_Material: {fileID: 0}
--- !u!114 &3003
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 33333333333344445555666677778888, type: 3}
";

        private const string MeshColliderFirstScene = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: FirstSceneMesh
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!64 &1002
MeshCollider:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}
  m_CookedOptDataCollisionMesh: {fileID: 0}
  m_Convex: 1
  m_IsTrigger: 0
  m_Center: {x: 0, y: 0, z: 0}
  m_Material: {fileID: 0}
--- !u!114 &1003
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44444444333344445555666677778888, type: 3}
";

        private const string MeshColliderSecondScene = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: SecondScenePlain
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!114 &1002
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 66666666333344445555666677778888, type: 3}
";

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
