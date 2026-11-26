using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    /// <summary>
    /// A single-mode scene load replaces the scene it loads over. The replaced
    /// scene's objects must not survive as findable objects, or a lookup hands
    /// back a stale object whose transform no longer follows the simulation.
    /// </summary>
    public static class SceneIsolationTests
    {
        private const ulong BallGuidHi = 0x1111222233334444UL;
        private const ulong BallGuidLo = 0x5555666677778888UL;
        private const float SpawnHeight = 5f;

        public static void Run()
        {
            ReloadingASceneDestroysTheReplacedScenesObjects();
            ReloadingASceneKeepsTheReloadedSceneUsable();
        }

        private sealed class FallingBallProbe : MonoBehaviour
        {
            public override void Update()
            {
                Rigidbody? body = GetComponent<Rigidbody>();
                if (body != null)
                {
                    body.velocity = new Vector3(3f, body.velocity.y, 0f);
                }
            }
        }

        private static void ReloadingASceneDestroysTheReplacedScenesObjects()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_scene_isolation_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "scene_isolation.unity");
            File.WriteAllText(scenePath, FallingBallScene);

            try
            {
                ScriptGlueRegistry.Clear();
                List<FallingBallProbe> probes = new List<FallingBallProbe>();
                ScriptGlueRegistry.Register(BallGuidHi, BallGuidLo, () =>
                {
                    FallingBallProbe probe = new FallingBallProbe();
                    probes.Add(probe);
                    return probe;
                });

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    GameObject? first = GameObject.Find("SceneBall");
                    Check.That("the first load materialises the scene object", first != null);
                    for (int tick = 0; tick < 5; tick++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That(
                        "the scene object follows the simulation",
                        first != null && first.transform.position.y < SpawnHeight);

                    host.LoadScene(scenePath, LoadSceneMode.Single);
                    host.Tick(0.02);
                    GameObject? second = GameObject.Find("SceneBall");
                    Check.That("the reload materialises the scene object again", second != null);
                    Check.That(
                        "the reload does not hand back the replaced scene's object",
                        !ReferenceEquals(first, second));
                    Check.That("the replaced scene's object is destroyed", first is not null && first.IsDestroyed);
                    Check.That("the replaced scene's object compares equal to null", first == null);
                    Check.That("the reload creates a second script instance", probes.Count >= 2);

                    for (int tick = 0; tick < 5; tick++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That(
                        "the live object follows the simulation",
                        second != null && second.transform.position.y < SpawnHeight);
                    Check.That(
                        "the live object sits at the simulated body position",
                        second != null
                            && host.TryGetBodyPosition(second.NativeEntityId, out Vector3 bodyPosition)
                            && (second.transform.position - bodyPosition).sqrMagnitude < 0.0001f);
                }
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

        /// <summary>
        /// The runtime hands a reloaded scene different entity ids than the
        /// first load used, because it already gave entities out. The managed
        /// objects of the reloaded scene have to follow those ids, or they lose
        /// their names, their transforms and their physics.
        /// </summary>
        private static void ReloadingASceneKeepsTheReloadedSceneUsable()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_scene_reload_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "scene_reload.unity");
            File.WriteAllText(scenePath, FallingBallScene);

            try
            {
                ScriptGlueRegistry.Clear();
                ScriptGlueRegistry.Register(BallGuidHi, BallGuidLo, () => new FallingBallProbe());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    GameObject? first = GameObject.Find("SceneBall");
                    Check.That("the first load materialises the scene object", first != null);

                    // A runtime object takes an entity id, so the runtime hands
                    // the reloaded scene ids that start where the first load's
                    // ids did not.
                    GameObject marker = new GameObject("RuntimeMarker");
                    Check.That("the runtime object took an entity id", marker.NativeEntityId != 0);

                    host.LoadScene(scenePath, LoadSceneMode.Single);
                    host.Tick(0.02);
                    GameObject? reloaded = GameObject.Find("SceneBall");
                    Check.That("the reloaded scene keeps the names of its objects", reloaded != null);
                    Check.That(
                        "the reloaded scene keeps the transforms of its objects",
                        reloaded != null && Math.Abs(reloaded.transform.position.y - SpawnHeight) < 0.5f);
                    Check.That(
                        "the reloaded scene keeps the physics of its objects",
                        reloaded != null && reloaded.GetComponent<Rigidbody>() != null);
                    Check.That(
                        "the reloaded object carries the entity the runtime gave it",
                        reloaded != null
                            && host.TryGetBodyPosition(reloaded.NativeEntityId, out Vector3 bodyPosition)
                            && (reloaded.transform.position - bodyPosition).sqrMagnitude < 0.0001f);
                }
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

        private const string FallingBallScene = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: SceneBall
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 5, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!54 &1002
Rigidbody:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Mass: 1
  m_UseGravity: 1
  m_IsKinematic: 0
--- !u!135 &1003
SphereCollider:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Radius: 0.5
  m_Center: {x: 0, y: 0, z: 0}
--- !u!114 &1004
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 11112222333344445555666677778888, type: 3}
";
    }
}
