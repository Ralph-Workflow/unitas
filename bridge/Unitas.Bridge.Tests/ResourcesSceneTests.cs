using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public sealed class ResourcePrefabComponent : MonoBehaviour
    {
        public int Power = 42;
    }

    public sealed class CallbackSceneTriggerScript : MonoBehaviour
    {
        public string? LoadSceneOnStart;

        public LoadSceneMode ModeOnStart = LoadSceneMode.Additive;

        public override void Start()
        {
            if (!string.IsNullOrEmpty(LoadSceneOnStart))
            {
                SceneManager.LoadSceneAsync(LoadSceneOnStart!, ModeOnStart);
            }
        }
    }

    public static class ResourcesSceneTests
    {
        private const string AlphaGuid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        private const string BetaGuid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        private const string AlphaFixture = "crates/unitas-runtime/tests/fixtures/scene_alpha.unity";
        private const string BetaFixture = "crates/unitas-runtime/tests/fixtures/scene_beta.unity";
        private const string BetaSceneName = "scene_beta";

        public static void Run()
        {
            TestLoadSceneAsyncHoldAtZeroPointNineUntilAllowSceneActivation();
            TestLoadSceneAsyncActivationOrderingAndSingleLoadDontDestroyOnLoad();
            TestLoadSceneAsyncAdditiveAndSetActiveScene();
            TestQueuedAsyncOperationsWaitBehindHeldActivation();
            TestDeferredSceneOperationsFromCallbacks();
            TestAmbiguousSceneShortNameWarnsAndLoadsFirstMatch();
            TestSceneAbsentFromBuildListAndUnloadOnlySceneRefused();
            TestResourcesLoadTextAssetPrefabInstantiateAndMissingPaths();
            TestResourcesUnloadUnusedAssetsAndUnloadAssetValidation();
            TestSessionStartScansResourcesViaNativeRuntimeEndToEnd();
            TestHeadlessApplicationScreenAndSystemInfoValues();
        }

        private static void TestLoadSceneAsyncHoldAtZeroPointNineUntilAllowSceneActivation()
        {
            SceneManager.ClearBuildScenes();
            FakeSceneProvider provider = new FakeSceneProvider("Assets/Scenes/Level1.unity", "Assets/Scenes/Level2.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                Check.Equal("initial scene is Level1", "Level1", SceneManager.GetActiveScene().name);

                List<string> events = new List<string>();
                Action<Scene, LoadSceneMode> onLoaded = (scene, mode) => events.Add("sceneLoaded:" + scene.name + ":" + mode);
                SceneManager.sceneLoaded += onLoaded;
                try
                {
                    AsyncOperation op = SceneManager.LoadSceneAsync("Level2", LoadSceneMode.Single);
                    op.allowSceneActivation = false;

                    bool completedFired = false;
                    op.completed += _ => completedFired = true;

                    host.Tick(0.016);
                    Check.Equal("progress holds at 0.9f when allowSceneActivation is false", 0.9f, op.progress);
                    Check.That("isDone remains false while allowSceneActivation is false", !op.isDone);
                    Check.That("completed callback has not fired while held", !completedFired);
                    Check.Equal("sceneLoaded does not fire while held at 0.9", 0, events.Count);

                    host.Tick(0.016);
                    Check.Equal("progress still holds at 0.9f on second tick", 0.9f, op.progress);
                    Check.That("isDone still false on second tick", !op.isDone);

                    op.allowSceneActivation = true;
                    host.Tick(0.016);

                    Check.Equal("progress reaches 1.0f after allowSceneActivation is set true", 1.0f, op.progress);
                    Check.That("isDone becomes true after activation", op.isDone);
                    Check.That("completed event fires on activation tick", completedFired);
                    Check.Equal("sceneLoaded fires once on activation", "sceneLoaded:Level2:Single", string.Join(",", events));
                    Check.Equal("active scene becomes Level2 after activation", "Level2", SceneManager.GetActiveScene().name);
                }
                finally
                {
                    SceneManager.sceneLoaded -= onLoaded;
                    SceneManager.ClearBuildScenes();
                }
            }
        }

        private static void TestLoadSceneAsyncActivationOrderingAndSingleLoadDontDestroyOnLoad()
        {
            SceneManager.ClearBuildScenes();
            string alpha = ResolveFixture(AlphaFixture);
            string beta = ResolveFixture(BetaFixture);
            Check.That("alpha fixture exists", File.Exists(alpha));
            Check.That("beta fixture exists", File.Exists(beta));

            ScriptGlueRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptGlueRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptGlueRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                GameObject persistentGo = new GameObject("PersistentAcrossAsyncSingle");
                GameObject transientGo = new GameObject("TransientInAlpha");
                UnityEngine.Object.DontDestroyOnLoad(persistentGo);

                SceneIntegrationEventLog.Clear();
                Action<Scene, LoadSceneMode> onLoaded = (scene, _) =>
                    SceneIntegrationEventLog.Record("sceneLoaded:" + Path.GetFileNameWithoutExtension(scene.path));
                SceneManager.sceneLoaded += onLoaded;

                try
                {
                    AsyncOperation loadBeta = SceneManager.LoadSceneAsync(BetaSceneName, LoadSceneMode.Single, false);
                    Check.Equal("held LoadSceneAsync starts at 0.9f progress", 0.9f, loadBeta.progress);
                    Check.That("held LoadSceneAsync is not done", !loadBeta.isDone);
                    Check.Equal("beta Awake has not run while held", 0, betaScript.AwakeCount);
                    Check.Equal("beta OnEnable has not run while held", 0, betaScript.EnableCount);
                    Check.Equal("beta Start has not run while held", 0, betaScript.StartCount);

                    host.Tick(0.016);
                    Check.Equal("beta Awake still 0 after tick while held", 0, betaScript.AwakeCount);
                    Check.Equal("alpha scene script not destroyed while beta is held", 0, alphaScript.DestroyCount);

                    loadBeta.allowSceneActivation = true;
                    host.Tick(0.016);

                    Check.That("loadBeta is done after activation tick", loadBeta.isDone);
                    Check.Equal("loadBeta progress is 1.0f after activation tick", 1.0f, loadBeta.progress);
                    Check.That("previous alpha script is destroyed on single load activation", alphaScript.DestroyCount > 0);

                    host.Tick(0.016);
                    host.Tick(0.016);

                    Check.Equal("beta Awake ran once", 1, betaScript.AwakeCount);
                    Check.Equal("beta OnEnable ran once", 1, betaScript.EnableCount);
                    Check.Equal("beta Start ran once", 1, betaScript.StartCount);
                    Check.That("beta Update ran after Start", betaScript.UpdateCount > 0);

                    int awakeIdx = SceneIntegrationEventLog.IndexOf("beta:Awake");
                    int enableIdx = SceneIntegrationEventLog.IndexOf("beta:OnEnable");
                    int startIdx = SceneIntegrationEventLog.IndexOf("beta:Start");
                    Check.That("Awake precedes OnEnable on activated scene", awakeIdx >= 0 && enableIdx > awakeIdx);
                    Check.That("OnEnable precedes Start on activated scene", enableIdx >= 0 && startIdx > enableIdx);
                    Check.That("sceneLoaded fired for beta scene", SceneIntegrationEventLog.IndexOf("sceneLoaded:scene_beta") >= 0);

                    GameObject? foundPersistent = GameObject.Find("PersistentAcrossAsyncSingle");
                    GameObject? foundTransient = GameObject.Find("TransientInAlpha");
                    Check.That("DontDestroyOnLoad object survives single async load", ReferenceEquals(foundPersistent, persistentGo));
                    Check.That("non-persistent object from previous scene is removed", foundTransient == null);
                }
                finally
                {
                    SceneManager.sceneLoaded -= onLoaded;
                    UnityEngine.Object.Destroy(persistentGo);
                    UnityEngine.Object.Destroy(transientGo);
                    SceneManager.ClearBuildScenes();
                }
            }
        }

        private static void TestLoadSceneAsyncAdditiveAndSetActiveScene()
        {
            SceneManager.ClearBuildScenes();
            string alpha = ResolveFixture(AlphaFixture);

            ScriptGlueRegistry.Clear();
            AlphaSceneRecorder alphaScript = new AlphaSceneRecorder();
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptGlueRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptGlueRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                AsyncOperation additiveOp = SceneManager.LoadSceneAsync(BetaSceneName, LoadSceneMode.Additive, false);
                host.Tick(0.016);
                Check.Equal("only 1 scene loaded while additive load is held", 1, SceneManager.sceneCount);

                additiveOp.allowSceneActivation = true;
                host.Tick(0.016);
                host.Tick(0.016);

                Check.Equal("both scenes are loaded after additive activation", 2, SceneManager.sceneCount);
                Scene scene0 = SceneManager.GetSceneAt(0);
                Scene scene1 = SceneManager.GetSceneAt(1);
                Check.That("scene 0 is valid and loaded", scene0.IsValid() && scene0.isLoaded);
                Check.That("scene 1 is valid and loaded", scene1.IsValid() && scene1.isLoaded);

                List<string> activeChanges = new List<string>();
                Action<Scene, Scene> onActiveChanged = (prev, next) =>
                    activeChanges.Add(prev.name + "->" + next.name);
                SceneManager.activeSceneChanged += onActiveChanged;
                try
                {
                    Check.That("SetActiveScene to scene1 succeeds", SceneManager.SetActiveScene(scene1));
                    Check.Equal("active scene is now scene1", scene1.name, SceneManager.GetActiveScene().name);
                    Check.That("activeSceneChanged event fired", activeChanges.Count == 1);
                }
                finally
                {
                    SceneManager.activeSceneChanged -= onActiveChanged;
                    SceneManager.ClearBuildScenes();
                }
            }
        }

        private static void TestQueuedAsyncOperationsWaitBehindHeldActivation()
        {
            SceneManager.ClearBuildScenes();
            FakeSceneProvider provider = new FakeSceneProvider("Assets/Level1.unity", "Assets/Level2.unity", "Assets/Level3.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;

                SceneManager.LoadScene("Level1", LoadSceneMode.Single);

                AsyncOperation first = SceneManager.LoadSceneAsync("Level2", LoadSceneMode.Additive, false);
                AsyncOperation second = SceneManager.LoadSceneAsync("Level3", LoadSceneMode.Additive, true);

                host.Tick(0.016);
                Check.That("first held operation is not done", !first.isDone);
                Check.That("second operation queued behind held operation also waits", !second.isDone);
                Check.Equal("only Level1 is loaded while first operation is held", 1, SceneManager.sceneCount);

                first.allowSceneActivation = true;
                host.Tick(0.016);

                Check.That("first operation completes once released", first.isDone);
                Check.That("second operation completes after first releases", second.isDone);
                Check.Equal("all three scenes are now loaded", 3, SceneManager.sceneCount);
                SceneManager.ClearBuildScenes();
            }
        }

        private static void TestDeferredSceneOperationsFromCallbacks()
        {
            SceneManager.ClearBuildScenes();
            string alpha = ResolveFixture(AlphaFixture);

            ScriptGlueRegistry.Clear();
            CallbackSceneTriggerScript triggerScript = new CallbackSceneTriggerScript
            {
                LoadSceneOnStart = BetaSceneName,
                ModeOnStart = LoadSceneMode.Additive,
            };
            BetaSceneRecorder betaScript = new BetaSceneRecorder();
            ScriptGlueRegistry.Register(AlphaGuid, () => triggerScript);
            ScriptGlueRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                Check.Equal("deferred LoadSceneAsync from Start callback activates on tick end", 2, SceneManager.sceneCount);
                host.Tick(0.016);
                host.Tick(0.016);
                Check.Equal("additively loaded scene script from Start callback initialized and ticked", 1, betaScript.AwakeCount);
                Check.That("additively loaded scene script updates", betaScript.UpdateCount > 0);
                SceneManager.ClearBuildScenes();
            }
        }

        private static void TestAmbiguousSceneShortNameWarnsAndLoadsFirstMatch()
        {
            SceneManager.ClearBuildScenes();
            LogRecorder.Clear();
            FakeSceneProvider provider = new FakeSceneProvider(
                "Assets/WorldA/Arena.unity",
                "Assets/WorldB/Arena.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes(
                    "Assets/WorldA/Arena.unity",
                    "Assets/WorldB/Arena.unity");

                Scene loaded = SceneManager.LoadScene("Arena", LoadSceneMode.Single);
                Check.Equal("ambiguous short name loads first matching scene", "Assets/WorldA/Arena.unity", loaded.path);

                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                Check.Equal("ambiguous short name logs one warning", 1, warnings.Count);
                if (warnings.Count > 0)
                {
                    Check.That(
                        "warning lists first matching path",
                        warnings[0].Message.Contains("Assets/WorldA/Arena.unity", StringComparison.Ordinal));
                    Check.That(
                        "warning lists second matching path",
                        warnings[0].Message.Contains("Assets/WorldB/Arena.unity", StringComparison.Ordinal));
                }

                SceneManager.ClearBuildScenes();
                LogRecorder.Clear();
            }
        }

        private static void TestSceneAbsentFromBuildListAndUnloadOnlySceneRefused()
        {
            SceneManager.ClearBuildScenes();
            LogRecorder.Clear();
            FakeSceneProvider provider = new FakeSceneProvider("Assets/Scenes/Included.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes("Assets/Scenes/Included.unity");
                SceneManager.EnforceBuildList = true;

                Scene included = SceneManager.LoadScene("Included", LoadSceneMode.Single);
                Check.That("included scene is valid", included.IsValid());
                Check.Equal("one scene is loaded", 1, SceneManager.sceneCount);

                bool missingThrew = false;
                try
                {
                    SceneManager.LoadScene("NotInBuildList", LoadSceneMode.Single);
                }
                catch (SceneManagementException)
                {
                    missingThrew = true;
                }

                Check.That("loading scene absent from build list throws SceneManagementException", missingThrew);
                Check.Equal("failed load of absent scene leaves loaded scenes unchanged", 1, SceneManager.sceneCount);
                Check.Equal("active scene remains Included", "Included", SceneManager.GetActiveScene().name);
                Check.That("absent scene logs an error", LogRecorder.OfSeverity(LogSeverity.Error).Count >= 1);

                LogRecorder.Clear();
                bool unloadOnlyThrew = false;
                try
                {
                    SceneManager.UnloadSceneAsync("Included");
                }
                catch (SceneManagementException)
                {
                    unloadOnlyThrew = true;
                }

                Check.That("unloading the only loaded scene is refused", unloadOnlyThrew);
                Check.Equal("refused unload leaves scene count at 1", 1, SceneManager.sceneCount);
                Check.That("refused unload logs an error", LogRecorder.OfSeverity(LogSeverity.Error).Count >= 1);

                SceneManager.ClearBuildScenes();
                LogRecorder.Clear();
            }
        }

        private static void TestResourcesLoadTextAssetPrefabInstantiateAndMissingPaths()
        {
            Resources.Clear();
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas_res_test_" + Guid.NewGuid().ToString("N"));
            string resConfigDir = Path.Combine(tempRoot, "Assets", "Resources", "Config");
            string resNestedDir = Path.Combine(tempRoot, "Assets", "Gameplay", "Resources", "Prefabs");
            Directory.CreateDirectory(resConfigDir);
            Directory.CreateDirectory(resNestedDir);

            string jsonPath = Path.Combine(resConfigDir, "settings.json");
            string metaPath = Path.Combine(resConfigDir, "settings.json.meta");
            string prefabFilePath = Path.Combine(resNestedDir, "DiskEnemy.prefab");
            File.WriteAllText(jsonPath, "{\"difficulty\":\"hard\"}");
            File.WriteAllText(metaPath, "fileFormatVersion: 2\nguid: aabbccddeeff00112233445566778899\n");
            File.WriteAllText(prefabFilePath, "%YAML 1.1\n--- !u!1 &1\nGameObject:\n  m_Name: DiskEnemy\n");

            List<UnityEngine.Object> cleanup = new List<UnityEngine.Object>();
            try
            {
                Resources.AddSearchRoot(Path.Combine(tempRoot, "Assets"));

                TextAsset? loadedText = Resources.Load<TextAsset>("Config/settings");
                Check.That("Resources.Load<TextAsset> resolves file under Assets/Resources", loadedText != null);
                if (loadedText != null)
                {
                    Check.Equal("TextAsset name is file stem", "settings", loadedText.name);
                    Check.Equal("TextAsset.text matches file content", "{\"difficulty\":\"hard\"}", loadedText.text);
                    Check.Equal(
                        "TextAsset.bytes matches UTF-8 bytes",
                        Encoding.UTF8.GetByteCount("{\"difficulty\":\"hard\"}"),
                        loadedText.bytes.Length);
                }

                UnityEngine.Object? untypedText = Resources.Load("Config/settings");
                Check.That("untyped Resources.Load returns cached TextAsset instance", ReferenceEquals(untypedText, loadedText));

                GameObject? mismatch = Resources.Load<GameObject>("Config/settings");
                Check.That("type mismatch on Resources.Load returns null without throwing", mismatch == null);

                UnityEngine.Object? missingObj = Resources.Load("No/Such/Resource");
                TextAsset? missingTyped = Resources.Load<TextAsset>("No/Such/Resource");
                Check.That("Resources.Load on missing path returns null without exception", missingObj == null);
                Check.That("Resources.Load<T> on missing path returns null without exception", missingTyped == null);

                ResourceRequest asyncTextReq = Resources.LoadAsync<TextAsset>("Config/settings");
                Check.That("Resources.LoadAsync completes and sets isDone true", asyncTextReq.isDone);
                Check.Equal("Resources.LoadAsync progress is 1.0f", 1.0f, asyncTextReq.progress);
                Check.That("Resources.LoadAsync.asset matches loaded TextAsset", ReferenceEquals(asyncTextReq.asset, loadedText));

                ResourceRequest asyncMissingReq = Resources.LoadAsync<TextAsset>("Missing/Async/Resource");
                Check.That("Resources.LoadAsync on missing path completes cleanly", asyncMissingReq.isDone);
                Check.That("Resources.LoadAsync on missing path has null asset", asyncMissingReq.asset == null);

                GameObject? diskPrefab = Resources.Load<GameObject>("Prefabs/DiskEnemy");
                Check.That("Resources.Load<GameObject> resolves .prefab under nested Resources folder", diskPrefab != null);
                if (diskPrefab != null)
                {
                    cleanup.Add(diskPrefab);
                    Check.Equal("loaded disk prefab name", "DiskEnemy", diskPrefab.name);
                }

                GameObject registeredPrefab = new GameObject("HeroPrefab");
                cleanup.Add(registeredPrefab);
                ResourcePrefabComponent heroComp = registeredPrefab.AddComponent<ResourcePrefabComponent>();
                heroComp.Power = 99;
                Resources.Register("Prefabs/Hero", registeredPrefab);

                GameObject? loadedHeroPrefab = Resources.Load<GameObject>("Prefabs/Hero");
                Check.That("Resources.Load<GameObject> finds registered prefab", ReferenceEquals(loadedHeroPrefab, registeredPrefab));

                ResourcePrefabComponent? loadedHeroComp = Resources.Load<ResourcePrefabComponent>("Prefabs/Hero");
                Check.That("Resources.Load<TComponent> extracts component from prefab GameObject", ReferenceEquals(loadedHeroComp, heroComp));

                if (loadedHeroPrefab != null)
                {
                    GameObject instance = UnityEngine.Object.Instantiate(loadedHeroPrefab);
                    cleanup.Add(instance);
                    Check.Equal("instantiated prefab clone has (Clone) suffix", "HeroPrefab (Clone)", instance.name);
                    ResourcePrefabComponent? clonedComp = instance.GetComponent<ResourcePrefabComponent>();
                    Check.That("instantiated prefab clone carries cloned component", clonedComp != null);
                    if (clonedComp != null)
                    {
                        Check.Equal("cloned component preserves field value", 99, clonedComp.Power);
                    }
                }

                TextAsset[] allConfigs = Resources.LoadAll<TextAsset>("Config");
                Check.Equal("Resources.LoadAll<TextAsset> finds 1 asset in Config folder and ignores .meta", 1, allConfigs.Length);
            }
            finally
            {
                for (int i = 0; i < cleanup.Count; i++)
                {
                    UnityEngine.Object.Destroy(cleanup[i]);
                }

                Resources.Clear();
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void TestResourcesUnloadUnusedAssetsAndUnloadAssetValidation()
        {
            Resources.Clear();
            TextAsset transientAsset = new TextAsset("ephemeral");
            Resources.Register("Temp/ephemeral", transientAsset);
            Check.Equal("resident asset count is 1 after register", 1, Resources.ResidentAssetCount);

            Resources.UnloadAsset(transientAsset);
            AsyncOperation unloadOp = Resources.UnloadUnusedAssets();
            Check.That("UnloadUnusedAssets returns completed AsyncOperation", unloadOp.isDone);
            Check.Equal("resident asset count drops to 0 after unload", 0, Resources.ResidentAssetCount);

            GameObject go = new GameObject("CannotUnloadViaResources");
            try
            {
                bool threwOnGo = false;
                try
                {
                    Resources.UnloadAsset(go);
                }
                catch (ArgumentException)
                {
                    threwOnGo = true;
                }

                Check.That("Resources.UnloadAsset rejects GameObject with ArgumentException", threwOnGo);
            }
            finally
            {
                UnityEngine.Object.Destroy(go);
                Resources.Clear();
            }
        }

        private static void TestSessionStartScansResourcesViaNativeRuntimeEndToEnd()
        {
            Resources.Clear();
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas_res_session_e2e_" + Guid.NewGuid().ToString("N"));
            string projectSettingsDir = Path.Combine(tempRoot, "ProjectSettings");
            string resConfigDir = Path.Combine(tempRoot, "Assets", "Resources", "Config");
            string resNestedDir = Path.Combine(tempRoot, "Assets", "Gameplay", "Resources", "Nested");
            string scenesDir = Path.Combine(tempRoot, "Assets", "Scenes");
            Directory.CreateDirectory(projectSettingsDir);
            Directory.CreateDirectory(resConfigDir);
            Directory.CreateDirectory(resNestedDir);
            Directory.CreateDirectory(scenesDir);

            string sessionItemPath = Path.Combine(resConfigDir, "session_item.txt");
            string sessionItemMetaPath = Path.Combine(resConfigDir, "session_item.txt.meta");
            string nestedNotePath = Path.Combine(resNestedDir, "note.txt");
            string scenePath = Path.Combine(scenesDir, "Main.unity");

            File.WriteAllText(sessionItemPath, "session_payload_v1");
            File.WriteAllText(sessionItemMetaPath, "fileFormatVersion: 2\nguid: 11223344556677889900aabbccddeeff\n");
            File.WriteAllText(nestedNotePath, "nested_resource_note");
            File.WriteAllText(
                scenePath,
                "%YAML 1.1\n"
                + "%TAG !u! tag:unity3d.com,2011:\n"
                + "--- !u!1 &100\n"
                + "GameObject:\n"
                + "  m_Name: Root\n"
                + "  m_IsActive: 1\n"
                + "--- !u!4 &101\n"
                + "Transform:\n"
                + "  m_GameObject: {fileID: 100}\n"
                + "  m_LocalPosition: {x: 0, y: 0, z: 0}\n"
                + "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n"
                + "  m_LocalScale: {x: 1, y: 1, z: 1}\n");

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);

                    TextAsset? configItem = Resources.Load<TextAsset>("Config/session_item");
                    Check.That("headless session Resources.Load<TextAsset> resolves Assets/Resources item without manual AddSearchRoot", configItem != null);
                    if (configItem != null)
                    {
                        Check.Equal("session TextAsset name matches stem", "session_item", configItem.name);
                        Check.Equal("session TextAsset text matches file content", "session_payload_v1", configItem.text);
                    }

                    TextAsset? nestedNote = Resources.Load<TextAsset>("Nested/note");
                    Check.That("headless session Resources.Load<TextAsset> resolves nested Assets/Gameplay/Resources item", nestedNote != null);
                    if (nestedNote != null)
                    {
                        Check.Equal("nested TextAsset name matches stem", "note", nestedNote.name);
                        Check.Equal("nested TextAsset text matches file content", "nested_resource_note", nestedNote.text);
                    }

                    UnityEngine.Object? missingUntyped = Resources.Load("Missing/DoesNotExist");
                    TextAsset? missingTyped = Resources.Load<TextAsset>("Missing/DoesNotExist");
                    Check.That("headless session Resources.Load on missing path returns null without throwing", missingUntyped == null);
                    Check.That("headless session Resources.Load<TextAsset> on missing path returns null without throwing", missingTyped == null);

                    Check.Equal("resident asset count reflects two loaded session resources", 2, Resources.ResidentAssetCount);

                    if (configItem != null)
                    {
                        Resources.UnloadAsset(configItem);
                    }

                    AsyncOperation firstUnload = Resources.UnloadUnusedAssets();
                    Check.That("UnloadUnusedAssets completes after first UnloadAsset", firstUnload.isDone);
                    Check.Equal("resident asset count drops to 1 after unloading first asset", 1, Resources.ResidentAssetCount);

                    if (nestedNote != null)
                    {
                        Resources.UnloadAsset(nestedNote);
                    }

                    AsyncOperation secondUnload = Resources.UnloadUnusedAssets();
                    Check.That("UnloadUnusedAssets completes after second UnloadAsset", secondUnload.isDone);
                    Check.Equal("resident asset count drops to 0 after unloading remaining asset", 0, Resources.ResidentAssetCount);
                }
            }
            finally
            {
                Resources.Clear();
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void TestHeadlessApplicationScreenAndSystemInfoValues()
        {
            Screen.ResetToDefaults();
            Check.That("headless Screen.width is positive and non-zero", Screen.width > 0);
            Check.That("headless Screen.height is positive and non-zero", Screen.height > 0);
            Check.Equal("default headless Screen.width is 1280", 1280, Screen.width);
            Check.Equal("default headless Screen.height is 720", 720, Screen.height);
            Check.Equal("Screen.currentResolution.width matches Screen.width", Screen.width, Screen.currentResolution.width);
            Check.Equal("Screen.currentResolution.height matches Screen.height", Screen.height, Screen.currentResolution.height);

            Screen.SetResolution(1920, 1080, true, 120);
            Check.Equal("Screen.SetResolution updates width", 1920, Screen.width);
            Check.Equal("Screen.SetResolution updates height", 1080, Screen.height);
            Check.That("Screen.SetResolution updates fullScreen", Screen.fullScreen);
            Check.Equal("Screen.SetResolution updates refreshRate", 120, Screen.currentResolution.refreshRate);
            Screen.ResetToDefaults();

            Check.Equal(
                "headless SystemInfo.graphicsDeviceType is GraphicsDeviceType.Null",
                (int)GraphicsDeviceType.Null,
                (int)SystemInfo.graphicsDeviceType);
            Check.That("SystemInfo.processorCount is at least 1", SystemInfo.processorCount >= 1);

            Check.Equal("Application.unityVersion reports 2022.3.0f1", "2022.3.0f1", Application.unityVersion);

            RuntimePlatform expectedEditorPlatform = RuntimePlatform.LinuxEditor;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                expectedEditorPlatform = RuntimePlatform.OSXEditor;
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                expectedEditorPlatform = RuntimePlatform.WindowsEditor;
            }

            using (BridgeHost host = new BridgeHost())
            {
                Check.Equal(
                    "Application.platform reports host editor platform in headless editor session",
                    (int)expectedEditorPlatform,
                    (int)Application.platform);

                Application.IgnoreQuitInPlayMode = true;
                try
                {
                    Application.Quit();
                    Check.That("Application.Quit is ignored during editor play mode when IgnoreQuitInPlayMode is enabled", Application.isPlaying);
                    Check.That("QuitRequestCount records the ignored quit call", Application.QuitRequestCount >= 1);
                }
                finally
                {
                    Application.IgnoreQuitInPlayMode = false;
                }
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
