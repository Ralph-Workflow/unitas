using System;
using System.Globalization;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class SceneTrailRendererTests
    {
        private const string SceneRelativePath = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";
        private const string PlayerName = "PlayerBall";

        public static void Run()
        {
            SceneTrailRendererIsMaterialized();
        }

        private static void SceneTrailRendererIsMaterialized()
        {
            PurgeStalePlayers();
            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(ResolveFixture(SceneRelativePath));

                GameObject? player = GameObject.Find(PlayerName);
                Check.That("The scene player object exists once the scene loads", player != null);
                if (player == null)
                {
                    return;
                }

                TrailRenderer? trail = player.GetComponent<TrailRenderer>();
                Check.That("The scene player object carries a trail renderer component", trail != null);
                if (trail == null)
                {
                    ReportTrailEntities(player);
                    return;
                }

                // The values the scene authored for the trail; the managed fallback of the
                // component carries other defaults, so these can only come from the native trail.
                Check.Equal("Scene trail time", 0.35f, trail.time);
                Check.Equal("Scene trail minimum vertex distance", 0.1f, trail.minVertexDistance);

                ITrailProvider? provider = TrailRenderer.Provider;
                Check.That("The scene trail has a native provider", provider != null);
                if (provider == null)
                {
                    return;
                }

                Check.That(
                    "The native runtime knows the scene trail",
                    provider.TryGetTrailEmitting(player.NativeEntityId, out _));

                trail.emitting = false;
                Check.That(
                    "The native trail stops emitting on request",
                    provider.TryGetTrailEmitting(player.NativeEntityId, out bool stopped) && !stopped);
                trail.emitting = true;
                Check.That(
                    "The native trail emits on request",
                    provider.TryGetTrailEmitting(player.NativeEntityId, out bool emitting) && emitting);
            }
        }

        // A trail that never binds leaves no trace, so the diagnostic names the entity
        // the bridge gave the player and the entities the native runtime gave trails.
        private static void ReportTrailEntities(GameObject player)
        {
            Console.WriteLine("DIAG player entity " + player.NativeEntityId);
            ITrailProvider? provider = TrailRenderer.Provider;
            if (provider == null)
            {
                Console.WriteLine("DIAG no trail provider for " + player.name);
                return;
            }

            ulong first = player.NativeEntityId > 16 ? player.NativeEntityId - 16 : 1;
            for (ulong probe = first; probe <= player.NativeEntityId + 16; probe++)
            {
                if (provider.TryGetTrailEmitting(probe, out _))
                {
                    Console.WriteLine("DIAG native trail entity " + probe.ToString(CultureInfo.InvariantCulture));
                }
            }
        }

        // GameObject.Find returns the first live match process-wide, so a player object
        // left behind by an earlier run would shadow the one this test loads.
        private static void PurgeStalePlayers()
        {
            for (int guard = 0; guard < 100; guard++)
            {
                GameObject? stale = GameObject.Find(PlayerName);
                if (stale == null)
                {
                    return;
                }

                UnityEngine.Object.Destroy(stale);
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
