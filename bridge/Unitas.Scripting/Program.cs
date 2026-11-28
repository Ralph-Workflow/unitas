using System;
using System.Collections.Generic;
using System.Globalization;

namespace Unitas.Scripting
{
    public static class Program
    {
        public static int Main(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                Console.Error.WriteLine("Usage: Unitas.Scripting <compile|reload|generate-ide|diagnose|symbols|usages|attach-component|play|loop|scenario> <projectRoot> [--player] [--development] [--debug] [--target <platform>] [--arch <arch>] [--define <SYMBOL>] [--type <TypeName>] [--script <ScriptOrGuid>] [--prefab <PrefabPath>] [--object <ObjectName>] [--field <Name=Value>] [--usages <ScriptOrGuid>] [--play [<SceneOrPrefabPath>]] [--ticks <N>] | scenario <projectRoot> <scenario.json> [--repeat 2] [--frame-rates 30,60,144,240]");
                return 1;
            }

            string command = args[0];
            bool isCompile = string.Equals(command, "compile", StringComparison.OrdinalIgnoreCase);
            bool isReload = string.Equals(command, "reload", StringComparison.OrdinalIgnoreCase);
            bool isGenerateIde = string.Equals(command, "generate-ide", StringComparison.OrdinalIgnoreCase);
            bool isDiagnose = string.Equals(command, "diagnose", StringComparison.OrdinalIgnoreCase);
            bool isSymbols = string.Equals(command, "symbols", StringComparison.OrdinalIgnoreCase);
            bool isUsages = string.Equals(command, "usages", StringComparison.OrdinalIgnoreCase);
            bool isAttachComponent = string.Equals(command, "attach-component", StringComparison.OrdinalIgnoreCase);
            bool isPlay = string.Equals(command, "play", StringComparison.OrdinalIgnoreCase);
            bool isLoop = string.Equals(command, "loop", StringComparison.OrdinalIgnoreCase);
            bool isScenario = string.Equals(command, "scenario", StringComparison.OrdinalIgnoreCase);

            if (!isCompile
                && !isReload
                && !isGenerateIde
                && !isDiagnose
                && !isSymbols
                && !isUsages
                && !isAttachComponent
                && !isPlay
                && !isLoop
                && !isScenario)
            {
                Console.Error.WriteLine("Unknown command '" + command + "'. Expected 'compile', 'reload', 'generate-ide', 'diagnose', 'symbols', 'usages', 'attach-component', 'play', 'loop', or 'scenario'.");
                return 1;
            }

            if (args.Length < 2 || string.IsNullOrWhiteSpace(args[1]))
            {
                Console.Error.WriteLine("Missing <projectRoot> argument for '" + command + "'.");
                return 1;
            }

            string projectRoot = args[1];

            if (isScenario)
            {
                return RunScenarioCommand(projectRoot, args);
            }

            ParseCliOptions(
                args,
                isPlay,
                out ScriptCompilationOptions options,
                out string? typeFilter,
                out string? scriptTarget,
                out string? prefabTarget,
                out string? targetObjectName,
                out string? usageQuery,
                out bool enterPlayMode,
                out string? playTarget,
                out int playTicks,
                out Dictionary<string, string> serializedFields,
                out bool debuggingEnabled);

            if (isReload)
            {
                return RunReloadCommand(projectRoot, options);
            }

            return DispatchCliCommand(
                projectRoot,
                options,
                isGenerateIde,
                isDiagnose,
                isSymbols,
                isUsages,
                isAttachComponent,
                isPlay,
                isLoop,
                typeFilter,
                scriptTarget,
                prefabTarget,
                targetObjectName,
                usageQuery,
                enterPlayMode,
                playTarget,
                playTicks,
                serializedFields,
                debuggingEnabled);
        }

        private static int RunReloadCommand(string projectRoot, ScriptCompilationOptions options)
        {
            ScriptDomainReload.Result reloadResult = ScriptDomainReload.Reload(new ScriptDomainReload.Request
            {
                ProjectRoot = projectRoot,
                Options = options,
            });
            IReadOnlyList<ScriptDiagnostic> diagnostics = reloadResult.Diagnostics;
            for (int i = 0; i < diagnostics.Count; i++)
            {
                Console.WriteLine(diagnostics[i].FormatCli());
            }

            if (!reloadResult.Succeeded && !string.IsNullOrEmpty(reloadResult.Error))
            {
                Console.Error.WriteLine(reloadResult.Error);
            }

            return reloadResult.Succeeded ? 0 : 1;
        }

