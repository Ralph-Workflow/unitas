using System;
using System.Collections.Generic;
using Unitas.Bridge;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public sealed class FakeSceneProvider : ISceneProvider
    {
        private readonly List<string> _scenePaths;

        public FakeSceneProvider(params string[] scenePaths)
        {
            _scenePaths = new List<string>(scenePaths);
        }

        public IReadOnlyList<string> ScenePaths => _scenePaths;

        public bool TryResolveScenePath(string sceneName, out string scenePath)
        {
            for (int index = 0; index < _scenePaths.Count; index++)
            {
                if (string.Equals(SceneName.Of(_scenePaths[index]), sceneName, StringComparison.Ordinal))
                {
                    scenePath = _scenePaths[index];
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

    public sealed class FakeSceneBackend : ISceneBackend
    {
        private readonly List<string> _scenes = new List<string>();
        private readonly List<string> _calls = new List<string>();

        public bool FailLoad { get; set; }

        public int DontDestroyOnLoadCalls { get; private set; }

        public List<string> Calls => _calls;

        public int SceneCount => _scenes.Count;

        public int ActiveSceneIndex { get; private set; } = -1;

        public string? GetSceneName(int index)
        {
            if (index < 0 || index >= _scenes.Count)
            {
                return null;
            }

            return _scenes[index];
        }

        public void LoadScene(string scenePath, LoadSceneMode mode)
        {
            _calls.Add("load:" + scenePath + ":" + mode);
            if (FailLoad)
            {
                throw new SceneManagementException("fake load failure for " + scenePath);
            }

            if (mode == LoadSceneMode.Single)
            {
                _scenes.Clear();
            }

            _scenes.Add(SceneName.Of(scenePath));
            ActiveSceneIndex = _scenes.Count - 1;
        }

        public void UnloadScene(string sceneName)
        {
            _calls.Add("unload:" + sceneName);
            if (!_scenes.Remove(sceneName))
            {
                throw new SceneManagementException("fake unload failure for " + sceneName);
            }

            ActiveSceneIndex = _scenes.Count - 1;
        }

        public bool SetActiveScene(int index)
        {
            if (index < 0 || index >= _scenes.Count)
            {
                return false;
            }

            ActiveSceneIndex = index;
            return true;
        }

        public void DontDestroyOnLoad(ulong entity)
        {
            DontDestroyOnLoadCalls++;
            _calls.Add("ddol:" + entity);
        }
    }

    internal static class SceneName
    {
        public static string Of(string path) => System.IO.Path.GetFileNameWithoutExtension(path);
    }

    public static class SceneManagementTests
    {
        public static void Run()
        {
            LoadSceneModeCodesMatchTheNativeAbi();
            SceneStructCarriesItsFields();
            ProviderResolvesSceneNamesAndBuildIndexes();
            AsyncOperationCompletesOnTheNextTick();
            SceneEventsFireAtTheCallBoundary();
            DontDestroyOnLoadFlagsManagedOnlyObjects();
            FailedLoadSurfacesSceneManagementException();
            NewObjectsTakeTheActiveScene();
            LoadSceneParametersDriveTheLoad();
        }

        private static void LoadSceneModeCodesMatchTheNativeAbi()
        {
            Check.Equal("LoadSceneMode.Single is the native mode code 0", 0, (int)LoadSceneMode.Single);
            Check.Equal("LoadSceneMode.Additive is the native mode code 1", 1, (int)LoadSceneMode.Additive);
        }

        private static void SceneStructCarriesItsFields()
        {
            Scene scene = new Scene("Level1", "assets/Level1.unity", 3, true, 2, 7);
            Check.Equal("scene name", "Level1", scene.name);
            Check.Equal("scene path", "assets/Level1.unity", scene.path);
            Check.Equal("scene build index", 3, scene.buildIndex);
            Check.That("scene is loaded", scene.isLoaded);
            Check.Equal("scene root count", 2, scene.rootCount);
            Check.Equal("scene handle", 7, scene.handle);
            Check.That("scene with a handle is valid", scene.IsValid());
            Check.That("the default scene is invalid", !default(Scene).IsValid());
            Check.That("scenes with the same handle are equal", scene.Equals(new Scene("Other", "other.unity", 0, false, 0, 7)));
            Check.That("scenes with different handles differ", scene != new Scene("Level1", "assets/Level1.unity", 3, true, 2, 8));
            Check.Equal("scene ToString is the scene name", "Level1", scene.ToString());
        }

        private static void ProviderResolvesSceneNamesAndBuildIndexes()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                Scene byName = SceneManager.GetSceneByName("Level2");
                Check.Equal("GetSceneByName resolves the path through the provider", "assets/Level2.unity", byName.path);
                Check.Equal("GetSceneByName reports the build index", 1, byName.buildIndex);
                Check.That("GetSceneByName reports an unloaded scene", !byName.isLoaded);

                Scene byIndex = SceneManager.GetSceneByBuildIndex(0);
                Check.Equal("GetSceneByBuildIndex resolves the path through the provider", "assets/Level1.unity", byIndex.path);
                Check.Equal("GetSceneByBuildIndex reports the build index", 0, byIndex.buildIndex);

                Check.That("GetSceneByName for an unknown scene is invalid", !SceneManager.GetSceneByName("Missing").IsValid());
                Check.That("GetSceneByBuildIndex for an unknown index is invalid", !SceneManager.GetSceneByBuildIndex(9).IsValid());
            }
        }

        private static void AsyncOperationCompletesOnTheNextTick()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                AsyncOperation operation = SceneManager.LoadSceneAsync("Level1");
                Check.That("the async operation is not done before the tick", !operation.isDone);
                Check.Equal("the async operation progress is zero before the tick", 0f, operation.progress);

                bool completed = false;
                operation.completed += _ => completed = true;
                Check.That("the completed callback does not fire synchronously", !completed);

                host.Tick(0.016);
                Check.That("the completed callback fires on the tick boundary", completed);
                Check.That("the async operation is done after the tick", operation.isDone);
                Check.Equal("the async operation progress is one after the tick", 1f, operation.progress);
            }
        }

        private static void SceneEventsFireAtTheCallBoundary()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            List<string> events = new List<string>();
            Action<Scene, LoadSceneMode>? onLoaded = null;
            Action<Scene>? onUnloaded = null;
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                onLoaded = (scene, mode) =>
                {
                    events.Add("loaded:" + scene.name + ":" + mode);
                    Check.That(
                        "the load call returned before the sceneLoaded event",
                        backend.Calls.Contains("load:assets/Level1.unity:Additive"));
                };
                onUnloaded = scene =>
                {
                    events.Add("unloaded:" + scene.name);
                    Check.That(
                        "the unload call returned before the sceneUnloaded event",
                        backend.Calls.Contains("unload:Level1"));
                };
                SceneManager.sceneLoaded += onLoaded;
                SceneManager.sceneUnloaded += onUnloaded;

                Scene loaded = SceneManager.LoadScene("Level1", LoadSceneMode.Additive);
                Check.Equal("the loaded scene carries the scene name", "Level1", loaded.name);
                Check.Equal("the loaded scene is reported as loaded", 1, SceneManager.sceneCount);
                Check.That("the loaded scene is found by name", SceneManager.GetSceneByName("Level1").isLoaded);
                Check.Equal("the sceneLoaded event fired once", 1, events.Count);

                SceneManager.UnloadScene("Level1");
                Check.Equal(
                    "the sceneUnloaded event fired after the sceneLoaded event",
                    "loaded:Level1:Additive,unloaded:Level1",
                    string.Join(",", events));
                Check.Equal("no scene is loaded after the unload", 0, SceneManager.sceneCount);

                SceneManager.sceneLoaded -= onLoaded;
                SceneManager.sceneUnloaded -= onUnloaded;
            }
        }

        private static void DontDestroyOnLoadFlagsManagedOnlyObjects()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                GameObject managed = new GameObject("ManagedOnly");
                UnityEngine.Object.DontDestroyOnLoad(managed);
                Check.That("a managed-only object is flagged persistent", managed.IsPersistent);
                Check.Equal("a managed-only object does not reach the backend", 0, backend.DontDestroyOnLoadCalls);
            }
        }

        private static void FailedLoadSurfacesSceneManagementException()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                SceneManager.LoadScene("Level1");
                int countBefore = SceneManager.sceneCount;
                backend.FailLoad = true;

                bool threw = false;
                try
                {
                    SceneManager.LoadScene("Level1");
                }
                catch (SceneManagementException)
                {
                    threw = true;
                }

                Check.That("a failed load surfaces SceneManagementException", threw);
                Check.Equal("a failed load leaves the scene set intact", countBefore, SceneManager.sceneCount);
            }
        }

        private static void NewObjectsTakeTheActiveScene()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                SceneManager.LoadScene("Level1");
                Scene level1 = SceneManager.GetActiveScene();
                Check.That("the first loaded scene is active", level1.IsValid());

                GameObject createdInLevel1 = new GameObject("CreatedInLevel1");
                Check.That("a new object reports the active scene", createdInLevel1.scene == level1);

                SceneManager.LoadScene("Level2", LoadSceneMode.Additive);
                Scene level2 = SceneManager.GetActiveScene();
                Check.That("the second loaded scene is active", level2 != level1);

                GameObject createdInLevel2 = new GameObject("CreatedInLevel2");
                Check.That("a new object reports the new active scene", createdInLevel2.scene == level2);
                Check.That("an object keeps the scene it was created in", createdInLevel1.scene == level1);
            }
        }

        private static void LoadSceneParametersDriveTheLoad()
        {
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                LoadSceneParameters parameters = new LoadSceneParameters
                {
                    loadScene = "Level2",
                    loadSceneMode = LoadSceneMode.Additive,
                    localPhysicsMode = LocalPhysicsMode.PhysicsMode3D,
                };

                Scene overridden = SceneManager.LoadScene("Level1", parameters);
                Check.Equal("the parameters load the scene they name", "Level2", overridden.name);
                Check.That(
                    "the parameters load with their own mode",
                    backend.Calls.Contains("load:assets/Level2.unity:Additive"));

                LoadSceneParameters withoutName = new LoadSceneParameters
                {
                    loadSceneMode = LoadSceneMode.Single,
                };

                Scene requested = SceneManager.LoadScene("Level1", withoutName);
                Check.Equal("an empty loadScene keeps the requested scene", "Level1", requested.name);
                Check.That(
                    "the parameters load with their own mode",
                    backend.Calls.Contains("load:assets/Level1.unity:Single"));
            }
        }
    }
}
