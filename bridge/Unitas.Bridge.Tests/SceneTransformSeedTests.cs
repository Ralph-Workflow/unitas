using System.Collections.Generic;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class TransformSeedProbeScript : MonoBehaviour
    {
        public string ObjectName { get; private set; } = string.Empty;

        public Vector3 LocalPosition { get; private set; }

        public Quaternion LocalRotation { get; private set; }

        public Vector3 LocalScale { get; private set; }

        public Vector3 WorldPosition { get; private set; }

        public override void Awake()
        {
            ObjectName = gameObject.name;
            LocalPosition = transform.localPosition;
            LocalRotation = transform.localRotation;
            LocalScale = transform.localScale;
            WorldPosition = transform.position;
        }
    }

    public static class SceneTransformSeedTests
    {
        internal const string FixtureRelativePath = "bridge/Unitas.Bridge.Tests/Fixtures/scene_transform_seeded.unity";
        internal const string ProbeGuid = "dddddddddddddddddddddddddddddddd";

        public static void Run()
        {
            TestSceneTransformIsSeededFromSerializedValues();
        }

        private static void TestSceneTransformIsSeededFromSerializedValues()
        {
            string scenePath = ResolveFixture(FixtureRelativePath);
            Check.That("Transform seed scene file exists", File.Exists(scenePath));

            List<TransformSeedProbeScript> probes = new List<TransformSeedProbeScript>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register<TransformSeedProbeScript>(ProbeGuid, () =>
            {
                TransformSeedProbeScript probe = new TransformSeedProbeScript();
                probes.Add(probe);
                return probe;
            });

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
            }

            TransformSeedProbeScript? root = FindProbe(probes, "SeededRoot");
            TransformSeedProbeScript? child = FindProbe(probes, "SeededChild");
            Check.That("Seeded root probe observed", root != null);
            Check.That("Seeded child probe observed", child != null);
            if (root == null || child == null)
            {
                return;
            }

            CheckVector3("Seeded root local position", new Vector3(-10f, 0.25f, 0f), root.LocalPosition);
            CheckQuaternion("Seeded root local rotation", new Quaternion(0f, 0.7071068f, 0f, 0.7071068f), root.LocalRotation);
            CheckVector3("Seeded root local scale", new Vector3(4f, 0.5f, 4f), root.LocalScale);
            CheckVector3("Seeded root world position", new Vector3(-10f, 0.25f, 0f), root.WorldPosition);

            CheckVector3("Seeded child local position", new Vector3(1f, 2f, 3f), child.LocalPosition);
            CheckVector3("Seeded child local scale", new Vector3(1f, 1f, 1f), child.LocalScale);
            CheckVector3("Seeded child world position under its parent", new Vector3(6f, 2f, 3f), child.WorldPosition);
        }

        private static TransformSeedProbeScript? FindProbe(List<TransformSeedProbeScript> probes, string objectName)
        {
            for (int index = 0; index < probes.Count; index++)
            {
                if (string.Equals(probes[index].ObjectName, objectName, System.StringComparison.Ordinal))
                {
                    return probes[index];
                }
            }

            return null;
        }

        private static void CheckVector3(string name, Vector3 expected, Vector3 actual)
        {
            Check.Equal(name + " x", expected.x, actual.x);
            Check.Equal(name + " y", expected.y, actual.y);
            Check.Equal(name + " z", expected.z, actual.z);
        }

        private static void CheckQuaternion(string name, Quaternion expected, Quaternion actual)
        {
            Check.Equal(name + " x", expected.x, actual.x);
            Check.Equal(name + " y", expected.y, actual.y);
            Check.Equal(name + " z", expected.z, actual.z);
            Check.Equal(name + " w", expected.w, actual.w);
        }

        internal static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
