using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public sealed class SceneAcceptanceOrderedScript : MonoBehaviour
    {
        public string Label = string.Empty;

        public bool MarkPersistentOnAwake;

        public int AwakeCount;

        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public int DisableCount;

        public int DestroyCount;

        public int InitialInstanceId;

        public override void Awake()
        {
            AwakeCount++;
            InitialInstanceId = gameObject.GetInstanceID();
            if (MarkPersistentOnAwake)
            {
                UnityEngine.Object.DontDestroyOnLoad(gameObject);
            }

            SceneIntegrationEventLog.Record(Label + ":Awake");
        }

        public override void OnEnable()
        {
            EnableCount++;
            SceneIntegrationEventLog.Record(Label + ":OnEnable");
        }

        public override void Start()
        {
            StartCount++;
            SceneIntegrationEventLog.Record(Label + ":Start");
        }

        public override void Update()
        {
            UpdateCount++;
        }

        public override void OnDisable()
        {
            DisableCount++;
            SceneIntegrationEventLog.Record(Label + ":OnDisable");
        }

        public override void OnDestroy()
        {
            DestroyCount++;
            SceneIntegrationEventLog.Record(Label + ":OnDestroy");
        }
    }

    public static class SceneAcceptanceTests
    {
        private const string AlphaGuid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        private const string BetaGuid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        private const string AlphaFixture = "crates/unitas-runtime/tests/fixtures/scene_alpha.unity";
        private const string BetaFixture = "crates/unitas-runtime/tests/fixtures/scene_beta.unity";
        private const string BetaSceneName = "scene_beta";

        public static void Run()
        {
            TestStory17LoadSceneAsyncAllowSceneActivationCallbackOrderAndDontDestroyOnLoad();
            TestStory17DontDestroyOnLoadSurvivesSingleModeWithoutReRunningAwakeOrStart();
            TestStory18AmbiguousSceneShortNameWarnsAndLoadsFirstMatchInBuildList();
            TestStory18LoadingSceneAbsentFromBuildListLogsErrorAndPreservesActiveScene();
            TestStory18RefusingToUnloadOnlyLoadedScene();
        }

        private static void TestStory17LoadSceneAsyncAllowSceneActivationCallbackOrderAndDontDestroyOnLoad()
        {
            SceneManager.ClearBuildScenes();
            string alpha = ResolveFixture(AlphaFixture);
            string beta = ResolveFixture(BetaFixture);
            Check.That("Story 17 alpha scene fixture exists", File.Exists(alpha));
            Check.That("Story 17 beta scene fixture exists", File.Exists(beta));

            ScriptGlueRegistry.Clear();
            SceneAcceptanceOrderedScript alphaScript = new SceneAcceptanceOrderedScript
            {
                Label = "alpha",
            };
            SceneAcceptanceOrderedScript betaScript = new SceneAcceptanceOrderedScript
            {
                Label = "beta",
            };
            ScriptGlueRegistry.Register(AlphaGuid, () => alphaScript);
            ScriptGlueRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                Check.Equal("Story 17 initial active scene is scene_alpha", "scene_alpha", Path.GetFileNameWithoutExtension(SceneManager.GetActiveScene().path));
                Check.Equal("Story 17 alpha Awake ran once", 1, alphaScript.AwakeCount);
                Check.Equal("Story 17 alpha OnEnable ran once", 1, alphaScript.EnableCount);
                Check.Equal("Story 17 alpha Start ran once", 1, alphaScript.StartCount);

                GameObject persistentObject = new GameObject("Story17_PersistentSurvivor");
                GameObject transientObject = new GameObject("Story17_TransientVictim");
                UnityEngine.Object.DontDestroyOnLoad(persistentObject);

                SceneIntegrationEventLog.Clear();
                Action<Scene, LoadSceneMode> onSceneLoaded = (scene, mode) =>
                {
                    SceneIntegrationEventLog.Record("sceneLoaded:" + Path.GetFileNameWithoutExtension(scene.path) + ":" + mode);
                };
                Action<Scene, Scene> onActiveSceneChanged = (prev, next) =>
                {
                    SceneIntegrationEventLog.Record(
                        "activeSceneChanged:"
                        + Path.GetFileNameWithoutExtension(prev.path)
                        + "->"
                        + Path.GetFileNameWithoutExtension(next.path));
                };
                Action<AsyncOperation> onCompleted = _ =>
                {
                    SceneIntegrationEventLog.Record("asyncOp:completed");
                };

                SceneManager.sceneLoaded += onSceneLoaded;
                SceneManager.activeSceneChanged += onActiveSceneChanged;

                try
                {
                    AsyncOperation op = SceneManager.LoadSceneAsync(BetaSceneName, LoadSceneMode.Single, false);
                    op.completed += onCompleted;

                    Check.Equal("Story 17 LoadSceneAsync with allowSceneActivation=false holds progress at 0.9f", 0.9f, op.progress);
                    Check.That("Story 17 LoadSceneAsync isDone is false while held", !op.isDone);

                    host.Tick(0.016);
                    host.Tick(0.016);

                    Check.Equal("Story 17 held operation remains at 0.9f across ticks", 0.9f, op.progress);
                    Check.That("Story 17 held operation remains not done across ticks", !op.isDone);
                    Check.Equal("Story 17 beta Awake has not run while held at 0.9", 0, betaScript.AwakeCount);
                    Check.Equal("Story 17 beta OnEnable has not run while held at 0.9", 0, betaScript.EnableCount);
                    Check.Equal("Story 17 beta Start has not run while held at 0.9", 0, betaScript.StartCount);
                    Check.Equal("Story 17 alpha script is not destroyed while beta is held", 0, alphaScript.DestroyCount);
                    Check.Equal("Story 17 active scene remains scene_alpha while beta is held", "scene_alpha", Path.GetFileNameWithoutExtension(SceneManager.GetActiveScene().path));

                    op.allowSceneActivation = true;
                    host.Tick(0.016);

                    Check.That("Story 17 operation isDone becomes true after allowSceneActivation=true", op.isDone);
                    Check.Equal("Story 17 operation progress reaches 1.0f after activation", 1.0f, op.progress);
                    Check.Equal("Story 17 alpha script is torn down on single load activation", 1, alphaScript.DestroyCount);
                    Check.Equal("Story 17 active scene becomes scene_beta", "scene_beta", Path.GetFileNameWithoutExtension(SceneManager.GetActiveScene().path));

                    host.Tick(0.016);
                    host.Tick(0.016);

                    Check.Equal("Story 17 beta Awake ran once after activation", 1, betaScript.AwakeCount);
                    Check.Equal("Story 17 beta OnEnable ran once after activation", 1, betaScript.EnableCount);
                    Check.Equal("Story 17 beta Start ran once after activation", 1, betaScript.StartCount);
                    Check.That("Story 17 beta Update ran after Start", betaScript.UpdateCount >= 1);

                    int awakeIdx = SceneIntegrationEventLog.IndexOf("beta:Awake");
                    int enableIdx = SceneIntegrationEventLog.IndexOf("beta:OnEnable");
                    int loadedIdx = SceneIntegrationEventLog.IndexOf("sceneLoaded:scene_beta:Single");
                    int activeChangedIdx = SceneIntegrationEventLog.IndexOf("activeSceneChanged:scene_alpha->scene_beta");
                    int startIdx = SceneIntegrationEventLog.IndexOf("beta:Start");
                    int completedIdx = SceneIntegrationEventLog.IndexOf("asyncOp:completed");

                    Check.That("Story 17 beta Awake precedes beta OnEnable", awakeIdx >= 0 && enableIdx > awakeIdx);
                    Check.That("Story 17 beta OnEnable precedes beta Start", enableIdx >= 0 && startIdx > enableIdx);
                    Check.That("Story 17 sceneLoaded fired for scene_beta", loadedIdx >= 0);
                    Check.That("Story 17 activeSceneChanged fired after sceneLoaded", activeChangedIdx > loadedIdx);
                    Check.That("Story 17 AsyncOperation.completed fired on activation", completedIdx > activeChangedIdx);

                    SceneIntegrationEventLog.Clear();
                    FakeSceneProvider orderProvider = new FakeSceneProvider("Assets/Scenes/OrderA.unity", "Assets/Scenes/OrderB.unity");
                    FakeSceneBackend orderBackend = new FakeSceneBackend();
                    host.Provider = orderProvider;
                    host.Backend = orderBackend;

                    SceneManager.LoadScene("OrderA", LoadSceneMode.Single);
                    SceneIntegrationEventLog.Clear();

                    SceneAcceptanceOrderedScript stagedScript = new SceneAcceptanceOrderedScript
                    {
                        Label = "orderB",
                    };
                    GameObject stagedGo = new GameObject("StagedOrderBObject");
                    stagedGo.AttachComponent(stagedScript);

                    AsyncOperation stagedOp = SceneManager.LoadSceneAsync("OrderB", LoadSceneMode.Single, false);
                    Check.Equal("Story 17 staged async op holds at 0.9f", 0.9f, stagedOp.progress);

                    stagedOp.allowSceneActivation = true;
                    stagedScript.Awake();
                    stagedScript.OnEnable();
                    SceneManager.FlushDeferredOperations();
                    stagedScript.Start();

                    int stagedAwake = SceneIntegrationEventLog.IndexOf("orderB:Awake");
                    int stagedEnable = SceneIntegrationEventLog.IndexOf("orderB:OnEnable");
                    int stagedLoaded = SceneIntegrationEventLog.IndexOf("sceneLoaded:OrderB:Single");
                    int stagedActive = SceneIntegrationEventLog.IndexOf("activeSceneChanged:OrderA->OrderB");
                    int stagedStart = SceneIntegrationEventLog.IndexOf("orderB:Start");

                    Check.That(
                        "Story 17 activation callback sequence is Awake -> OnEnable -> sceneLoaded -> activeSceneChanged -> Start",
                        stagedAwake >= 0
                        && stagedEnable > stagedAwake
                        && stagedLoaded > stagedEnable
                        && stagedActive > stagedLoaded
                        && stagedStart > stagedActive);

                    UnityEngine.Object.Destroy(stagedGo);

                    Check.That(
                        "Story 17 DontDestroyOnLoad object survives single-mode LoadSceneAsync",
                        ReferenceEquals(GameObject.Find("Story17_PersistentSurvivor"), persistentObject));
                    Check.That(
                        "Story 17 non-persistent object from replaced scene is removed",
                        GameObject.Find("Story17_TransientVictim") == null);
                }
                finally
                {
                    SceneManager.sceneLoaded -= onSceneLoaded;
                    SceneManager.activeSceneChanged -= onActiveSceneChanged;
                    UnityEngine.Object.Destroy(persistentObject);
                    UnityEngine.Object.Destroy(transientObject);
                    SceneManager.ClearBuildScenes();
                }
            }
        }

        private static void TestStory17DontDestroyOnLoadSurvivesSingleModeWithoutReRunningAwakeOrStart()
        {
            SceneManager.ClearBuildScenes();
            string alpha = ResolveFixture(AlphaFixture);
            string beta = ResolveFixture(BetaFixture);

            ScriptGlueRegistry.Clear();
            SceneAcceptanceOrderedScript persistentScript = new SceneAcceptanceOrderedScript
            {
                Label = "persistentAlpha",
                MarkPersistentOnAwake = true,
            };
            SceneAcceptanceOrderedScript betaScript = new SceneAcceptanceOrderedScript
            {
                Label = "betaReplacement",
            };
            ScriptGlueRegistry.Register(AlphaGuid, () => persistentScript);
            ScriptGlueRegistry.Register(BetaGuid, () => betaScript);

            using (BridgeHost host = CreateHost(alpha))
            {
                host.LoadScene(alpha, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                Check.Equal("Story 17 persistent script Awake ran once in initial scene", 1, persistentScript.AwakeCount);
                Check.Equal("Story 17 persistent script OnEnable ran once in initial scene", 1, persistentScript.EnableCount);
                Check.Equal("Story 17 persistent script Start ran once in initial scene", 1, persistentScript.StartCount);
                Check.That("Story 17 persistent script ticked in initial scene", persistentScript.UpdateCount >= 1);
                int updatesBeforeSingleLoad = persistentScript.UpdateCount;
                int originalInstanceId = persistentScript.InitialInstanceId;

                host.LoadScene(beta, LoadSceneMode.Single);
                host.Tick(0.016);
                host.Tick(0.016);

                Check.Equal("Story 17 persistent script Awake does not re-run across single-mode scene load", 1, persistentScript.AwakeCount);
                Check.Equal("Story 17 persistent script OnEnable does not re-run across single-mode scene load", 1, persistentScript.EnableCount);
                Check.Equal("Story 17 persistent script Start does not re-run across single-mode scene load", 1, persistentScript.StartCount);
                Check.Equal("Story 17 persistent script OnDisable is not called on single-mode scene load", 0, persistentScript.DisableCount);
                Check.Equal("Story 17 persistent script OnDestroy is not called on single-mode scene load", 0, persistentScript.DestroyCount);
                Check.Equal("Story 17 persistent GameObject retains its instance ID across single-mode scene load", originalInstanceId, persistentScript.gameObject.GetInstanceID());
                Check.That("Story 17 persistent script continues receiving Update after single-mode scene load", persistentScript.UpdateCount > updatesBeforeSingleLoad);
                Check.Equal("Story 17 newly loaded beta script ran Awake and Start once", 1, betaScript.AwakeCount);
                Check.Equal("Story 17 newly loaded beta script Start count is 1", 1, betaScript.StartCount);
            }
        }

        private static void TestStory18AmbiguousSceneShortNameWarnsAndLoadsFirstMatchInBuildList()
        {
            SceneManager.ClearBuildScenes();
            LogRecorder.Clear();
            FakeSceneProvider provider = new FakeSceneProvider(
                "Assets/Regions/North/Dungeon.unity",
                "Assets/Regions/South/Dungeon.unity",
                "Assets/Regions/East/Boss.unity");
            FakeSceneBackend backend = new FakeSceneBackend();

            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes(
                    "Assets/Regions/North/Dungeon.unity",
                    "Assets/Regions/South/Dungeon.unity",
                    "Assets/Regions/East/Boss.unity");

                try
                {
                    Scene loaded = SceneManager.LoadScene("Dungeon", LoadSceneMode.Single);
                    Check.Equal(
                        "Story 18 ambiguous short name loads first matching scene in build list",
                        "Assets/Regions/North/Dungeon.unity",
                        loaded.path);
                    Check.Equal("Story 18 loaded scene name is Dungeon", "Dungeon", loaded.name);

                    IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                    Check.Equal("Story 18 ambiguous short name logs exactly one warning", 1, warnings.Count);
                    if (warnings.Count == 1)
                    {
                        string msg = warnings[0].Message;
                        Check.That(
                            "Story 18 warning lists first match Assets/Regions/North/Dungeon.unity",
                            msg.Contains("Assets/Regions/North/Dungeon.unity", StringComparison.Ordinal));
                        Check.That(
                            "Story 18 warning lists second match Assets/Regions/South/Dungeon.unity",
                            msg.Contains("Assets/Regions/South/Dungeon.unity", StringComparison.Ordinal));
                    }

                    LogRecorder.Clear();
                    Scene disambiguated = SceneManager.LoadScene("Assets/Regions/South/Dungeon.unity", LoadSceneMode.Single);
                    Check.Equal(
                        "Story 18 full path disambiguates and loads second scene without warning",
                        "Assets/Regions/South/Dungeon.unity",
                        disambiguated.path);
                    Check.Equal("Story 18 full path load logs zero warnings", 0, LogRecorder.OfSeverity(LogSeverity.Warning).Count);
                }
                finally
                {
                    SceneManager.ClearBuildScenes();
                    LogRecorder.Clear();
                }
            }
        }

        private static void TestStory18LoadingSceneAbsentFromBuildListLogsErrorAndPreservesActiveScene()
        {
            SceneManager.ClearBuildScenes();
            LogRecorder.Clear();
            FakeSceneProvider provider = new FakeSceneProvider(
                "Assets/Scenes/MainMenu.unity",
                "Assets/Scenes/SecretLevel.unity");
            FakeSceneBackend backend = new FakeSceneBackend();

            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes("Assets/Scenes/MainMenu.unity");
                SceneManager.EnforceBuildList = true;

                try
                {
                    Scene mainMenu = SceneManager.LoadScene("MainMenu", LoadSceneMode.Single);
                    Check.That("Story 18 build-listed MainMenu loads", mainMenu.IsValid() && mainMenu.isLoaded);
                    Check.Equal("Story 18 active scene is MainMenu", "MainMenu", SceneManager.GetActiveScene().name);
                    Check.Equal("Story 18 sceneCount is 1", 1, SceneManager.sceneCount);

                    LogRecorder.Clear();
                    bool syncAbsentThrew = false;
                    try
                    {
                        SceneManager.LoadScene("SecretLevel", LoadSceneMode.Single);
                    }
                    catch (SceneManagementException)
                    {
                        syncAbsentThrew = true;
                    }

                    Check.That("Story 18 synchronous LoadScene of scene absent from build list fails with SceneManagementException", syncAbsentThrew);
                    Check.Equal("Story 18 failed sync load leaves sceneCount at 1", 1, SceneManager.sceneCount);
                    Check.Equal("Story 18 failed sync load preserves active scene MainMenu", "MainMenu", SceneManager.GetActiveScene().name);

                    IReadOnlyList<LogEntry> syncErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                    Check.Equal("Story 18 failed sync load of absent scene logs an error", 1, syncErrors.Count);
                    if (syncErrors.Count == 1)
                    {
                        Check.That(
                            "Story 18 error names the absent scene SecretLevel",
                            syncErrors[0].Message.Contains("SecretLevel", StringComparison.Ordinal));
                        Check.That(
                            "Story 18 error mentions build settings",
                            syncErrors[0].Message.Contains("build settings", StringComparison.OrdinalIgnoreCase));
                    }

                    LogRecorder.Clear();
                    bool asyncAbsentThrew = false;
                    try
                    {
                        SceneManager.LoadSceneAsync("UnlistedAsyncScene", LoadSceneMode.Single);
                    }
                    catch (SceneManagementException)
                    {
                        asyncAbsentThrew = true;
                    }

                    Check.That("Story 18 LoadSceneAsync of scene absent from build list fails with SceneManagementException", asyncAbsentThrew);
                    Check.Equal("Story 18 failed async load leaves sceneCount at 1", 1, SceneManager.sceneCount);
                    Check.Equal("Story 18 failed async load preserves active scene MainMenu", "MainMenu", SceneManager.GetActiveScene().name);
                    Check.Equal("Story 18 failed async load logs an error", 1, LogRecorder.OfSeverity(LogSeverity.Error).Count);
                }
                finally
                {
                    SceneManager.ClearBuildScenes();
                    LogRecorder.Clear();
                }
            }
        }

        private static void TestStory18RefusingToUnloadOnlyLoadedScene()
        {
            SceneManager.ClearBuildScenes();
            LogRecorder.Clear();
            FakeSceneProvider provider = new FakeSceneProvider(
                "Assets/Scenes/SoleScene.unity",
                "Assets/Scenes/ExtraScene.unity");
            FakeSceneBackend backend = new FakeSceneBackend();

            using (BridgeHost host = new BridgeHost())
            {
                host.Provider = provider;
                host.Backend = backend;
                SceneManager.SetBuildScenes(
                    "Assets/Scenes/SoleScene.unity",
                    "Assets/Scenes/ExtraScene.unity");
                SceneManager.EnforceBuildList = true;

                try
                {
                    Scene sole = SceneManager.LoadScene("SoleScene", LoadSceneMode.Single);
                    Check.That("Story 18 SoleScene is loaded and valid", sole.IsValid() && sole.isLoaded);
                    Check.Equal("Story 18 sceneCount is 1 before unload attempt", 1, SceneManager.sceneCount);

                    LogRecorder.Clear();
                    bool unloadOnlyByNameThrew = false;
                    try
                    {
                        SceneManager.UnloadSceneAsync("SoleScene");
                    }
                    catch (SceneManagementException)
                    {
                        unloadOnlyByNameThrew = true;
                    }

                    Check.That("Story 18 UnloadSceneAsync(string) refuses to unload the only loaded scene", unloadOnlyByNameThrew);
                    Check.Equal("Story 18 refused UnloadSceneAsync leaves sceneCount at 1", 1, SceneManager.sceneCount);
                    Check.Equal("Story 18 refused UnloadSceneAsync preserves active scene SoleScene", "SoleScene", SceneManager.GetActiveScene().name);

                    IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                    Check.Equal("Story 18 refusing to unload the only scene logs an error", 1, errors.Count);
                    if (errors.Count == 1)
                    {
                        Check.That(
                            "Story 18 unload refusal error names SoleScene",
                            errors[0].Message.Contains("SoleScene", StringComparison.Ordinal));
                    }

                    LogRecorder.Clear();
                    bool unloadOnlyByStructThrew = false;
                    try
                    {
                        SceneManager.UnloadSceneAsync(sole);
                    }
                    catch (SceneManagementException)
                    {
                        unloadOnlyByStructThrew = true;
                    }

                    Check.That("Story 18 UnloadSceneAsync(Scene) also refuses to unload the only loaded scene", unloadOnlyByStructThrew);
                    Check.Equal("Story 18 refused UnloadSceneAsync(Scene) logs an error", 1, LogRecorder.OfSeverity(LogSeverity.Error).Count);

                    SceneManager.LoadScene("ExtraScene", LoadSceneMode.Additive);
                    Check.Equal("Story 18 additive load brings sceneCount to 2", 2, SceneManager.sceneCount);

                    AsyncOperation unloadFirst = SceneManager.UnloadSceneAsync("SoleScene");
                    host.Tick(0.016);
                    Check.That("Story 18 UnloadSceneAsync succeeds when another scene remains loaded", unloadFirst.isDone);
                    Check.Equal("Story 18 sceneCount drops to 1 after unloading one of two scenes", 1, SceneManager.sceneCount);
                    Check.Equal("Story 18 remaining active scene is ExtraScene", "ExtraScene", SceneManager.GetActiveScene().name);
                }
                finally
                {
                    SceneManager.ClearBuildScenes();
                    LogRecorder.Clear();
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
