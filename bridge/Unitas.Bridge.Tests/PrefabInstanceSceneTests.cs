using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class PrefabInstanceProbe : MonoBehaviour
    {
        public string ObjectName { get; private set; } = string.Empty;

        public Vector3 WorldPosition { get; private set; }

        public Vector3 LocalPosition { get; private set; }

        public Vector3 BallPosition { get; private set; }

        public float DistanceToBall { get; private set; } = -1f;

        public bool BallFound { get; private set; }

        public override void Awake()
        {
            ObjectName = gameObject.name;
            WorldPosition = transform.position;
            LocalPosition = transform.localPosition;
        }

        public override void Update()
        {
            GameObject? ball = GameObject.Find("PlayerBall");
            BallFound = ball != null;
            WorldPosition = transform.position;
            LocalPosition = transform.localPosition;
            BallPosition = ball != null ? ball.transform.position : Vector3.zero;
            DistanceToBall = ball != null ? Vector3.Distance(ball.transform.position, transform.position) : -1f;
        }
    }

    public static class PrefabInstanceSceneTests
    {
        internal const string SampleSceneRelativePath = "samples/marble-rush/Assets/Scenes/MarbleRush.unity";
        internal const string PickupScriptGuid = "b2c3d4e5f60718293a4b5c6d7e8f90a1";

        public static void Run()
        {
            string scenePath = ResolveFixture(SampleSceneRelativePath);
            Console.WriteLine("[PREFABDIAG] scene " + scenePath + " exists=" + File.Exists(scenePath));

            List<PrefabInstanceProbe> probes = new List<PrefabInstanceProbe>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register<PrefabInstanceProbe>(PickupScriptGuid, () =>
            {
                PrefabInstanceProbe probe = new PrefabInstanceProbe();
                probes.Add(probe);
                return probe;
            });

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    for (int i = 0; i < probes.Count; i++)
                    {
                        PrefabInstanceProbe probe = probes[i];
                        Console.WriteLine(
                            "[PREFABDIAG] probe " + i
                            + " name=" + probe.ObjectName
                            + " world=" + probe.WorldPosition
                            + " local=" + probe.LocalPosition
                            + " ball=" + probe.BallPosition
                            + " dist=" + probe.DistanceToBall
                            + " ballFound=" + probe.BallFound);
                    }
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
            }
        }

        internal static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
