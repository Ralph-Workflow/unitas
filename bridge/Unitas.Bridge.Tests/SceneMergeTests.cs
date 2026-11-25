using System;
using System.Collections.Generic;
using Unitas.Bridge;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public sealed class SceneMergeProbe : MonoBehaviour
    {
    }

    public static class SceneMergeTests
    {
        public static void Run()
        {
            MergeScenesMovesSourceRootsIntoTheDestinationAndUnloadsTheSource();
            MergeScenesMovesChildHierarchiesAndComponentSceneAssignments();
            MergeScenesRaisesSceneUnloadedForTheSourceScene();
            MergeScenesRefusesAnUnloadedSourceOrDestination();
            MergeScenesLeavesTheDestinationAsTheOnlySceneThatCannotBeUnloaded();
            AmbiguousPathQueryLoadsTheFirstMatchAndListsEveryMatch();
        }

        private static void MergeScenesMovesSourceRootsIntoTheDestinationAndUnloadsTheSource()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                Scene source = SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                GameObject root = new GameObject("SourceRoot");
                Scene destination = SceneManager.LoadScene("Level2", LoadSceneMode.Additive);
                GameObject destinationRoot = new GameObject("DestinationRoot");

                Check.That("the source root starts in the source scene", root.scene == source);
                Check.That("the destination root starts in the destination scene", destinationRoot.scene == destination);

                SceneManager.MergeScenes(source, destination);

                Check.Equal("the merge leaves one loaded scene", 1, SceneManager.sceneCount);
                Check.That("the source scene is unloaded after the merge", !SceneManager.GetSceneByName("Level1").isLoaded);
                Check.That("the destination scene stays loaded after the merge", SceneManager.GetSceneByName("Level2").isLoaded);
                Check.That("the merged root belongs to the destination scene", root.scene == destination);
                Check.That("the destination root keeps the destination scene", destinationRoot.scene == destination);
                Check.That("the merged root survives the source scene unload", root != null);
            }
        }

        private static void MergeScenesMovesChildHierarchiesAndComponentSceneAssignments()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                Scene source = SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                GameObject root = new GameObject("SourceRoot");
                GameObject child = new GameObject("SourceChild");
                child.transform.SetParent(root.transform);
                SceneMergeProbe probe = root.AddComponent<SceneMergeProbe>();
                Scene destination = SceneManager.LoadScene("Level2", LoadSceneMode.Additive);

                Check.That("the child starts in the source scene", child.scene == source);
                Check.That("the component starts in the source scene", probe.scene == source);

                SceneManager.MergeScenes(source, destination);

                Check.That("the merged child belongs to the destination scene", child.scene == destination);
                Check.That("the merged component belongs to the destination scene", probe.scene == destination);
                Check.That("the merged child keeps its parent", child.transform.parent == root.transform);
                Check.That("the merged child is still alive", child != null);
            }
        }

        private static void MergeScenesRaisesSceneUnloadedForTheSourceScene()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            List<string> unloadedScenes = new List<string>();
            Action<Scene>? onUnloaded = scene => unloadedScenes.Add(scene.name);
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.sceneUnloaded += onUnloaded;
                try
                {
                    Scene source = SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                    Scene destination = SceneManager.LoadScene("Level2", LoadSceneMode.Additive);
                    SceneManager.MergeScenes(source, destination);
                }
                finally
                {
                    SceneManager.sceneUnloaded -= onUnloaded;
                }

                Check.Equal("the merge raises sceneUnloaded once", 1, unloadedScenes.Count);
                if (unloadedScenes.Count == 1)
                {
                    Check.Equal("the merge reports the source scene as unloaded", "Level1", unloadedScenes[0]);
                }
            }
        }

        private static void MergeScenesRefusesAnUnloadedSourceOrDestination()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                Scene source = SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                Scene destination = SceneManager.LoadScene("Level2", LoadSceneMode.Additive);

                bool invalidDestinationThrew = false;
                try
                {
                    SceneManager.MergeScenes(source, default);
                }
                catch (ArgumentException)
                {
                    invalidDestinationThrew = true;
                }

                Check.That("merging into an invalid destination throws ArgumentException", invalidDestinationThrew);

                bool invalidSourceThrew = false;
                try
                {
                    SceneManager.MergeScenes(default, destination);
                }
                catch (ArgumentException)
                {
                    invalidSourceThrew = true;
                }

                Check.That("merging an invalid source throws ArgumentException", invalidSourceThrew);
                Check.Equal("the refused merges leave both scenes loaded", 2, SceneManager.sceneCount);
            }
        }

        private static void MergeScenesLeavesTheDestinationAsTheOnlySceneThatCannotBeUnloaded()
        {
            SceneManager.ClearBuildScenes();
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes("assets/Level1.unity", "assets/Level2.unity");
                SceneManager.EnforceBuildList = true;

                try
                {
                    Scene source = SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                    Scene destination = SceneManager.LoadScene("Level2", LoadSceneMode.Additive);
                    SceneManager.MergeScenes(source, destination);

                    Check.Equal("the merge leaves the destination as the only loaded scene", 1, SceneManager.sceneCount);

                    bool unloadThrew = false;
                    try
                    {
                        SceneManager.UnloadSceneAsync("Level2");
                    }
                    catch (SceneManagementException)
                    {
                        unloadThrew = true;
                    }

                    Check.That("unloading the only scene left behind by a merge is refused", unloadThrew);
                    Check.Equal("the refused unload keeps the destination loaded", 1, SceneManager.sceneCount);
                    Check.Equal("the refused unload keeps the destination active", "Level2", SceneManager.GetActiveScene().name);
                }
                finally
                {
                    SceneManager.ClearBuildScenes();
                }
            }
        }

        private static void AmbiguousPathQueryLoadsTheFirstMatchAndListsEveryMatch()
        {
            SceneManager.ClearBuildScenes();
            LogRecorder.Clear();
            FakeSceneProvider provider = new FakeSceneProvider(
                "Assets/Regions/North/Dungeon.unity",
                "Assets/Regions/South/Dungeon.unity");
            FakeSceneBackend backend = new FakeSceneBackend();

            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes(
                    "Assets/Regions/North/Dungeon.unity",
                    "Assets/Regions/South/Dungeon.unity");

                try
                {
                    Scene loaded = SceneManager.LoadScene("Dungeon.unity", LoadSceneMode.Single);
                    Check.Equal(
                        "the ambiguous path query loads the first matching scene",
                        "Assets/Regions/North/Dungeon.unity",
                        loaded.path);

                    IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                    Check.Equal("the ambiguous path query logs exactly one warning", 1, warnings.Count);
                    if (warnings.Count == 1)
                    {
                        Check.That(
                            "the warning lists the first matching scene",
                            warnings[0].Message.Contains("Assets/Regions/North/Dungeon.unity", StringComparison.Ordinal));
                        Check.That(
                            "the warning lists the second matching scene",
                            warnings[0].Message.Contains("Assets/Regions/South/Dungeon.unity", StringComparison.Ordinal));
                    }
                }
                finally
                {
                    SceneManager.ClearBuildScenes();
                    LogRecorder.Clear();
                }
            }
        }
    }
}
