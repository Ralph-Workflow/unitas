using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public static class ApplicationTests
    {
        public static void Run()
        {
            TestStandalonePathsAndDefaultsWithoutHost();
            TestSessionStartupShutdownAndPostShutdownGuards();
            TestBuildKindDistinguishesEditorDevelopmentAndReleaseSessions();
            TestFocusAndPauseEventsAndScriptDispatchWithExceptionContainment();
            TestQuitVetoAndNormalQuitLifecycleOrdering();
            TestProjectIdentityResolutionAndFallbacks();
        }

        private static void TestStandalonePathsAndDefaultsWithoutHost()
        {
            Application.Host = null;
            Application.ClearEvents();

            Check.Equal("Standalone isPlaying is false without host", false, Application.isPlaying);
            Check.Equal("Standalone isEditor is false without host", false, Application.isEditor);
            Check.Equal("Standalone isFocused is false without host", false, Application.isFocused);
            Check.Equal("Standalone isPaused is false without host", false, Application.isPaused);
            Check.Equal("Standalone isDevelopmentBuild is false without host", false, Application.isDevelopmentBuild);
            Check.Equal("Standalone companyName default", "DefaultCompany", Application.companyName);
            Check.Equal("Standalone productName default", "UnitasProduct", Application.productName);
            Check.Equal("Standalone version default", "0.1", Application.version);
            Check.Equal("Standalone identifier default", "com.defaultcompany.unitasproduct", Application.identifier);
            Check.That("Standalone unityVersion is non-empty", !string.IsNullOrEmpty(Application.unityVersion));

            string persistentPath = Application.persistentDataPath;
            string tempCachePath = Application.temporaryCachePath;
            string dataPath = Application.dataPath;
            string streamingPath = Application.streamingAssetsPath;

            Check.That("Standalone persistentDataPath is non-empty", !string.IsNullOrEmpty(persistentPath));
            Check.That("Standalone persistentDataPath is rooted", Path.IsPathRooted(persistentPath));
            Check.That("Standalone persistentDataPath directory exists", Directory.Exists(persistentPath));

            Check.That("Standalone temporaryCachePath is non-empty", !string.IsNullOrEmpty(tempCachePath));
            Check.That("Standalone temporaryCachePath is rooted", Path.IsPathRooted(tempCachePath));
            Check.That("Standalone temporaryCachePath directory exists", Directory.Exists(tempCachePath));

            Check.That("Standalone dataPath is non-empty", !string.IsNullOrEmpty(dataPath));
            Check.That("Standalone dataPath is rooted", Path.IsPathRooted(dataPath));
            Check.That("Standalone streamingAssetsPath is non-empty", !string.IsNullOrEmpty(streamingPath));
            Check.That("Standalone streamingAssetsPath is rooted", Path.IsPathRooted(streamingPath));

            string repoRoot = Path.GetFullPath(".");
            Check.That(
                "Standalone persistentDataPath is outside project root",
                !Path.GetFullPath(persistentPath).StartsWith(repoRoot + Path.DirectorySeparatorChar, StringComparison.Ordinal));

            string tempOverride = Path.Combine(Path.GetTempPath(), "unitas_app_pdp_override_" + Guid.NewGuid().ToString("N"));
            try
            {
                Application.PersistentDataPathOverride = tempOverride;
                Check.Equal("PersistentDataPathOverride redirects persistentDataPath", Path.GetFullPath(tempOverride), Path.GetFullPath(Application.persistentDataPath));
                Check.That("PersistentDataPathOverride directory is created", Directory.Exists(tempOverride));
            }
            finally
            {
                Application.PersistentDataPathOverride = null;
                if (Directory.Exists(tempOverride))
                {
                    Directory.Delete(tempOverride, true);
                }
            }
        }

        private static void TestSessionStartupShutdownAndPostShutdownGuards()
        {
            Application.ClearEvents();
            Check.Equal("Before host ctor isPlaying is false", false, Application.isPlaying);

            Action onQuitting = () => Debug.Log("APP_TRACE:quitting");
            Application.quitting += onQuitting;

            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("Scene fixture exists for startup/shutdown test", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            CompilationTests.EnsureCompiledGlueLoaded();
            LogRecorder.Clear();

            BridgeHost host = new BridgeHost();
            try
            {
                Check.Equal("After host ctor isPlaying is true", true, Application.isPlaying);
                Check.Equal("After host ctor isEditor is true by default", true, Application.isEditor);
                Check.Equal("After host ctor isFocused is true by default", true, Application.isFocused);
                Check.Equal("After host ctor isPaused is false by default", false, Application.isPaused);

                host.LoadScene(scenePath);
                host.Tick(0.02);

                Check.Equal(
                    "Host dataPath points to loaded scene directory when no Assets parent exists",
                    Path.GetDirectoryName(scenePath) ?? string.Empty,
                    Application.dataPath);
                Check.Equal(
                    "Host streamingAssetsPath points to StreamingAssets under dataPath",
                    Path.Combine(Application.dataPath, "StreamingAssets"),
                    Application.streamingAssetsPath);

                host.Shutdown();

                Check.Equal("After Shutdown isPlaying is false", false, Application.isPlaying);
                Check.Equal(
                    "Shutdown fires quitting then OnApplicationQuit then native teardown callbacks",
                    "S1:OnEnable, quitting, S1:Quit, S1:OnDisable, S1:OnDestroy",
                    CollectAppTrace());

                host.Shutdown();
                Check.Equal(
                    "Second Shutdown is idempotent and does not re-fire quitting or OnApplicationQuit",
                    "S1:OnEnable, quitting, S1:Quit, S1:OnDisable, S1:OnDestroy",
                    CollectAppTrace());

                bool tickThrew = false;
                try
                {
                    host.Tick(0.02);
                }
                catch (ObjectDisposedException)
                {
                    tickThrew = true;
                }

                Check.That("Tick after Shutdown throws ObjectDisposedException", tickThrew);

                bool loadThrew = false;
                try
                {
                    host.LoadScene(scenePath);
                }
                catch (ObjectDisposedException)
                {
                    loadThrew = true;
                }

                Check.That("LoadScene(path) after Shutdown throws ObjectDisposedException", loadThrew);

                bool loadModeThrew = false;
                try
                {
                    host.LoadScene(scenePath, LoadSceneMode.Single);
                }
                catch (ObjectDisposedException)
                {
                    loadModeThrew = true;
                }

                Check.That("LoadScene(path, mode) after Shutdown throws ObjectDisposedException", loadModeThrew);
            }
            finally
            {
                Application.quitting -= onQuitting;
                host.Dispose();
            }
        }

        private static void TestBuildKindDistinguishesEditorDevelopmentAndReleaseSessions()
        {
            using (BridgeHost host = new BridgeHost())
            {
                Check.Equal("Default host BuildKind is Editor", (int)ApplicationBuildKind.Editor, (int)host.BuildKind);
                Check.Equal("Editor session reports isEditor true", true, Application.isEditor);
                Check.Equal("Editor session reports isDevelopmentBuild true", true, Application.isDevelopmentBuild);
                Check.Equal("Editor session reports Debug.isDebugBuild true", true, Debug.isDebugBuild);
                Check.Equal("Editor session reports Application.buildKind Editor", (int)ApplicationBuildKind.Editor, (int)Application.buildKind);
                string editorIdentifier = Application.identifier;

                host.BuildKind = ApplicationBuildKind.DevelopmentPlayer;
                Check.Equal("DevelopmentPlayer session reports isEditor false", false, Application.isEditor);
                Check.Equal("DevelopmentPlayer session reports isDevelopmentBuild true", true, Application.isDevelopmentBuild);
                Check.Equal("DevelopmentPlayer session reports Debug.isDebugBuild true", true, Debug.isDebugBuild);
                Check.Equal(
                    "DevelopmentPlayer session reports Application.buildKind DevelopmentPlayer",
                    (int)ApplicationBuildKind.DevelopmentPlayer,
                    (int)Application.buildKind);
                string devIdentifier = Application.identifier;

                host.BuildKind = ApplicationBuildKind.ReleasePlayer;
                Check.Equal("ReleasePlayer session reports isEditor false", false, Application.isEditor);
                Check.Equal("ReleasePlayer session reports isDevelopmentBuild false", false, Application.isDevelopmentBuild);
                Check.Equal("ReleasePlayer session reports Debug.isDebugBuild false", false, Debug.isDebugBuild);
                Check.Equal(
                    "ReleasePlayer session reports Application.buildKind ReleasePlayer",
                    (int)ApplicationBuildKind.ReleasePlayer,
                    (int)Application.buildKind);
                string releaseIdentifier = Application.identifier;

                Check.That("Editor and DevelopmentPlayer identifiers are distinct", editorIdentifier != devIdentifier);
                Check.That("DevelopmentPlayer and ReleasePlayer identifiers are distinct", devIdentifier != releaseIdentifier);
                Check.That("Editor and ReleasePlayer identifiers are distinct", editorIdentifier != releaseIdentifier);
            }
        }

        private static void TestFocusAndPauseEventsAndScriptDispatchWithExceptionContainment()
        {
            Application.ClearEvents();
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Physics query scene exists for multi-script focus/pause test", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            CompilationTests.EnsureCompiledGlueLoaded();

            Action<bool> onFocus = focused => Debug.Log("APP_TRACE:Event:Focus:" + (focused ? "True" : "False"));
            Action<bool> onPause = paused => Debug.Log("APP_TRACE:Event:Pause:" + (paused ? "True" : "False"));
            Application.focusChanged += onFocus;
            Application.pauseChanged += onPause;
            LogRecorder.Clear();

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    MonoBehaviour? scriptA = host.GetScript(1UL);
                    MonoBehaviour? scriptB = host.GetScript(2UL);
                    Check.That("Attached compiled scriptA is non-null", scriptA != null);
                    Check.That("Attached compiled scriptB is non-null", scriptB != null);
                    scriptA!.SendMessage("SetThrowOnFocus", true);
                    scriptA.SendMessage("SetThrowOnPause", true);
                    LogRecorder.Clear();

                    host.SetFocus(false);
                    Check.Equal("Application.isFocused is false after SetFocus(false)", false, Application.isFocused);

                    host.SetPaused(true);
                    Check.Equal("Application.isPaused is true after SetPaused(true)", true, Application.isPaused);

                    host.SetPaused(false);
                    Check.Equal("Application.isPaused is false after SetPaused(false)", false, Application.isPaused);

                    host.SetFocus(true);
                    Check.Equal("Application.isFocused is true after SetFocus(true)", true, Application.isFocused);

                    scriptB!.enabled = false;
                    host.SetFocus(false);
                    scriptB.enabled = true;
                    scriptB.gameObject.SetActive(false);
                    host.SetPaused(true);
                    scriptB.gameObject.SetActive(true);
                    host.SetFocus(true);

                    Check.Equal(
                        "Focus and pause events fire before eligible script callbacks in registration order, skip disabled/inactive scripts, and survive script exceptions",
                        "Event:Focus:False, A:Focus:False, B:Focus:False, Event:Pause:True, A:Pause:True, B:Pause:True, Event:Pause:False, A:Pause:False, B:Pause:False, Event:Focus:True, A:Focus:True, B:Focus:True, Event:Focus:False, A:Focus:False, B:OnDisable, Event:Pause:True, A:Pause:True, B:OnEnable, Event:Focus:True, A:Focus:True, B:Focus:True",
                        CollectAppTrace());

                    IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                    Check.Equal("Throwing focus and pause callbacks log seven exception entries", 7, errors.Count);
                    for (int i = 0; i < errors.Count; i++)
                    {
                        Check.That("Logged exception carries scriptA as origin", ReferenceEquals(errors[i].Origin, scriptA));
                    }
                }
            }
            finally
            {
                Application.focusChanged -= onFocus;
                Application.pauseChanged -= onPause;
            }
        }

        private static void TestQuitVetoAndNormalQuitLifecycleOrdering()
        {
            Application.ClearEvents();
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Scene exists for Quit veto and normal Quit test", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            CompilationTests.EnsureCompiledGlueLoaded();

            bool allowQuit = false;
            Func<bool> vetoGate = () =>
            {
                Debug.Log("APP_TRACE:wantsToQuit:" + (allowQuit ? "Allow" : "Veto"));
                return allowQuit;
            };
            Action onQuitting = () => Debug.Log("APP_TRACE:Event:quitting");

            Application.wantsToQuit += vetoGate;
            Application.quitting += onQuitting;
            LogRecorder.Clear();

            BridgeHost host = new BridgeHost();
            try
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);
                MonoBehaviour? scriptA = host.GetScript(1UL);
                MonoBehaviour? scriptB = host.GetScript(2UL);
                Check.That("Attached compiled scriptA is non-null for Quit test", scriptA != null);
                Check.That("Attached compiled scriptB is non-null for Quit test", scriptB != null);
                scriptA!.SendMessage("SetThrowOnQuit", true);
                LogRecorder.Clear();

                Application.Quit();

                Check.Equal("Vetoed Quit leaves isPlaying true", true, Application.isPlaying);
                Check.Equal("Vetoed Quit records only wantsToQuit", "wantsToQuit:Veto", CollectAppTrace());

                host.Tick(0.02);
                Check.Equal("Host remains usable and ticks after vetoed Quit", true, Application.isPlaying);

                LogRecorder.Clear();
                allowQuit = true;
                Application.Quit();

                Check.Equal("Accepted Quit sets isPlaying to false", false, Application.isPlaying);
                Check.Equal(
                    "Accepted Quit runs wantsToQuit -> quitting -> OnApplicationQuit in order -> native OnDisable/OnDestroy",
                    "wantsToQuit:Allow, Event:quitting, A:Quit, B:Quit, B:OnDisable, A:OnDisable, B:OnDestroy, A:OnDestroy",
                    CollectAppTrace());

                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Exception in A.OnApplicationQuit is logged and does not stop B.OnApplicationQuit", 1, errors.Count);
                if (errors.Count == 1)
                {
                    Check.That("Logged quit exception carries scriptA as origin", ReferenceEquals(errors[0].Origin, scriptA));
                }

                bool tickAfterQuitThrew = false;
                try
                {
                    host.Tick(0.02);
                }
                catch (ObjectDisposedException)
                {
                    tickAfterQuitThrew = true;
                }

                Check.That("Host rejects Tick after accepted Application.Quit()", tickAfterQuitThrew);
            }
            finally
            {
                Application.wantsToQuit -= vetoGate;
                Application.quitting -= onQuitting;
                host.Dispose();
            }
        }

        private static void TestProjectIdentityResolutionAndFallbacks()
        {
            string sceneFixture = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            string settingsSceneFixture = ResolveFixture("crates/unitas-runtime/tests/fixtures/project_with_settings/Assets/Scenes/SettingsScene.unity");
            string projectSettingsFixture = ResolveFixture("crates/unitas-asset/tests/fixtures/prefab_project/ProjectSettings/ProjectSettings.asset");
            Check.That("ProjectSettings.asset fixture exists", File.Exists(projectSettingsFixture));
            Check.That("SettingsScene.unity fixture exists", File.Exists(settingsSceneFixture));

            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas_app_id_test_" + Guid.NewGuid().ToString("N"));
            string settingsDir = Path.Combine(tempRoot, "ProjectSettings");
            string assetsDir = Path.Combine(tempRoot, "Assets");
            Directory.CreateDirectory(settingsDir);
            Directory.CreateDirectory(assetsDir);

            string tempSettings = Path.Combine(settingsDir, "ProjectSettings.asset");
            string tempScene = Path.Combine(assetsDir, "IdentityScene.unity");
            File.WriteAllText(
                tempSettings,
                "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  companyName: Acme Studios\n  productName: Super Game\n  bundleVersion: 2.4.1\n");
            File.Copy(sceneFixture, tempScene, true);

            try
            {
                ScriptGlueRegistry.Clear();
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(settingsSceneFixture);
                    Check.Equal("Fixture project_with_settings resolves companyName", "Unitas", Application.companyName);
                    Check.Equal("Fixture project_with_settings resolves productName", "SettingsFixture", Application.productName);
                    Check.Equal("Fixture project_with_settings resolves version", "1.2.3", Application.version);
                    Check.Equal("Fixture project_with_settings derives editor identifier", "com.unitas.settingsfixture.editor", Application.identifier);

                    host.LoadScene(tempScene);
                    Check.Equal("Loaded project resolves companyName", "Acme Studios", Application.companyName);
                    Check.Equal("Loaded project resolves productName", "Super Game", Application.productName);
                    Check.Equal("Loaded project resolves version", "2.4.1", Application.version);
                    Check.Equal("Loaded project derives editor identifier", "com.acmestudios.supergame.editor", Application.identifier);
                    host.BuildKind = ApplicationBuildKind.ReleasePlayer;
                    Check.Equal("Loaded project derives release identifier", "com.acmestudios.supergame", Application.identifier);
                    host.BuildKind = ApplicationBuildKind.Editor;
                    Check.Equal("Loaded project resolves dataPath to Assets folder", Path.GetFullPath(assetsDir), Path.GetFullPath(Application.dataPath));
                    Check.Equal(
                        "Loaded project resolves streamingAssetsPath",
                        Path.GetFullPath(Path.Combine(assetsDir, "StreamingAssets")),
                        Path.GetFullPath(Application.streamingAssetsPath));

                    host.LoadScene(sceneFixture);
                    Check.Equal("Scene without ProjectSettings falls back to DefaultCompany", "DefaultCompany", Application.companyName);
                    Check.Equal("Scene without ProjectSettings falls back to version 0.1", "0.1", Application.version);
                    Check.That("Scene without ProjectSettings has non-empty productName fallback", !string.IsNullOrEmpty(Application.productName));
                }
            }
            finally
            {
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static string CollectAppTrace()
        {
            List<string> trace = new List<string>();
            IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
            for (int i = 0; i < entries.Count; i++)
            {
                string msg = entries[i].Message;
                if (msg.StartsWith("APP_TRACE:", StringComparison.Ordinal))
                {
                    trace.Add(msg.Substring("APP_TRACE:".Length));
                }
            }

            return string.Join(", ", trace);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
