using System;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Playtest;

namespace Unitas.Scripting
{
    public static class ScenarioCommand
    {
        public static int Run(string projectRoot, string scenarioPath, int repeat)
        {
            if (string.IsNullOrWhiteSpace(projectRoot))
            {
                Console.Error.WriteLine("Missing <projectRoot> argument for 'scenario'.");
                return 1;
            }

            if (string.IsNullOrWhiteSpace(scenarioPath) || !File.Exists(scenarioPath))
            {
                Console.Error.WriteLine("Scenario file was not found: " + scenarioPath);
                return 1;
            }

            // Mirror CodeLoop's compile + glue steps so scene scripts resolve from generated glue.
            ScriptCompilationOptions options = new ScriptCompilationOptions();
            options.EmitGeneratedGlue = true;
            options.AutoLoadGeneratedGlue = true;

            DiagnoseReport diagnose = ScriptCompilation.DiagnoseProject(projectRoot, options);
            if (!diagnose.Succeeded || diagnose.ErrorCount > 0)
            {
                Console.Error.WriteLine(diagnose.ToJson());
                return 1;
            }

            ScenarioDocument scenario;
            try
            {
                scenario = ScenarioDocument.Parse(File.ReadAllText(scenarioPath));
            }
            catch (ScenarioFormatException ex)
            {
                Console.Error.WriteLine("Scenario file is invalid: " + ex.Message);
                return 1;
            }

            ScenarioRunner runner = new ScenarioRunner();
            ScenarioResult first = runner.Run(scenario, () => new BridgeHost());

            // shortcut: any repeat count >= 2 runs the scenario exactly twice, which is all the comparison needs.
            bool repeatable = true;
            if (repeat >= 2)
            {
                ScenarioResult second = runner.Run(scenario, () => new BridgeHost());
                repeatable = ScenarioRunner.CompareForRepeatability(first, second, out string? difference);
                if (!repeatable)
                {
                    Console.Error.WriteLine("Scenario is not repeatable: " + difference);
                }
            }

            Console.WriteLine(first.ToReportJson());

            if (first.Status != ScenarioStatus.Passed)
            {
                return 1;
            }

            return repeat >= 2 && !repeatable ? 1 : 0;
        }
    }
}
