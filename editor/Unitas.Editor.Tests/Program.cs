using System;
using Unitas.Editor.Tests.Core;
using Unitas.Editor.Tests.Play;

namespace Unitas.Editor.Tests
{
    public static class Program
    {
        public static int Main(string[] args)
        {
            if (args.Length > 0 && string.Equals(args[0], "--perf", StringComparison.Ordinal))
            {
                return EditorPerfTests.Run(args);
            }

            // The suite table is explicit so the runner discovers suites without reflection.
            (string Name, Action Run)[] suites =
            {
                ("AuditTests", AuditTests.Run),
                ("BuildToolsTests", BuildToolsTests.Run),
                ("BuiltinDefaultsTests", BuiltinDefaultsTests.Run),
                ("ChangeReviewTests", ChangeReviewTests.Run),
                ("ExecuteMethodToolsTests", ExecuteMethodToolsTests.Run),
                ("ClockTests", ClockTests.Run),
                ("HeadlessCliTests", HeadlessCliTests.Run),
                ("HttpTransportTests", HttpTransportTests.Run),
                ("NativeToolBridgeTests", NativeToolBridgeTests.Run),
                ("OperationStoreTests", OperationStoreTests.Run),
                ("OperationToolsTests", OperationToolsTests.Run),
                ("PlaySessionLifecycleTests", PlayAndCaptureTests.PlaySessionLifecycleAndIsolationWithWorker),
                ("PlayWorkerCrashTests", PlayAndCaptureTests.WorkerCrashEndsOnlyThatPlaySessionAndHostContinues),
                ("PlayPersonOnlyGrantTests", PlayAndCaptureTests.PersonOnlyGrantRefusedWithoutCredentialOrOverStdioAndAllowedWithCredential),
                ("PlayLiveCaptureTests", PlayAndCaptureTests.LiveWorkerCaptureReturnsImageIdMapAndResolvableObjectBounds),
                ("PlayRuntimeDisposalTests", PlayAndCaptureTests.RuntimeDisposalStopsThePlayWorker),
                ("RegistryTests", RegistryTests.Run),
                ("ScenarioToolsTests", ScenarioToolsTests.Run),
                ("ScopeTests", ScopeTests.Run),
                ("SessionTests", SessionTests.Run),
                ("StdioTransportTests", StdioTransportTests.Run),
                ("UntrustedContentTests", UntrustedContentTests.Run),
                ("AuthoringToolsTests", AuthoringToolsTests.Run),
                ("ScenarioGlueDiscoveryTests", ScenarioGlueDiscoveryTests.Run),
                ("SceneTests", SceneTests.Run),
                ("UndoHistoryTests", UndoHistoryTests.Run),
                ("SceneToolsTests", SceneToolsTests.Run),
            };

            if (args.Length == 0)
            {
                foreach ((string name, Action run) in suites)
                {
                    RunSuite(name, run);
                }
            }
            else
            {
                RunSelected(suites, args[0]);
            }

            System.Console.WriteLine("failures: " + Check.FailureCount);
            return Check.FailureCount;
        }

        private static void RunSelected((string Name, Action Run)[] suites, string wanted)
        {
            foreach (string single in wanted.Split(',', StringSplitOptions.RemoveEmptyEntries))
            {
                string name = single.Trim();
                bool found = false;
                foreach ((string suiteName, Action run) in suites)
                {
                    if (suiteName == name)
                    {
                        RunSuite(suiteName, run);
                        found = true;
                        break;
                    }
                }

                if (!found)
                {
                    System.Console.WriteLine("unknown suite: " + name);
                    Check.That("suite " + name + " is in the suite table", false);
                }
            }
        }

        private static void RunSuite(string name, Action run)
        {
            Check.Suite = name;
            try
            {
                run();
            }
            catch (Exception error)
            {
                Check.That(name + " runs without throwing", false);
                System.Console.WriteLine("      " + error);
            }
        }
    }
}
