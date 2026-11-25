using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public sealed class RuntimeInitializeProbeBehaviour : MonoBehaviour
    {
        private static readonly List<string> s_sharedTrace = new List<string>();

        public static IReadOnlyList<string> SharedTrace => s_sharedTrace;

        public static void ClearTrace()
        {
            s_sharedTrace.Clear();
        }

        [RuntimeInitializeOnLoadMethod]
        public static void OnDefaultAfterSceneLoad()
        {
            s_sharedTrace.Add("Init:DefaultAfterSceneLoad");
        }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
        public static void OnBeforeSceneLoad()
        {
            s_sharedTrace.Add("Init:BeforeSceneLoad");
        }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSplashScreen)]
        public static void OnBeforeSplashScreen()
        {
            s_sharedTrace.Add("Init:BeforeSplashScreen");
        }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterAssembliesLoaded)]
        public static void OnAfterAssembliesLoaded()
        {
            s_sharedTrace.Add("Init:AfterAssembliesLoaded");
        }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.SubsystemRegistration)]
        public static void OnSubsystemRegistration()
        {
            s_sharedTrace.Add("Init:SubsystemRegistration");
        }

        public override void Awake()
        {
            s_sharedTrace.Add("Script:Awake");
        }

        public override void OnEnable()
        {
            s_sharedTrace.Add("Script:OnEnable");
        }

        public override void Start()
        {
            s_sharedTrace.Add("Script:Start");
        }
    }

    public static class DomainReloadRetentionProbe
    {
        public static int UnresetCounter { get; set; }

        public static int SubsystemResetCounter { get; set; }

        public static bool ResetInSubsystemRegistration { get; set; }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.SubsystemRegistration)]
        public static void OnSubsystemRegistration()
        {
            if (ResetInSubsystemRegistration)
            {
                SubsystemResetCounter = 0;
            }
        }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        public static void OnAfterSceneLoad()
        {
            UnresetCounter++;
            SubsystemResetCounter++;
        }
    }

    public static class RuntimeInitializeTests
    {
        private const string ProbeScriptGuid = "00000000000000000000000000000001";
        private const string SceneFixture = "crates/unitas-runtime/tests/fixtures/scene_with_script.unity";

        public static void Run()
        {
            TestAttributeConstructorsAndEnumValues();
            TestLoadTypeOrderingAndOncePerPlaySessionWithSceneLifecycle();
            TestDomainReloadDisabledRetainsStaticsUntilSubsystemRegistrationReset();
            TestDomainReloadEnabledResetsRegisteredCallbacksWhenConfigured();
            TestExceptionContainmentDuringRuntimeInitializeCallbacks();
            TestGlueDiscoveryFindsRuntimeInitializeMethods();
        }

        private static void TestAttributeConstructorsAndEnumValues()
        {
            Check.Equal("RuntimeInitializeLoadType.AfterSceneLoad is 0", 0, (int)RuntimeInitializeLoadType.AfterSceneLoad);
            Check.Equal("RuntimeInitializeLoadType.BeforeSceneLoad is 1", 1, (int)RuntimeInitializeLoadType.BeforeSceneLoad);
            Check.Equal("RuntimeInitializeLoadType.AfterAssembliesLoaded is 2", 2, (int)RuntimeInitializeLoadType.AfterAssembliesLoaded);
            Check.Equal("RuntimeInitializeLoadType.BeforeSplashScreen is 3", 3, (int)RuntimeInitializeLoadType.BeforeSplashScreen);
            Check.Equal("RuntimeInitializeLoadType.SubsystemRegistration is 4", 4, (int)RuntimeInitializeLoadType.SubsystemRegistration);

            RuntimeInitializeOnLoadMethodAttribute defaultAttr = new RuntimeInitializeOnLoadMethodAttribute();
            Check.Equal(
                "Default RuntimeInitializeOnLoadMethodAttribute loadType is AfterSceneLoad",
                (int)RuntimeInitializeLoadType.AfterSceneLoad,
                (int)defaultAttr.loadType);

            RuntimeInitializeOnLoadMethodAttribute subsystemAttr =
                new RuntimeInitializeOnLoadMethodAttribute(RuntimeInitializeLoadType.SubsystemRegistration);
            Check.Equal(
                "Explicit RuntimeInitializeOnLoadMethodAttribute(SubsystemRegistration) preserves loadType",
                (int)RuntimeInitializeLoadType.SubsystemRegistration,
                (int)subsystemAttr.loadType);

            RuntimeInitializeOnLoadMethodAttribute beforeSceneAttr =
                new RuntimeInitializeOnLoadMethodAttribute(RuntimeInitializeLoadType.BeforeSceneLoad);
            Check.Equal(
                "Explicit RuntimeInitializeOnLoadMethodAttribute(BeforeSceneLoad) preserves loadType",
                (int)RuntimeInitializeLoadType.BeforeSceneLoad,
                (int)beforeSceneAttr.loadType);
        }

        private static void TestLoadTypeOrderingAndOncePerPlaySessionWithSceneLifecycle()
        {
            string scenePath = ResolveFixture(SceneFixture);
            Check.That("Story 13 scene fixture exists", File.Exists(scenePath));

            RuntimeInitializeProbeBehaviour.ClearTrace();
            RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            ScriptGlueRegistry.Clear();

            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeProbeBehaviour.OnDefaultAfterSceneLoad);
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.BeforeSceneLoad,
                RuntimeInitializeProbeBehaviour.OnBeforeSceneLoad);
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.BeforeSplashScreen,
                RuntimeInitializeProbeBehaviour.OnBeforeSplashScreen);
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.AfterAssembliesLoaded,
                RuntimeInitializeProbeBehaviour.OnAfterAssembliesLoaded);
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.SubsystemRegistration,
                RuntimeInitializeProbeBehaviour.OnSubsystemRegistration);

            ScriptGlueRegistry.Register(ProbeScriptGuid, () => new RuntimeInitializeProbeBehaviour());

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    IReadOnlyList<string> trace = RuntimeInitializeProbeBehaviour.SharedTrace;
                    Check.Equal(
                        "Story 13 pre-scene load types run on play-session start in SubsystemRegistration -> AfterAssembliesLoaded -> BeforeSplashScreen order",
                        "Init:SubsystemRegistration, Init:AfterAssembliesLoaded, Init:BeforeSplashScreen",
                        string.Join(", ", trace));

                    host.LoadScene(scenePath);

                    Check.Equal(
                        "Story 13 first scene load runs BeforeSceneLoad before scene Awake/OnEnable and AfterSceneLoad after scene Awake/OnEnable",
                        "Init:SubsystemRegistration, Init:AfterAssembliesLoaded, Init:BeforeSplashScreen, "
                        + "Init:BeforeSceneLoad, Script:Awake, Script:OnEnable, Init:DefaultAfterSceneLoad",
                        string.Join(", ", trace));

                    host.Tick(0.02);

                    Check.Equal(
                        "Story 13 first tick runs Start without re-running RuntimeInitializeOnLoadMethod callbacks",
                        "Init:SubsystemRegistration, Init:AfterAssembliesLoaded, Init:BeforeSplashScreen, "
                        + "Init:BeforeSceneLoad, Script:Awake, Script:OnEnable, Init:DefaultAfterSceneLoad, Script:Start",
                        string.Join(", ", trace));

                    int countBeforeSecondLoad = trace.Count;
                    host.LoadScene(scenePath, LoadSceneMode.Single);
                    host.Tick(0.02);

                    int initOccurrences = 0;
                    for (int i = 0; i < trace.Count; i++)
                    {
                        if (trace[i].StartsWith("Init:", StringComparison.Ordinal))
                        {
                            initOccurrences++;
                        }
                    }

                    Check.Equal("Story 13 subsequent scene load in same play session does not re-run RuntimeInitializeOnLoadMethod callbacks (total 5)", 5, initOccurrences);
                    Check.That("Story 13 second scene load still ran script lifecycle callbacks", trace.Count > countBeforeSecondLoad);
                }
            }
            finally
            {
                RuntimeInitializeProbeBehaviour.ClearTrace();
                RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
                ScriptGlueRegistry.Clear();
            }
        }

        private static void TestDomainReloadDisabledRetainsStaticsUntilSubsystemRegistrationReset()
        {
            string scenePath = ResolveFixture(SceneFixture);
            RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeScriptGuid, () => new RuntimeInitializeProbeBehaviour());

            DomainReloadRetentionProbe.UnresetCounter = 0;
            DomainReloadRetentionProbe.SubsystemResetCounter = 0;
            DomainReloadRetentionProbe.ResetInSubsystemRegistration = false;

            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.SubsystemRegistration,
                DomainReloadRetentionProbe.OnSubsystemRegistration);
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.AfterSceneLoad,
                DomainReloadRetentionProbe.OnAfterSceneLoad);

            bool savedDomainReloadDisabled = BridgeHost.DomainReloadDisabled;
            try
            {
                BridgeHost.DomainReloadDisabled = true;

                using (BridgeHost session1 = new BridgeHost())
                {
                    Check.Equal("Story 13 session 1 reports IsDomainReloadDisabled true", true, session1.IsDomainReloadDisabled);
                    session1.LoadScene(scenePath);
                    Check.Equal("Story 13 session 1 increments UnresetCounter to 1", 1, DomainReloadRetentionProbe.UnresetCounter);
                    Check.Equal("Story 13 session 1 increments SubsystemResetCounter to 1", 1, DomainReloadRetentionProbe.SubsystemResetCounter);
                }

                using (BridgeHost session2 = new BridgeHost())
                {
                    session2.LoadScene(scenePath);
                    Check.Equal(
                        "Story 13 with domain reload disabled, unreset static counter retains value across play sessions (2)",
                        2,
                        DomainReloadRetentionProbe.UnresetCounter);
                    Check.Equal(
                        "Story 13 before SubsystemRegistration reset is enabled, second counter also retains and increments to 2",
                        2,
                        DomainReloadRetentionProbe.SubsystemResetCounter);
                }

                DomainReloadRetentionProbe.ResetInSubsystemRegistration = true;

                using (BridgeHost session3 = new BridgeHost())
                {
                    Check.Equal(
                        "Story 13 SubsystemRegistration resets counter to 0 upon entering session 3 before scene load",
                        0,
                        DomainReloadRetentionProbe.SubsystemResetCounter);
                    Check.Equal(
                        "Story 13 unreset static counter still retains 2 upon entering session 3 before AfterSceneLoad",
                        2,
                        DomainReloadRetentionProbe.UnresetCounter);

                    session3.LoadScene(scenePath);

                    Check.Equal(
                        "Story 13 after session 3 scene load, SubsystemRegistration-reset counter is 1",
                        1,
                        DomainReloadRetentionProbe.SubsystemResetCounter);
                    Check.Equal(
                        "Story 13 after session 3 scene load, unreset static counter retained and incremented to 3",
                        3,
                        DomainReloadRetentionProbe.UnresetCounter);
                }
            }
            finally
            {
                BridgeHost.DomainReloadDisabled = savedDomainReloadDisabled;
                DomainReloadRetentionProbe.UnresetCounter = 0;
                DomainReloadRetentionProbe.SubsystemResetCounter = 0;
                DomainReloadRetentionProbe.ResetInSubsystemRegistration = false;
                RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
                ScriptGlueRegistry.Clear();
            }
        }

        private static void TestDomainReloadEnabledResetsRegisteredCallbacksWhenConfigured()
        {
            List<string> order = new List<string>();
            RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();

            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.AfterSceneLoad,
                () => order.Add("AfterSceneLoad"));
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.BeforeSceneLoad,
                () => order.Add("BeforeSceneLoad"));
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.BeforeSplashScreen,
                () => order.Add("BeforeSplashScreen"));
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.AfterAssembliesLoaded,
                () => order.Add("AfterAssembliesLoaded"));
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.SubsystemRegistration,
                () => order.Add("SubsystemRegistration"));

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.CompleteRuntimeInitializationForSession();
                    Check.Equal(
                        "Story 13 CompleteRuntimeInitializationForSession runs all 5 load types in exact order",
                        "SubsystemRegistration, AfterAssembliesLoaded, BeforeSplashScreen, BeforeSceneLoad, AfterSceneLoad",
                        string.Join(", ", order));

                    host.CompleteRuntimeInitializationForSession();
                    Check.Equal(
                        "Story 13 CompleteRuntimeInitializationForSession is idempotent within the same play session",
                        5,
                        order.Count);
                }
            }
            finally
            {
                RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            }
        }

        private static void TestExceptionContainmentDuringRuntimeInitializeCallbacks()
        {
            List<string> trace = new List<string>();
            RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            LogRecorder.Clear();

            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.SubsystemRegistration,
                () =>
                {
                    trace.Add("FirstSubsystem");
                    throw new InvalidOperationException("Boom in SubsystemRegistration");
                });
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.SubsystemRegistration,
                () => trace.Add("SecondSubsystem"));
            RuntimeInitializeOnLoadMethodAttribute.Register(
                RuntimeInitializeLoadType.AfterAssembliesLoaded,
                () => trace.Add("AfterAssemblies"));

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    _ = host;
                    Check.Equal(
                        "Exception in one RuntimeInitializeOnLoadMethod callback logs and continues to remaining callbacks",
                        "FirstSubsystem, SecondSubsystem, AfterAssemblies",
                        string.Join(", ", trace));
                    Check.Equal(
                        "Faulting RuntimeInitializeOnLoadMethod logs one error entry",
                        1,
                        LogRecorder.OfSeverity(LogSeverity.Error).Count);
                }
            }
            finally
            {
                LogRecorder.Clear();
                RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            }
        }

        private static void TestGlueDiscoveryFindsRuntimeInitializeMethods()
        {
            string tempFile = Path.Combine(
                Path.GetTempPath(),
                "unitas_runtime_init_discover_" + Guid.NewGuid().ToString("N", System.Globalization.CultureInfo.InvariantCulture) + ".cs");
            try
            {
                File.WriteAllText(
                    tempFile,
                    "using UnityEngine;\n"
                    + "public class CompiledInitProbe : MonoBehaviour\n"
                    + "{\n"
                    + "    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.SubsystemRegistration)]\n"
                    + "    public static void OnSubsystems() { }\n"
                    + "    [RuntimeInitializeOnLoadMethod]\n"
                    + "    public static void OnAfterScene() { }\n"
                    + "}\n");
                List<Unitas.Scripting.RuntimeInitializeMethodBinding> found =
                    Unitas.Scripting.ScriptAnalyzerAndGlueGenerator.DiscoverRuntimeInitializeMethodsForAssembly(
                        new[] { tempFile },
                        new HashSet<string>());
                Check.Equal("Story 13 glue discovery finds 2 [RuntimeInitializeOnLoadMethod] bindings", 2, found.Count);
            }
            finally
            {
                if (File.Exists(tempFile))
                {
                    File.Delete(tempFile);
                }
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
