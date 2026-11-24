using System;
using System.IO;
using Unitas.Bridge;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public static class SceneManagementPersistenceTests
    {
        private const string TargetGuid = "11112222333344445555666677778888";
        private const string OtherGuid = "22222222333344445555666677778888";
        private const string BetaGuid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        private const string FirstFixture = "crates/unitas-runtime/tests/fixtures/scene_physics_query.unity";
        private const string ReplacementFixture = "crates/unitas-runtime/tests/fixtures/scene_beta.unity";

        public static void Run()
        {
            DontDestroyOnLoadCoroutineSurvivesSingleLoad();
        }

        private static void DontDestroyOnLoadCoroutineSurvivesSingleLoad()
        {
            string firstScene = ResolveFixture(FirstFixture);
            string replacementScene = ResolveFixture(ReplacementFixture);
            Check.That("the first scene fixture exists", File.Exists(firstScene));
            Check.That("the replacement scene fixture exists", File.Exists(replacementScene));

            ScriptRegistry.Clear();
            PersistentSceneIntegrationRecorder persistent = new PersistentSceneIntegrationRecorder
            {
                StartCoroutineOnStart = true,
            };
            AlphaSceneRecorder transient = new AlphaSceneRecorder
            {
                StartCoroutineOnStart = true,
            };
            BetaSceneRecorder replacement = new BetaSceneRecorder();
            ScriptRegistry.Register(TargetGuid, () => persistent);
            ScriptRegistry.Register(OtherGuid, () => transient);
            ScriptRegistry.Register(BetaGuid, () => replacement);

            try
            {
                using (BridgeHost host = CreateHost(firstScene))
                {
                    host.LoadScene(firstScene, LoadSceneMode.Single);
                    host.Tick(0.016);
                    host.Tick(0.016);

                    int persistentTicksBeforeLoad = persistent.CoroutineTickCount;
                    int transientTicksBeforeLoad = transient.CoroutineTickCount;
                    Check.That("the persistent script coroutine advances before the replacement load", persistentTicksBeforeLoad >= 2);
                    Check.That("the transient script coroutine advances before the replacement load", transientTicksBeforeLoad >= 2);

                    host.LoadScene(replacementScene, LoadSceneMode.Single);
                    host.Tick(0.016);
                    host.Tick(0.016);

                    Check.Equal(
                        "the persistent script keeps the same game object instance id across a single load",
                        persistent.GameObjectInstanceId,
                        persistent.gameObject.GetInstanceID());
                    Check.That(
                        "the persistent script coroutine keeps advancing across ticks after a single load",
                        persistent.CoroutineTickCount > persistentTicksBeforeLoad);
                    Check.Equal(
                        "the replaced scene script coroutine is stopped and does not advance after a single load",
                        transientTicksBeforeLoad,
                        transient.CoroutineTickCount);
                    Check.That("the replacement scene script ticks after the single load", replacement.UpdateCount > 0);
                }
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(persistent);
                CoroutineScheduler.OwnerDestroyed(transient);
            }
        }

        private static BridgeHost CreateHost(string sceneRoot)
        {
            BridgeHost host = new BridgeHost();
            LoadedSceneFileProvider provider = new LoadedSceneFileProvider();
            provider.SetSceneRoot(sceneRoot);
            host.Provider = provider;
            return host;
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
