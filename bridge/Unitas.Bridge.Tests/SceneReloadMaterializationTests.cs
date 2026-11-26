using System;
using System.IO;
using Unitas.Bridge;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    /// <summary>
    /// The objects of a scene exist right after the scene loads, so a lookup finds
    /// them without waiting for a tick. The objects of the replaced scene stay
    /// destroyed, so the lookup cannot hand back a stale object.
    /// </summary>
    public static class SceneReloadMaterializationTests
    {
        public static void Run()
        {
            ReloadedSceneObjectsAreFindableWithoutATick();
        }

        private static void ReloadedSceneObjectsAreFindableWithoutATick()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_scene_reload_materialization_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "scene_reload_materialization.unity");
            File.WriteAllText(scenePath, NamedObjectScene);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    GameObject? first = GameObject.Find("SceneMarker");
                    Check.That("the first load materialises the scene object", first != null);

                    host.LoadScene(scenePath, LoadSceneMode.Single);
                    GameObject? second = GameObject.Find("SceneMarker");
                    Check.That("the reload makes the scene object findable without a tick", second != null);
                    Check.That(
                        "the reload does not hand back the replaced scene's object",
                        !ReferenceEquals(first, second));
                    Check.That("the replaced scene's object is destroyed", first is not null && first.IsDestroyed);
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

        private const string NamedObjectScene = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: SceneMarker
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 2, z: 0}
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
";
    }
}
