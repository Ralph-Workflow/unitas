using System;
using Unitas.Editor.Tests.Core;
using Unitas.Editor.Tests.Play;

namespace Unitas.Editor.Tests
{
    public static class Program
    {
        public static int Main(string[] args)
        {
            // The suite table is explicit so the runner discovers suites without reflection.
            (string Name, Action Run)[] suites =
            {
                ("AuditTests", AuditTests.Run),
                ("BuildToolsTests", BuildToolsTests.Run),
                ("BuiltinDefaultsTests", BuiltinDefaultsTests.Run),
                ("ChangeReviewTests", ChangeReviewTests.Run),
                ("ClockTests", ClockTests.Run),
                ("HeadlessCliTests", HeadlessCliTests.Run),
                ("HttpTransportTests", HttpTransportTests.Run),
                ("NativeToolBridgeTests", NativeToolBridgeTests.Run),
                ("OperationStoreTests", OperationStoreTests.Run),
                ("OperationToolsTests", OperationToolsTests.Run),
                ("PlayAndCaptureTests", PlayAndCaptureTests.Run),
                ("RegistryTests", RegistryTests.Run),
                ("ScenarioToolsTests", ScenarioToolsTests.Run),
                ("ScopeTests", ScopeTests.Run),
                ("SessionTests", SessionTests.Run),
                ("StdioTransportTests", StdioTransportTests.Run),
                ("UntrustedContentTests", UntrustedContentTests.Run),
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
            foreach ((string name, Action run) in suites)
            {
                if (name == wanted)
                {
                    RunSuite(name, run);
                    return;
                }
            }

            System.Console.WriteLine("unknown suite: " + wanted);
            Check.That("suite " + wanted + " is in the suite table", false);
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
