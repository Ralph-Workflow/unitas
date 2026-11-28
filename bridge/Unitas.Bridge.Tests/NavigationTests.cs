using System;
using System.IO;
using UnityEngine;
using UnityEngine.AI;

namespace Unitas.Bridge.Tests
{
    public static class NavigationTests
    {
        private const string NavigationSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: FloorWest
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -3, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &102
BoxCollider:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: -0.5, z: 0}
--- !u!114 &103
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 7a5ac11cc976e418e8d13136b07e1f52, type: 3}
  m_AgentTypeID: 0
  m_CollectObjects: 0
  m_DefaultArea: 0
--- !u!1 &110
GameObject:
  m_Name: FloorMid
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &111
Transform:
  m_GameObject: {fileID: 110}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -1, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &112
BoxCollider:
  m_GameObject: {fileID: 110}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: -0.5, z: 0}
--- !u!1 &120
GameObject:
  m_Name: FloorEast
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &121
Transform:
  m_GameObject: {fileID: 120}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 1, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &122
BoxCollider:
  m_GameObject: {fileID: 120}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: -0.5, z: 0}
--- !u!1 &130
GameObject:
  m_Name: FloorNorthMid
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &131
Transform:
  m_GameObject: {fileID: 130}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -1, y: 0, z: 2}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &132
BoxCollider:
  m_GameObject: {fileID: 130}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: -0.5, z: 0}
--- !u!1 &140
GameObject:
  m_Name: FloorNorthWest
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &141
Transform:
  m_GameObject: {fileID: 140}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -3, y: 0, z: 2}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &142
BoxCollider:
  m_GameObject: {fileID: 140}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: -0.5, z: 0}
--- !u!1 &150
GameObject:
  m_Name: FloorNorthEast
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &151
Transform:
  m_GameObject: {fileID: 150}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 1, y: 0, z: 2}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &152
BoxCollider:
  m_GameObject: {fileID: 150}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: -0.5, z: 0}
--- !u!1 &200
GameObject:
  m_Name: NavRunner
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -3.2, y: 0, z: -0.2}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!195 &202
NavMeshAgent:
  m_GameObject: {fileID: 200}
  m_Enabled: 1
  m_Radius: 0.4
  m_Speed: 4.0
  m_Acceleration: 10.0
  m_AngularSpeed: 180.0
  m_StoppingDistance: 0.05
  m_Height: 1.8
  avoidancePriority: 40
--- !u!1 &300
GameObject:
  m_Name: NavBlocker
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -1.0, y: 0, z: 0.0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!208 &302
NavMeshObstacle:
  m_GameObject: {fileID: 300}
  m_Enabled: 1
  m_Shape: 1
  m_Center: {x: 0, y: 0, z: 0}
  m_Size: {x: 1.8, y: 2.0, z: 1.8}
  m_Carve: 0
  m_CarveOnlyStationary: 0
