using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    /// <summary>
    /// End-to-end coverage of the scene management slice against the real native runtime: the managed facade
    /// drives the native scene ABI through <see cref="NativeSceneBackend"/>, so every assertion here proves
    /// the native behaviour rather than a test double.
    /// </summary>
    public static class SceneManagementIntegrationTests
    {
        private const string AlphaGuid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        private const string BetaGuid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        private const string AlphaFixture = "crates/unitas-runtime/tests/fixtures/scene_alpha.unity";
        private const string BetaFixture = "crates/unitas-runtime/tests/fixtures/scene_beta.unity";
        private const string MissingFixture = "crates/unitas-runtime/tests/fixtures/does_not_exist.unity";
        private const string AlphaSceneName = "scene_alpha";
        private const string BetaSceneName = "scene_beta";
        private const string AlphaSceneFile = "scene_alpha.unity";
        private const string BetaSceneFile = "scene_beta.unity";

        public static void Run()
        {
            SingleLoadReplacesSceneSet();
            AdditiveLoadPreservesExistingScenes();
            ActiveSceneIsQueryableAndChangeable();
            UnloadTearsDownInOrder();
            DontDestroyOnLoadSurvivesSingleLoad();
            AsyncOperationsCompleteOnTheNextTick();
            FailedLoadLeavesSceneSetIntact();
            FailedUnloadLeavesSceneSetIntact();
            LoadByNameAndByBuildIndex();
        }

        private static void SingleLoadReplacesSceneSet()
        {
            string alpha = ResolveFixture(AlphaFixture);
            string beta = ResolveFixture(BetaFixture);
            Check.That("the alpha scene fixture exists", File.Exists(alpha));
            Check.That("the beta scene fixture exists", File.Exists(beta));

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                Check.That("the alpha script ticks before the replacement load", alphaScript.UpdateCount > 0);

                host.LoadScene(beta, LoadSceneMode.Single);
                host.Tick(0.016);

                Check.Equal("a single load leaves exactly one scene loaded", 1, SceneManager.sceneCount);
                Check.That("the replaced scene script is torn down", alphaScript.DestroyCount > 0);
                int alphaUpdates = alphaScript.UpdateCount;
                host.Tick(0.016);
                Check.Equal("the replaced scene script receives no further updates", alphaUpdates, alphaScript.UpdateCount);
                Check.That("the replacement scene script is alive and ticking", betaScript.UpdateCount > 0);
            }
        }

        private static void AdditiveLoadPreservesExistingScenes()
        {
            string alpha = ResolveFixture(AlphaFixture);

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                Check.That("the alpha script ticks before the additive load", alphaScript.UpdateCount > 0);

                SceneManager.LoadScene(BetaSceneName, LoadSceneMode.Additive);
                host.Tick(0.016);

                Check.Equal("an additive load keeps both scenes loaded", 2, SceneManager.sceneCount);
                Check.That("the additive load leaves the first scene script alive", alphaScript.DestroyCount == 0);

                int alphaUpdates = alphaScript.UpdateCount;
                int betaUpdates = betaScript.UpdateCount;
                host.Tick(0.016);
                Check.That("the first scene script keeps ticking after the additive load", alphaScript.UpdateCount > alphaUpdates);
                Check.That("the additive scene script ticks", betaScript.UpdateCount > betaUpdates);
            }
        }

        private static void ActiveSceneIsQueryableAndChangeable()
        {
            string alpha = ResolveFixture(AlphaFixture);

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                SceneManager.LoadScene(BetaSceneName, LoadSceneMode.Additive);
                host.Tick(0.016);

                Scene active = SceneManager.GetActiveScene();
                Check.That("an active scene is reported", active.isLoaded);
                Check.That(
                    "the reported active scene is one of the loaded scenes",
                    StemOf(active) == AlphaSceneName || StemOf(active) == BetaSceneName);

                Scene first = SceneManager.GetSceneAt(0);
                Scene second = SceneManager.GetSceneAt(1);
                Check.That("the two loaded scenes are distinct", first.handle != second.handle);

                bool switched = SceneManager.SetActiveScene(second);
                Check.That("the active scene switch is accepted", switched);
                Check.Equal("the active scene is now the requested scene", StemOf(second), StemOf(SceneManager.GetActiveScene()));
                Check.Equal("switching the active scene keeps the scene count", 2, SceneManager.sceneCount);

                bool switchedBack = SceneManager.SetActiveScene(first);
                Check.That("the active scene switch back is accepted", switchedBack);
                Check.Equal("the active scene is now the first scene", StemOf(first), StemOf(SceneManager.GetActiveScene()));
                Check.Equal("switching the active scene back keeps the scene count", 2, SceneManager.sceneCount);
            }
        }

        private static void UnloadTearsDownInOrder()
        {
            string alpha = ResolveFixture(AlphaFixture);

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                SceneManager.LoadScene(BetaSceneName, LoadSceneMode.Additive);
                host.Tick(0.016);
                Check.Equal("both scenes are loaded before the unload", 2, SceneManager.sceneCount);

                SceneIntegrationEventLog.Clear();
                SceneManager.sceneUnloaded += OnSceneUnloaded;

                SceneManager.UnloadScene(AlphaSceneName);
                host.Tick(0.016);

                int disableIndex = SceneIntegrationEventLog.IndexOf("alpha:OnDisable");
                int destroyIndex = SceneIntegrationEventLog.IndexOf("alpha:OnDestroy");
                int unloadedIndex = SceneIntegrationEventLog.IndexOf("sceneUnloaded");

                Check.That("the unloaded scene script observed OnDisable", disableIndex >= 0);
                Check.That("the unloaded scene script observed OnDestroy", destroyIndex >= 0);
                Check.That("OnDisable is observed before OnDestroy", disableIndex >= 0 && destroyIndex > disableIndex);
                Check.That(
                    "the sceneUnloaded event is observed after both teardown callbacks",
                    unloadedIndex >= 0 && unloadedIndex > destroyIndex);
                Check.Equal("the unload drops the scene count to one", 1, SceneManager.sceneCount);

                int betaUpdates = betaScript.UpdateCount;
                host.Tick(0.016);
                Check.That("the remaining scene script keeps ticking after the unload", betaScript.UpdateCount > betaUpdates);

                SceneManager.sceneUnloaded -= OnSceneUnloaded;
            }
        }

        private static void OnSceneUnloaded(Scene scene)
        {
            SceneIntegrationEventLog.Record("sceneUnloaded");
        }

        private static void DontDestroyOnLoadSurvivesSingleLoad()
        {
            string alpha = ResolveFixture(AlphaFixture);
            string beta = ResolveFixture(BetaFixture);

            ScriptRegistry.Clear();
            PersistentSceneIntegrationRecorder persistent = new PersistentSceneIntegrationRecorder();
            ScriptRegistry.Register(AlphaGuid, () => persistent);
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                Check.That("the persistent recorder captured an instance id", persistent.GameObjectInstanceId != 0);
                Check.Equal(
                    "the persistent recorder captured its marker",
                    PersistentSceneIntegrationRecorder.MarkerText,
                    persistent.Marker);
                Check.That("the persistent recorder ticks before the replacement load", persistent.UpdateCount > 0);

                host.LoadScene(beta, LoadSceneMode.Single);
                host.Tick(0.016);

                Check.Equal(
                    "the persistent recorder keeps the same instance id",
                    persistent.GameObjectInstanceId,
                    persistent.gameObject.GetInstanceID());
                Check.Equal(
                    "the persistent recorder keeps its captured field",
                    PersistentSceneIntegrationRecorder.MarkerText,
                    persistent.Marker);
                Check.Equal("the persistent recorder is not destroyed by the single load", 0, persistent.DestroyCount);
                Check.That("the persistent game object is flagged persistent", persistent.gameObject.IsPersistent);

                int updates = persistent.UpdateCount;
                host.Tick(0.016);
                Check.That("the persistent recorder keeps ticking after the single load", persistent.UpdateCount > updates);
            }
        }

        private static void AsyncOperationsCompleteOnTheNextTick()
        {
            string alpha = ResolveFixture(AlphaFixture);

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                AsyncOperation load = SceneManager.LoadSceneAsync(BetaSceneName, LoadSceneMode.Single);
                Check.That("the async load operation is not done before the tick boundary", !load.isDone);
                int loadCompletions = 0;
                load.completed += (AsyncOperation operation) => loadCompletions++;
                Check.Equal("the async load completion callback has not fired yet", 0, loadCompletions);

                host.Tick(0.016);
                Check.That("the async load operation is done after the tick", load.isDone);
                Check.Equal("the async load operation progress is one after the tick", 1f, load.progress);
                Check.Equal("the async load completion callback fires once after the next tick", 1, loadCompletions);
                host.Tick(0.016);
                Check.Equal("the async load completion callback does not fire twice", 1, loadCompletions);
                Check.Equal("the async single load left one scene loaded", 1, SceneManager.sceneCount);
                Check.Equal(
                    "the async single load made the loaded scene active",
                    BetaSceneName,
                    StemOf(SceneManager.GetActiveScene()));

                AsyncOperation unload = SceneManager.UnloadSceneAsync(BetaSceneName);
                Check.That("the async unload operation is not done before the tick boundary", !unload.isDone);
                int unloadCompletions = 0;
                unload.completed += (AsyncOperation operation) => unloadCompletions++;
                Check.Equal("the async unload completion callback has not fired yet", 0, unloadCompletions);

                host.Tick(0.016);
                Check.That("the async unload operation is done after the tick", unload.isDone);
                Check.Equal("the async unload operation progress is one after the tick", 1f, unload.progress);
                Check.Equal("the async unload completion callback fires once after the next tick", 1, unloadCompletions);
                host.Tick(0.016);
                Check.Equal("the async unload completion callback does not fire twice", 1, unloadCompletions);
                Check.Equal("the async unload dropped the scene count to zero", 0, SceneManager.sceneCount);
            }
        }

        private static void FailedLoadLeavesSceneSetIntact()
        {
            string alpha = ResolveFixture(AlphaFixture);
            string missing = ResolveFixture(MissingFixture);
            Check.That("the missing scene fixture does not exist", !File.Exists(missing));

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                Check.That("the alpha script ticks before the failed load", alphaScript.UpdateCount > 0);
                Check.Equal("one scene is loaded before the failed load", 1, SceneManager.sceneCount);

                int countBefore = SceneManager.sceneCount;
                string activeBefore = StemOf(SceneManager.GetActiveScene());

                bool threw = false;
                string errorText = string.Empty;
                try
                {
                    SceneManager.LoadScene("does_not_exist", LoadSceneMode.Single);
                }
                catch (SceneManagementException ex)
                {
                    threw = true;
                    errorText = ex.Message;
                }

                Check.That("loading an unknown scene name throws", threw);
                Check.That("the failed load reports an error message", errorText.Length > 0);
                Check.Equal("the failed load leaves the scene count unchanged", countBefore, SceneManager.sceneCount);
                Check.Equal(
                    "the failed load leaves the active scene unchanged",
                    activeBefore,
                    StemOf(SceneManager.GetActiveScene()));

                int updatesBefore = alphaScript.UpdateCount;
                host.Tick(0.016);
                Check.That("the loaded scene keeps ticking after the failed load", alphaScript.UpdateCount > updatesBefore);

                host.Provider = new IntegrationSceneProvider(alpha, missing);
                bool nativeThrew = false;
                string nativeErrorText = string.Empty;
                try
                {
                    SceneManager.LoadScene("does_not_exist", LoadSceneMode.Single);
                }
                catch (SceneManagementException ex)
                {
                    nativeThrew = true;
                    nativeErrorText = ex.Message;
                }

                Check.That("loading a scene file that does not exist throws", nativeThrew);
                Check.That("the failed native load reports an error message", nativeErrorText.Length > 0);
                Check.Equal("the failed native load leaves the scene count unchanged", countBefore, SceneManager.sceneCount);
                Check.Equal(
                    "the failed native load leaves the active scene unchanged",
                    activeBefore,
                    StemOf(SceneManager.GetActiveScene()));

                string? nativeError = host.GetErrorMessage();
                Check.That("the native error message is surfaced after the failed load", !string.IsNullOrEmpty(nativeError));

                int updatesAfter = alphaScript.UpdateCount;
                host.Tick(0.016);
                Check.That("the loaded scene keeps ticking after the failed native load", alphaScript.UpdateCount > updatesAfter);
            }
        }

        private static void FailedUnloadLeavesSceneSetIntact()
        {
            string alpha = ResolveFixture(AlphaFixture);

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);
                Check.Equal("one scene is loaded before the failed unload", 1, SceneManager.sceneCount);

                int countBefore = SceneManager.sceneCount;
                string activeBefore = StemOf(SceneManager.GetActiveScene());

                bool threw = false;
                string errorText = string.Empty;
                try
                {
                    SceneManager.UnloadScene("NoSuchScene");
                }
                catch (SceneManagementException ex)
                {
                    threw = true;
                    errorText = ex.Message;
                }

                Check.That("unloading an unknown scene throws", threw);
                Check.That("the failed unload reports an error message", errorText.Length > 0);
                Check.Equal("the failed unload leaves the scene count unchanged", countBefore, SceneManager.sceneCount);
                Check.Equal(
                    "the failed unload leaves the active scene unchanged",
                    activeBefore,
                    StemOf(SceneManager.GetActiveScene()));

                string? nativeError = host.GetErrorMessage();
                Check.That("the native error message is surfaced after the failed unload", !string.IsNullOrEmpty(nativeError));

                int updatesBefore = alphaScript.UpdateCount;
                host.Tick(0.016);
                Check.That("the loaded scene keeps ticking after the failed unload", alphaScript.UpdateCount > updatesBefore);
            }
        }

        private static void LoadByNameAndByBuildIndex()
        {
            string alpha = ResolveFixture(AlphaFixture);
            string beta = ResolveFixture(BetaFixture);

            ScriptRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                LoadedSceneFileProvider provider = (LoadedSceneFileProvider)host.Provider;
                int alphaIndex = IndexOfSceneFile(provider.ScenePaths, AlphaSceneFile);
                int betaIndex = IndexOfSceneFile(provider.ScenePaths, BetaSceneFile);
                Check.That("the alpha scene has a build index", alphaIndex >= 0);
                Check.That("the beta scene has a build index", betaIndex >= 0);

                Scene byName = SceneManager.GetSceneByName(AlphaSceneName);
                Check.Equal("the alpha scene resolves by name to its path", AlphaSceneFile, FileNameOf(byName.path));
                Check.Equal("the alpha scene resolved by name reports its build index", alphaIndex, byName.buildIndex);

                Scene byIndex = SceneManager.GetSceneByBuildIndex(alphaIndex);
                Check.Equal("the alpha scene resolves by build index to the same path", AlphaSceneFile, FileNameOf(byIndex.path));
                Check.Equal("the alpha scene resolved by build index reports its name", AlphaSceneName, StemOf(byIndex));

                SceneManager.LoadScene(AlphaSceneName, LoadSceneMode.Single);
                host.Tick(0.016);
                Check.Equal("loading by resolved name loads one scene", 1, SceneManager.sceneCount);
                Check.Equal(
                    "the scene loaded by resolved name reports its name",
                    AlphaSceneName,
                    StemOf(SceneManager.GetSceneAt(0)));
                Check.Equal(
                    "the scene loaded by resolved name reports its path",
                    AlphaSceneFile,
                    FileNameOf(SceneManager.GetSceneAt(0).path));

                Scene betaByIndex = SceneManager.GetSceneByBuildIndex(betaIndex);
                Check.Equal("the beta scene resolves by build index to its path", BetaSceneFile, FileNameOf(betaByIndex.path));
                host.LoadScene(betaByIndex.path, LoadSceneMode.Single);
                host.Tick(0.016);
                Check.Equal("loading by resolved build index loads one scene", 1, SceneManager.sceneCount);
                Check.Equal(
                    "the scene loaded by resolved build index reports its name",
                    BetaSceneName,
                    StemOf(SceneManager.GetSceneAt(0)));
                Check.Equal(
                    "the scene loaded by resolved build index reports its path",
                    BetaSceneFile,
                    FileNameOf(SceneManager.GetSceneAt(0).path));
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

        private static int IndexOfSceneFile(IReadOnlyList<string> paths, string fileName)
        {
            for (int i = 0; i < paths.Count; i++)
            {
                if (string.Equals(FileNameOf(paths[i]), fileName, StringComparison.Ordinal))
                {
                    return i;
                }
            }

            return -1;
        }

        private static string FileNameOf(string path) => Path.GetFileName(path);

        private static string StemOf(Scene scene) => Path.GetFileNameWithoutExtension(scene.path);

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }

    /// <summary>
    /// Scene provider with an explicit scene list so a test can point a scene name at a file that is not on disk.
    /// </summary>
    internal sealed class IntegrationSceneProvider : ISceneProvider
    {
        private readonly List<string> _scenePaths = new List<string>();

        public IntegrationSceneProvider(params string[] scenePaths)
        {
            for (int i = 0; i < scenePaths.Length; i++)
            {
                _scenePaths.Add(scenePaths[i]);
            }
        }

        public IReadOnlyList<string> ScenePaths => _scenePaths;

        public bool TryResolveScenePath(string sceneName, out string scenePath)
        {
            for (int i = 0; i < _scenePaths.Count; i++)
            {
                if (string.Equals(Path.GetFileNameWithoutExtension(_scenePaths[i]), sceneName, StringComparison.Ordinal))
                {
                    scenePath = _scenePaths[i];
                    return true;
                }
            }

            scenePath = string.Empty;
            return false;
        }

        public bool TryResolveScenePathByBuildIndex(int buildIndex, out string scenePath)
        {
            if (buildIndex < 0 || buildIndex >= _scenePaths.Count)
            {
                scenePath = string.Empty;
                return false;
            }

            scenePath = _scenePaths[buildIndex];
            return true;
        }
    }
}
