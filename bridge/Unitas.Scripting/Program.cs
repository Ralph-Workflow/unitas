using System;
using System.Collections.Generic;

namespace Unitas.Scripting
{
    public static class Program
    {
        public static int Main(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                Console.Error.WriteLine("Usage: Unitas.Scripting <compile|generate-ide|diagnose|symbols|loop> <projectRoot> [--player] [--development] [--target <platform>] [--arch <arch>] [--define <SYMBOL>] [--type <TypeName>]");
                return 1;
            }

            string command = args[0];
            bool isCompile = string.Equals(command, "compile", StringComparison.OrdinalIgnoreCase);
            bool isGenerateIde = string.Equals(command, "generate-ide", StringComparison.OrdinalIgnoreCase);
            bool isDiagnose = string.Equals(command, "diagnose", StringComparison.OrdinalIgnoreCase);
            bool isSymbols = string.Equals(command, "symbols", StringComparison.OrdinalIgnoreCase);
            bool isLoop = string.Equals(command, "loop", StringComparison.OrdinalIgnoreCase);

            if (!isCompile && !isGenerateIde && !isDiagnose && !isSymbols && !isLoop)
            {
                Console.Error.WriteLine("Unknown command '" + command + "'. Expected 'compile', 'generate-ide', 'diagnose', 'symbols', or 'loop'.");
                return 1;
            }

            if (args.Length < 2 || string.IsNullOrWhiteSpace(args[1]))
            {
                Console.Error.WriteLine("Missing <projectRoot> argument for '" + command + "'.");
                return 1;
            }

            string projectRoot = args[1];
            ScriptCompilationOptions options = new ScriptCompilationOptions();
            string? typeFilter = null;

            for (int i = 2; i < args.Length; i++)
            {
                string arg = args[i];
                if (string.Equals(arg, "--player", StringComparison.OrdinalIgnoreCase))
                {
                    options.IsEditor = false;
                }
                else if (string.Equals(arg, "--editor", StringComparison.OrdinalIgnoreCase))
                {
                    options.IsEditor = true;
                }
                else if (string.Equals(arg, "--development", StringComparison.OrdinalIgnoreCase))
                {
                    options.IsDevelopmentBuild = true;
                }
                else if (string.Equals(arg, "--untrusted", StringComparison.OrdinalIgnoreCase))
                {
                    options.IsProjectTrusted = false;
                }
                else if (string.Equals(arg, "--target", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.TargetPlatform = ParsePlatform(args[i]);
                }
                else if (string.Equals(arg, "--arch", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.TargetArchitecture = args[i];
                }
                else if (string.Equals(arg, "--out", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.OutputDirectory = args[i];
                }
                else if (string.Equals(arg, "--define", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.AdditionalDefines.Add(args[i]);
                }
                else if (string.Equals(arg, "--type", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    typeFilter = args[i];
                }
            }

            if (isGenerateIde)
            {
                IdeGenerationResult ideResult = IdeProjectGenerator.Generate(projectRoot, options);
                IReadOnlyList<ScriptDiagnostic> ideDiagnostics = ideResult.Diagnostics;
                for (int i = 0; i < ideDiagnostics.Count; i++)
                {
                    Console.WriteLine(ideDiagnostics[i].FormatCli());
                }

                return ideResult.Succeeded ? 0 : 1;
            }

            if (isDiagnose)
            {
                DiagnoseReport report = ScriptCompilation.DiagnoseProject(projectRoot, options);
                Console.WriteLine(report.ToJson());
                return report.ErrorCount == 0 && report.Succeeded ? 0 : 1;
            }

            if (isSymbols)
            {
                SymbolQueryResult queryResult = SymbolQuery.QueryProject(projectRoot, typeFilter, options);
                Console.WriteLine(queryResult.ToJson());
                return 0;
            }

            if (isLoop)
            {
                CodeLoopReport loopReport = CodeLoop.Run(projectRoot, options, typeFilter);
                Console.WriteLine(loopReport.ToJson());
                return loopReport.Succeeded ? 0 : 1;
            }

            ScriptCompilationResult result = ScriptCompilation.CompileProject(projectRoot, options);
            IReadOnlyList<ScriptDiagnostic> diagnostics = result.Diagnostics;
            for (int i = 0; i < diagnostics.Count; i++)
            {
                Console.WriteLine(diagnostics[i].FormatCli());
            }

            return result.Succeeded ? 0 : 1;
        }

        private static ScriptTargetPlatform ParsePlatform(string value)
        {
            if (string.Equals(value, "StandaloneWindows64", StringComparison.OrdinalIgnoreCase)
                || string.Equals(value, "Windows", StringComparison.OrdinalIgnoreCase))
            {
                return ScriptTargetPlatform.StandaloneWindows64;
            }

            if (string.Equals(value, "StandaloneLinux64", StringComparison.OrdinalIgnoreCase)
                || string.Equals(value, "Linux", StringComparison.OrdinalIgnoreCase))
            {
                return ScriptTargetPlatform.StandaloneLinux64;
            }

            if (string.Equals(value, "DedicatedServer", StringComparison.OrdinalIgnoreCase)
                || string.Equals(value, "Server", StringComparison.OrdinalIgnoreCase))
            {
                return ScriptTargetPlatform.DedicatedServer;
            }

            if (string.Equals(value, "Android", StringComparison.OrdinalIgnoreCase))
            {
                return ScriptTargetPlatform.Android;
            }

            if (string.Equals(value, "iOS", StringComparison.OrdinalIgnoreCase))
            {
                return ScriptTargetPlatform.iOS;
            }

            if (string.Equals(value, "WebGL", StringComparison.OrdinalIgnoreCase))
            {
                return ScriptTargetPlatform.WebGL;
            }

            return ScriptTargetPlatform.StandaloneOSX;
        }
    }
}