";

        public static void Run()
        {
            TestNullProviderDefaults();
            TestEndToEndSceneNavigationAndQueries();
            TestNavMeshObstacleCarvingDetour();
        }

        private static void TestNullProviderDefaults()
        {
            INavigationProvider? saved = NavMesh.Provider;
            NavMesh.Provider = null;
            try
            {
                Check.Equal("NavMesh.AllAreas value", -1, NavMesh.AllAreas);
                Check.Equal("NavMeshPathStatus.PathComplete value", 0, (int)NavMeshPathStatus.PathComplete);
                Check.Equal("NavMeshPathStatus.PathPartial value", 1, (int)NavMeshPathStatus.PathPartial);
                Check.Equal("NavMeshPathStatus.PathInvalid value", 2, (int)NavMeshPathStatus.PathInvalid);

                GameObject go = new GameObject("OfflineNavAgent");
                NavMeshAgent agent = go.AddComponent<NavMeshAgent>();
                Check.Equal("Offline default speed", 3.5f, agent.speed);
                Check.Equal("Offline default acceleration", 8f, agent.acceleration);
                Check.Equal("Offline default angularSpeed", 120f, agent.angularSpeed);
                Check.Equal("Offline default stoppingDistance", 0f, agent.stoppingDistance);
                Check.Equal("Offline default radius", 0.5f, agent.radius);
                Check.Equal("Offline default height", 2f, agent.height);
                Check.Equal("Offline default isStopped", false, agent.isStopped);
                Check.Equal("Offline default isOnNavMesh", false, agent.isOnNavMesh);
                Check.Equal("Offline default hasPath", false, agent.hasPath);
                Check.Equal("Offline default pathPending", false, agent.pathPending);
                Check.Equal("Offline default remainingDistance", 0f, agent.remainingDistance);

                agent.speed = 6f;
                agent.stoppingDistance = 0.25f;
                agent.radius = 0.35f;
                agent.height = 1.6f;
                agent.isStopped = true;
                Check.Equal("Offline speed round trip", 6f, agent.speed);
                Check.Equal("Offline stoppingDistance round trip", 0.25f, agent.stoppingDistance);
                Check.Equal("Offline radius round trip", 0.35f, agent.radius);
                Check.Equal("Offline height round trip", 1.6f, agent.height);
                Check.Equal("Offline isStopped round trip", true, agent.isStopped);

                bool destOk = agent.SetDestination(new Vector3(5f, 0f, 5f));
                Check.Equal("Offline SetDestination returns false", false, destOk);
                Check.That("Offline pathStatus after failed SetDestination", agent.pathStatus == NavMeshPathStatus.PathInvalid);

                bool sampled = NavMesh.SamplePosition(Vector3.zero, out NavMeshHit sampleHit, 2f, NavMesh.AllAreas);
                Check.Equal("Offline SamplePosition returns false", false, sampled);
                Check.Equal("Offline SamplePosition hit is false", false, sampleHit.hit);

                bool rayBlocked = NavMesh.Raycast(Vector3.zero, Vector3.forward, out NavMeshHit rayHit, NavMesh.AllAreas);
                Check.Equal("Offline Raycast returns false", false, rayBlocked);
                Check.Equal("Offline Raycast hit is false", false, rayHit.hit);

                NavMeshPath path = new NavMeshPath();
                bool pathOk = NavMesh.CalculatePath(Vector3.zero, Vector3.forward, NavMesh.AllAreas, path);
                Check.Equal("Offline CalculatePath returns false", false, pathOk);
                Check.That("Offline CalculatePath status is PathInvalid", path.status == NavMeshPathStatus.PathInvalid);
                Check.Equal("Offline CalculatePath corners empty", 0, path.corners.Length);

                NavMeshObstacle obstacle = go.AddComponent<NavMeshObstacle>();
                Check.Equal("Offline obstacle default carving", false, obstacle.carving);
                Check.Equal("Offline obstacle default carveOnlyStationary", true, obstacle.carveOnlyStationary);
                Check.That("Offline obstacle default shape", obstacle.shape == NavMeshObstacleShape.Box);
                obstacle.carving = true;
                obstacle.shape = NavMeshObstacleShape.Capsule;
                obstacle.radius = 0.75f;
                obstacle.height = 2.5f;
                Check.Equal("Offline obstacle carving round trip", true, obstacle.carving);
                Check.That("Offline obstacle shape round trip", obstacle.shape == NavMeshObstacleShape.Capsule);
                Check.Equal("Offline obstacle radius round trip", 0.75f, obstacle.radius);
                Check.Equal("Offline obstacle height round trip", 2.5f, obstacle.height);

                UnityEngine.Object.Destroy(go);
            }
            finally
            {
                NavMesh.Provider = saved;
            }
        }

        private static void TestEndToEndSceneNavigationAndQueries()
        {
            PurgeStaleNavigationObjects();
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_nav_test_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "NavTest.unity");
            File.WriteAllText(scenePath, NavigationSceneYaml);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    if (NavMesh.Provider == null)
                    {
                        NavMesh.Provider = new RuntimeNavigationProvider(host.Handle);
                    }
                    host.LoadScene(scenePath);

                    GameObject? runnerObj = GameObject.Find("NavRunner");
                    Check.That("NavRunner exists in loaded scene", runnerObj != null);
                    if (runnerObj == null)
                    {
                        return;
                    }

                    NavMeshAgent agent = runnerObj.GetComponent<NavMeshAgent>() ?? runnerObj.AddComponent<NavMeshAgent>();
                    Check.Equal("Scene NavMeshAgent isOnNavMesh", true, agent.isOnNavMesh);
                    Check.Equal("Scene NavMeshAgent speed from YAML", 4.0f, agent.speed);
                    Check.Equal("Scene NavMeshAgent radius from YAML", 0.4f, agent.radius);
                    Check.Equal("Scene NavMeshAgent height from YAML", 1.8f, agent.height);

                    bool sampled = NavMesh.SamplePosition(new Vector3(-1.2f, 0.2f, 0.1f), out NavMeshHit sampleHit, 1.0f, NavMesh.AllAreas);
                    Check.Equal("NavMesh.SamplePosition finds floor polygon", true, sampled);
                    Check.Equal("NavMesh.SamplePosition hit flag", true, sampleHit.hit);
                    Check.That("NavMesh.SamplePosition projects onto y=0 floor", Math.Abs(sampleHit.position.y) < 0.01f);

                    bool clearRay = NavMesh.Raycast(new Vector3(-3.2f, 0f, -0.2f), new Vector3(1.2f, 0f, -0.2f), out NavMeshHit clearHit, NavMesh.AllAreas);
                    Check.Equal("NavMesh.Raycast unblocked across floor returns false", false, clearRay);
                    Check.Equal("NavMesh.Raycast unblocked hit.hit is false", false, clearHit.hit);

                    bool edgeRay = NavMesh.Raycast(new Vector3(-1.2f, 0f, -0.2f), new Vector3(-1.2f, 0f, -4.0f), out NavMeshHit edgeHit, NavMesh.AllAreas);
                    Check.Equal("NavMesh.Raycast across navmesh edge returns true", true, edgeRay);
                    Check.Equal("NavMesh.Raycast blocked hit.hit is true", true, edgeHit.hit);
                    Check.That("NavMesh.Raycast boundary hit z is near -1", Math.Abs(edgeHit.position.z - (-1.0f)) < 0.05f);

                    NavMeshPath path = new NavMeshPath();
                    bool calcOk = NavMesh.CalculatePath(new Vector3(-3.2f, 0f, -0.2f), new Vector3(1.2f, 0f, -0.2f), NavMesh.AllAreas, path);
                    Check.Equal("NavMesh.CalculatePath succeeds", true, calcOk);
                    Check.That("NavMesh.CalculatePath status is PathComplete", path.status == NavMeshPathStatus.PathComplete);
                    Check.That("NavMesh.CalculatePath produces >= 2 corners", path.corners.Length >= 2);
                    Vector3[] buffer = new Vector3[8];
                    int nonAllocCount = path.GetCornersNonAlloc(buffer);
                    Check.Equal("NavMeshPath.GetCornersNonAlloc copies corner count", path.corners.Length, nonAllocCount);

                    Vector3 target = new Vector3(1.2f, 0f, -0.2f);
                    bool setOk = agent.SetDestination(target);
                    Check.Equal("NavMeshAgent.SetDestination returns true", true, setOk);
                    Check.That("NavMeshAgent.pathStatus is PathComplete", agent.pathStatus == NavMeshPathStatus.PathComplete);
                    Check.Equal("NavMeshAgent.hasPath is true after SetDestination", true, agent.hasPath);
                    float initialRem = agent.remainingDistance;
                    Check.That("Initial remainingDistance > 3.5", initialRem > 3.5f);

                    agent.isStopped = true;
                    host.Tick(0.1);
                    host.Tick(0.1);
                    Check.That("Stopped agent does not advance", Math.Abs(runnerObj.transform.position.x - (-3.2f)) < 0.05f);

                    agent.isStopped = false;
                    for (int step = 0; step < 30; step++)
                    {
                        host.Tick(0.05);
                    }

                    Vector3 finalPos = runnerObj.transform.position;
                    Check.That("Stepping host moves agent transform.position to destination x", Math.Abs(finalPos.x - target.x) < 0.15f);
                    Check.That("Stepping host moves agent transform.position to destination z", Math.Abs(finalPos.z - target.z) < 0.15f);
                    Check.That("Remaining distance decreases to near zero", agent.remainingDistance <= 0.1f);

                    bool warped = agent.Warp(new Vector3(-3.0f, 0f, 0.2f));
                    Check.Equal("NavMeshAgent.Warp onto navmesh returns true", true, warped);
                    Check.That("Warp updates transform.position x", Math.Abs(runnerObj.transform.position.x - (-3.0f)) < 0.01f);
                    Check.Equal("Warp clears path", false, agent.hasPath);

                    agent.SetDestination(target);
                    Check.Equal("SetDestination after Warp restores hasPath", true, agent.hasPath);
                    agent.ResetPath();
                    Check.Equal("ResetPath clears hasPath", false, agent.hasPath);
                    Check.Equal("ResetPath zeroes remainingDistance", 0f, agent.remainingDistance);
                }
            }
            finally
            {
                NavMesh.Provider = null;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestNavMeshObstacleCarvingDetour()
        {
            PurgeStaleNavigationObjects();
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_nav_carve_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "NavCarveTest.unity");
            File.WriteAllText(scenePath, NavigationSceneYaml);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    if (NavMesh.Provider == null)
                    {
                        NavMesh.Provider = new RuntimeNavigationProvider(host.Handle);
                    }
                    host.LoadScene(scenePath);

                    GameObject? blockerObj = GameObject.Find("NavBlocker");
                    Check.That("NavBlocker exists in loaded scene", blockerObj != null);
                    if (blockerObj == null)
                    {
                        return;
                    }

                    NavMeshObstacle obstacle = blockerObj.GetComponent<NavMeshObstacle>() ?? blockerObj.AddComponent<NavMeshObstacle>();
                    obstacle.center = Vector3.zero;
                    obstacle.size = new Vector3(1.8f, 2.0f, 1.8f);
                    obstacle.carveOnlyStationary = false;
                    obstacle.carving = true;
                    Check.Equal("NavMeshObstacle.carving is true", true, obstacle.carving);

                    NavMeshPath detouredPath = new NavMeshPath();
                    bool foundDetour = NavMesh.CalculatePath(
                        new Vector3(-3.2f, 0f, -0.2f),
                        new Vector3(1.2f, 0f, -0.2f),
                        NavMesh.AllAreas,
                        detouredPath);
                    Check.Equal("CalculatePath around carving obstacle succeeds", true, foundDetour);
                    Check.That("Detoured path is PathComplete", detouredPath.status == NavMeshPathStatus.PathComplete);
                    bool goesNorth = false;
                    for (int i = 0; i < detouredPath.corners.Length; i++)
                    {
                        if (detouredPath.corners[i].z > 0.9f)
                        {
                            goesNorth = true;
                            break;
                        }
                    }
                    Check.Equal("Detoured path routes through northern cells around carved center cell", true, goesNorth);
                }
            }
            finally
            {
                NavMesh.Provider = null;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void PurgeStaleNavigationObjects()
        {
            string[] names = { "NavRunner", "NavBlocker", "FloorWest", "FloorMid", "FloorEast", "FloorNorthMid", "FloorNorthWest", "FloorNorthEast" };
            for (int n = 0; n < names.Length; n++)
            {
                for (int guard = 0; guard < 20; guard++)
                {
                    GameObject? stale = GameObject.Find(names[n]);
                    if (stale == null)
                    {
                        break;
                    }
                    UnityEngine.Object.Destroy(stale);
                }
            }
        }
    }
}