        private static int RunScenarioCommand(string projectRoot, string[] args)
        {
            if (args.Length < 3 || string.IsNullOrWhiteSpace(args[2]))
            {
                Console.Error.WriteLine("Missing <scenario.json> argument for 'scenario'.");
                return 1;
            }

            int repeat = 1;
            for (int i = 3; i < args.Length; i++)
            {
                if (string.Equals(args[i], "--repeat", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    if (!int.TryParse(args[i], NumberStyles.Integer, CultureInfo.InvariantCulture, out repeat) || repeat < 1)
                    {
                        Console.Error.WriteLine("Invalid --repeat value '" + args[i] + "' for 'scenario'.");
                        return 1;
                    }
                }
            }

            return ScenarioCommand.Run(projectRoot, args[2], repeat);
        }

        private static void ParseCliOptions(
            string[] args,
            bool isPlay,
            out ScriptCompilationOptions options,
            out string? typeFilter,
            out string? scriptTarget,
            out string? prefabTarget,
            out string? targetObjectName,
            out string? usageQuery,
            out bool enterPlayMode,
            out string? playTarget,
            out int playTicks,
            out Dictionary<string, string> serializedFields,
            out bool debuggingEnabled)
        {
            options = new ScriptCompilationOptions();
            typeFilter = null;
            scriptTarget = null;
            prefabTarget = null;
            targetObjectName = null;
            usageQuery = null;
            enterPlayMode = isPlay;
            playTarget = null;
            playTicks = 2;
            serializedFields = new Dictionary<string, string>(StringComparer.Ordinal);
            debuggingEnabled = false;

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
                else if (string.Equals(arg, "--debug", StringComparison.OrdinalIgnoreCase))
                {
                    debuggingEnabled = true;
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
                else if (string.Equals(arg, "--script", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    scriptTarget = args[i];
                }
                else if (string.Equals(arg, "--prefab", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    prefabTarget = args[i];
                }
                else if (string.Equals(arg, "--object", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    targetObjectName = args[i];
                }
                else if (string.Equals(arg, "--field", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    string pair = args[i];
                    int eq = pair.IndexOf('=', StringComparison.Ordinal);
                    if (eq > 0)
                    {
                        string k = pair.Substring(0, eq).Trim();
                        string v = pair.Substring(eq + 1).Trim();
                        if (!string.IsNullOrEmpty(k))
                        {
                            serializedFields[k] = v;
                        }
                    }
                }
                else if (string.Equals(arg, "--usages", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    usageQuery = args[i];
                }
                else if (string.Equals(arg, "--play", StringComparison.OrdinalIgnoreCase))
                {
                    enterPlayMode = true;
                    if (i + 1 < args.Length && !args[i + 1].StartsWith("--", StringComparison.Ordinal))
                    {
                        i++;
                        playTarget = args[i];
                    }
                }
                else if (string.Equals(arg, "--scene", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    playTarget = args[i];
                }
                else if (string.Equals(arg, "--ticks", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    if (int.TryParse(args[i], NumberStyles.Integer, CultureInfo.InvariantCulture, out int parsedTicks)
                        && parsedTicks > 0)
                    {
                        playTicks = parsedTicks;
                    }
                }
            }
        }

        private static int DispatchCliCommand(
            string projectRoot,
            ScriptCompilationOptions options,
            bool isGenerateIde,
            bool isDiagnose,
            bool isSymbols,
            bool isUsages,
            bool isAttachComponent,
            bool isPlay,
            bool isLoop,
            string? typeFilter,
            string? scriptTarget,
            string? prefabTarget,
            string? targetObjectName,
            string? usageQuery,
            bool enterPlayMode,
            string? playTarget,
            int playTicks,
            Dictionary<string, string> serializedFields,
            bool debuggingEnabled)
        {
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

            if (isUsages)
            {
                string query = scriptTarget ?? usageQuery ?? typeFilter ?? string.Empty;
                if (string.IsNullOrWhiteSpace(query))
                {
                    Console.Error.WriteLine("Missing --script <ScriptOrGuid> argument for 'usages'.");
                    return 1;
                }

                ContentUsageSearchResult usagesResult = CodeLoop.FindScriptUsages(projectRoot, query, options);
                Console.WriteLine(usagesResult.ToJson());
                return 0;
            }

            if (isAttachComponent)
            {
                if (string.IsNullOrWhiteSpace(prefabTarget) || string.IsNullOrWhiteSpace(scriptTarget))
                {
                    Console.Error.WriteLine("Missing --prefab <PrefabPath> or --script <ScriptOrGuid> for 'attach-component'.");
                    return 1;
                }

                PrefabComponentAttachment attachment = CodeLoop.AttachComponentToPrefab(
                    projectRoot,
                    prefabTarget!,
                    scriptTarget!,
                    targetObjectName,
                    serializedFields);
                Console.WriteLine(attachment.ToJson());
                return 0;
            }

            if (isPlay)
            {
                PlayModeExecutionReport playReport = CodeLoop.EnterPlayMode(
                    projectRoot,
                    playTarget ?? prefabTarget,
                    playTicks,
                    0.02,
                    options);
                Console.WriteLine(playReport.ToJson());
                return playReport.Succeeded ? 0 : 1;
            }

            if (isLoop)
            {
                CodeLoopReport loopReport = CodeLoop.Run(
                    projectRoot,
                    options,
                    typeFilter,
                    testRunner: null,
                    usageQuery: usageQuery ?? scriptTarget,
                    enterPlayMode: enterPlayMode,
                    playSceneOrPrefabPath: playTarget,
                    playTickCount: playTicks);
                Console.WriteLine(loopReport.ToJson());
                return loopReport.Succeeded ? 0 : 1;
            }

            ScriptCompilationResult result = ScriptCompilation.CompileProject(projectRoot, options, debuggingEnabled);
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
