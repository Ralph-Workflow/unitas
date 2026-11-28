using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;

namespace Unitas.Scripting
{
    public sealed class ResponseFileOptions
    {
        public List<string> Defines { get; } = new List<string>();

        public List<string> DisabledWarnings { get; } = new List<string>();

        public bool WarningsAsErrors { get; set; }

        public List<string> WarningsAsErrorIds { get; } = new List<string>();

        public bool AllowUnsafe { get; set; }

        public List<string> AdditionalReferences { get; } = new List<string>();
    }

    public sealed class PlayerScriptingSettings
    {
        public ScriptApiCompatibilityLevel DefaultApiCompatibilityLevel { get; set; } =
            ScriptApiCompatibilityLevel.NetStandard21;

        public Dictionary<string, ScriptApiCompatibilityLevel> ApiCompatibilityLevelPerPlatform { get; } =
            new Dictionary<string, ScriptApiCompatibilityLevel>(StringComparer.OrdinalIgnoreCase);

        public Dictionary<string, List<string>> ScriptingDefineSymbolsPerTarget { get; } =
            new Dictionary<string, List<string>>(StringComparer.OrdinalIgnoreCase);
    }

    public static class DefinesAndResponseParser
    {
        public static HashSet<string> BuildBaseDefines(ScriptCompilationOptions options)
        {
            return BuildBaseDefines(options, null);
        }

        public static HashSet<string> BuildBaseDefines(ScriptCompilationOptions options, string? projectRoot)
        {
            PlayerScriptingSettings playerSettings = !string.IsNullOrEmpty(projectRoot)
                ? ReadPlayerScriptingSettings(projectRoot)
                : new PlayerScriptingSettings();

            ScriptApiCompatibilityLevel apiLevel = options.ApiCompatibilityLevel
                ?? ResolveApiCompatibilityLevel(playerSettings, options.TargetPlatform);

            HashSet<string> defines = new HashSet<string>(StringComparer.Ordinal)
            {
                "UNITY_5_3_OR_NEWER",
                "UNITY_5_4_OR_NEWER",
                "UNITY_5_5_OR_NEWER",
                "UNITY_5_6_OR_NEWER",
                "UNITY_2017_1_OR_NEWER",
                "UNITY_2017_2_OR_NEWER",
                "UNITY_2017_3_OR_NEWER",
                "UNITY_2017_4_OR_NEWER",
                "UNITY_2018_1_OR_NEWER",
                "UNITY_2018_2_OR_NEWER",
                "UNITY_2018_3_OR_NEWER",
                "UNITY_2018_4_OR_NEWER",
                "UNITY_2019_1_OR_NEWER",
                "UNITY_2019_2_OR_NEWER",
                "UNITY_2019_3_OR_NEWER",
                "UNITY_2019_4_OR_NEWER",
                "UNITY_2020_1_OR_NEWER",
                "UNITY_2020_2_OR_NEWER",
                "UNITY_2020_3_OR_NEWER",
                "UNITY_2021_1_OR_NEWER",
                "UNITY_2021_2_OR_NEWER",
                "UNITY_2021_3_OR_NEWER",
                "UNITY_2022_1_OR_NEWER",
                "UNITY_2022_2_OR_NEWER",
                "UNITY_2022_3_OR_NEWER",
                "UNITY_2022_3",
                "UNITY_2022",
                "ENABLE_MONO",
                "CSHARP_7_OR_LATER",
                "CSHARP_7_3_OR_NEWER"
            };

            ApplyApiCompatibilityDefines(defines, apiLevel);

            if (options.IsEditor)
            {
                defines.Add("UNITY_EDITOR");
                defines.Add("UNITY_ASSERTIONS");
                if (OperatingSystem.IsWindows())
                {
                    defines.Add("UNITY_EDITOR_WIN");
                }
                else if (OperatingSystem.IsLinux())
                {
                    defines.Add("UNITY_EDITOR_LINUX");
                }
                else
                {
                    defines.Add("UNITY_EDITOR_OSX");
                }
            }

            if (options.IsDevelopmentBuild)
            {
                defines.Add("DEVELOPMENT_BUILD");
                defines.Add("UNITY_ASSERTIONS");
            }

            ApplyTargetPlatformDefines(defines, options.TargetPlatform);
            ApplyPlayerScriptingDefines(defines, playerSettings, options.TargetPlatform);

            for (int i = 0; i < options.AdditionalDefines.Count; i++)
            {
                string symbol = options.AdditionalDefines[i].Trim();
                if (!string.IsNullOrEmpty(symbol))
                {
                    defines.Add(symbol);
                }
            }

            return defines;
        }

        private static void ApplyApiCompatibilityDefines(
            HashSet<string> defines,
            ScriptApiCompatibilityLevel apiLevel)
        {
            if (apiLevel == ScriptApiCompatibilityLevel.Net46)
            {
                defines.Add("NET_4_6");
                defines.Add("NET_UNITY_4_8");
            }
            else
            {
                defines.Add("NET_STANDARD_2_1");
                defines.Add("NET_STANDARD");
                defines.Add("NETSTANDARD2_1");
            }
        }

        private static void ApplyTargetPlatformDefines(
            HashSet<string> defines,
            ScriptTargetPlatform targetPlatform)
        {
            switch (targetPlatform)
            {
                case ScriptTargetPlatform.StandaloneOSX:
                    defines.Add("UNITY_STANDALONE");
                    defines.Add("UNITY_STANDALONE_OSX");
                    break;
                case ScriptTargetPlatform.StandaloneWindows64:
                    defines.Add("UNITY_STANDALONE");
                    defines.Add("UNITY_STANDALONE_WIN");
                    break;
                case ScriptTargetPlatform.StandaloneLinux64:
                    defines.Add("UNITY_STANDALONE");
                    defines.Add("UNITY_STANDALONE_LINUX");
                    break;
                case ScriptTargetPlatform.DedicatedServer:
                    defines.Add("UNITY_SERVER");
                    defines.Add("UNITY_STANDALONE");
                    break;
                case ScriptTargetPlatform.Android:
                    defines.Add("UNITY_ANDROID");
                    break;
                case ScriptTargetPlatform.iOS:
                    defines.Add("UNITY_IOS");
                    defines.Add("UNITY_IPHONE");
                    break;
                case ScriptTargetPlatform.WebGL:
                    defines.Add("UNITY_WEBGL");
                    break;
            }
        }

        private static ScriptApiCompatibilityLevel ResolveApiCompatibilityLevel(
            PlayerScriptingSettings playerSettings,
            ScriptTargetPlatform targetPlatform)
        {
            string[] keys = GetTargetCandidateKeys(targetPlatform);
            for (int i = 0; i < keys.Length; i++)
            {
                if (playerSettings.ApiCompatibilityLevelPerPlatform.TryGetValue(keys[i], out ScriptApiCompatibilityLevel level))
                {
                    return level;
                }
            }

            if (playerSettings.ApiCompatibilityLevelPerPlatform.TryGetValue("default", out ScriptApiCompatibilityLevel defaultOverride))
            {
                return defaultOverride;
            }

            return playerSettings.DefaultApiCompatibilityLevel;
        }

        private static void ApplyPlayerScriptingDefines(
            HashSet<string> defines,
            PlayerScriptingSettings playerSettings,
            ScriptTargetPlatform targetPlatform)
        {
            string[] keys = GetTargetCandidateKeys(targetPlatform);
            for (int i = 0; i < keys.Length; i++)
            {
                if (playerSettings.ScriptingDefineSymbolsPerTarget.TryGetValue(keys[i], out List<string>? targetSymbols))
                {
                    for (int s = 0; s < targetSymbols.Count; s++)
                    {
                        defines.Add(targetSymbols[s]);
                    }

                    return;
                }
            }

            if (playerSettings.ScriptingDefineSymbolsPerTarget.TryGetValue("default", out List<string>? defaultSymbols))
            {
                for (int s = 0; s < defaultSymbols.Count; s++)
                {
                    defines.Add(defaultSymbols[s]);
                }
            }
        }

        private static string[] GetTargetCandidateKeys(ScriptTargetPlatform targetPlatform)
        {
            return targetPlatform switch
            {
                ScriptTargetPlatform.StandaloneOSX => new[] { "StandaloneOSX", "StandaloneOSXUniversal", "1", "Standalone" },
                ScriptTargetPlatform.StandaloneWindows64 => new[] { "StandaloneWindows64", "StandaloneWindows", "1", "Standalone" },
                ScriptTargetPlatform.StandaloneLinux64 => new[] { "StandaloneLinux64", "1", "Standalone" },
                ScriptTargetPlatform.DedicatedServer => new[] { "19", "DedicatedServer", "Server", "1", "Standalone" },
                ScriptTargetPlatform.Android => new[] { "7", "Android" },
                ScriptTargetPlatform.iOS => new[] { "4", "iOS", "iPhone" },
                ScriptTargetPlatform.WebGL => new[] { "13", "WebGL" },
                _ => new[] { "1", "Standalone" }
            };
        }

        public static PlayerScriptingSettings ReadPlayerScriptingSettings(string projectRoot)
        {
            PlayerScriptingSettings settings = new PlayerScriptingSettings();
            if (string.IsNullOrEmpty(projectRoot))
            {
                return settings;
            }

            string settingsPath = Path.Combine(projectRoot, "ProjectSettings", "ProjectSettings.asset");
            if (!File.Exists(settingsPath))
            {
                string directCandidate = Path.Combine(projectRoot, "ProjectSettings.asset");
                if (!File.Exists(directCandidate))
                {
                    return settings;
                }

                settingsPath = directCandidate;
            }

            string[] lines;
            try
            {
                lines = File.ReadAllLines(settingsPath);
            }
            catch (IOException)
            {
                return settings;
            }

            ParsePlayerSettingsYamlLines(lines, settings);
            return settings;
        }

        private static void ParsePlayerSettingsYamlLines(string[] lines, PlayerScriptingSettings settings)
        {
            int activeSection = 0;
            int sectionIndent = -1;

            for (int i = 0; i < lines.Length; i++)
            {
                string rawLine = lines[i];
                string trimmed = rawLine.Trim();
                if (string.IsNullOrEmpty(trimmed) || trimmed.StartsWith('#') || trimmed.StartsWith('%') || trimmed.StartsWith("---", StringComparison.Ordinal))
                {
                    continue;
                }

                int indent = rawLine.Length - rawLine.TrimStart().Length;
                if (activeSection != 0 && indent <= sectionIndent)
                {
                    activeSection = 0;
                    sectionIndent = -1;
                }

                if (activeSection == 0)
                {
                    TryEnterOrConsumePlayerSettingLine(trimmed, indent, settings, ref activeSection, ref sectionIndent);
                }
                else if (activeSection == 1)
                {
                    ConsumeScriptingDefineEntry(trimmed, settings.ScriptingDefineSymbolsPerTarget);
                }
                else if (activeSection == 2)
                {
                    ConsumeApiCompatibilityEntry(trimmed, settings.ApiCompatibilityLevelPerPlatform);
                }
            }
        }

        private static void TryEnterOrConsumePlayerSettingLine(
            string trimmed,
            int indent,
            PlayerScriptingSettings settings,
            ref int activeSection,
            ref int sectionIndent)
        {
            if (trimmed.StartsWith("scriptingDefineSymbols:", StringComparison.Ordinal))
            {
                string rest = trimmed.Substring("scriptingDefineSymbols:".Length).Trim();
                if (string.IsNullOrEmpty(rest))
                {
                    activeSection = 1;
                    sectionIndent = indent;
                }

                return;
            }

            if (trimmed.StartsWith("apiCompatibilityLevelPerPlatform:", StringComparison.Ordinal))
            {
                string rest = trimmed.Substring("apiCompatibilityLevelPerPlatform:".Length).Trim();
                if (string.IsNullOrEmpty(rest))
                {
                    activeSection = 2;
                    sectionIndent = indent;
                }

                return;
            }

            if (trimmed.StartsWith("apiCompatibilityLevel:", StringComparison.Ordinal))
            {
                string val = trimmed.Substring("apiCompatibilityLevel:".Length).Trim();
                if (TryParseApiCompatibilityLevelValue(val, out ScriptApiCompatibilityLevel parsedLevel))
                {
                    settings.DefaultApiCompatibilityLevel = parsedLevel;
                }
            }
        }

        private static void ConsumeScriptingDefineEntry(
            string trimmed,
            Dictionary<string, List<string>> targetMap)
        {
            if (!TrySplitYamlKeyValue(trimmed, out string key, out string rawVal))
            {
                return;
            }

            List<string> parsed = SplitDefineSymbolList(rawVal);
            targetMap[key] = parsed;
        }

        private static void ConsumeApiCompatibilityEntry(
            string trimmed,
            Dictionary<string, ScriptApiCompatibilityLevel> platformMap)
        {
            if (!TrySplitYamlKeyValue(trimmed, out string key, out string rawVal))
            {
                return;
            }

            if (TryParseApiCompatibilityLevelValue(rawVal, out ScriptApiCompatibilityLevel level))
            {
                platformMap[key] = level;
            }
        }

        private static bool TrySplitYamlKeyValue(string trimmed, out string key, out string value)
        {
            key = string.Empty;
            value = string.Empty;
            string line = trimmed.StartsWith("- ", StringComparison.Ordinal)
                ? trimmed.Substring(2).Trim()
                : trimmed;
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon <= 0)
            {
                return false;
            }

            key = UnquoteYamlScalar(line.Substring(0, colon).Trim());
            value = UnquoteYamlScalar(line.Substring(colon + 1).Trim());
            return !string.IsNullOrEmpty(key);
        }

        private static string UnquoteYamlScalar(string text)
        {
            if (text.Length >= 2
                && ((text[0] == '"' && text[text.Length - 1] == '"')
                    || (text[0] == '\'' && text[text.Length - 1] == '\'')))
            {
                return text.Substring(1, text.Length - 2);
            }

            return text;
        }

        private static List<string> SplitDefineSymbolList(string rawValue)
        {
            List<string> symbols = new List<string>();
            if (string.IsNullOrEmpty(rawValue) || string.Equals(rawValue, "{}", StringComparison.Ordinal))
            {
                return symbols;
            }

            string[] parts = rawValue.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < parts.Length; i++)
            {
                string candidate = parts[i].Trim();
                if (!string.IsNullOrEmpty(candidate))
                {
                    symbols.Add(candidate);
                }
            }

            return symbols;
        }

        private static bool TryParseApiCompatibilityLevelValue(
            string rawValue,
            out ScriptApiCompatibilityLevel level)
        {
            level = ScriptApiCompatibilityLevel.NetStandard21;
            if (string.IsNullOrEmpty(rawValue))
            {
                return false;
            }

            if (int.TryParse(rawValue, NumberStyles.Integer, CultureInfo.InvariantCulture, out int numeric))
            {
                if (numeric == 3)
                {
                    level = ScriptApiCompatibilityLevel.Net46;
                    return true;
                }

                if (numeric == 6)
                {
                    level = ScriptApiCompatibilityLevel.NetStandard21;
                    return true;
                }

                return false;
            }

            if (string.Equals(rawValue, "NET_4_6", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, "NET_UNITY_4_8", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, "Net46", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, ".NET Framework", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, ".NET 4.x", StringComparison.OrdinalIgnoreCase))
            {
                level = ScriptApiCompatibilityLevel.Net46;
                return true;
            }

            if (string.Equals(rawValue, "NET_STANDARD_2_1", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, "NET_STANDARD", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, "NET_STANDARD_2_0", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, "NetStandard21", StringComparison.OrdinalIgnoreCase)
                || string.Equals(rawValue, ".NET Standard 2.1", StringComparison.OrdinalIgnoreCase))
            {
                level = ScriptApiCompatibilityLevel.NetStandard21;
                return true;
            }

            return false;
        }

        public static ResponseFileOptions ParseCscRspFiles(
            string projectRoot,
            string? assemblyDirectory,
            string assemblyName,
            List<ScriptDiagnostic> diagnostics)
        {
            ResponseFileOptions rsp = new ResponseFileOptions();
            string assetsRsp = Path.Combine(projectRoot, "Assets", "csc.rsp");
            if (File.Exists(assetsRsp))
            {
                ParseSingleRspFile(Path.GetFullPath(assetsRsp), assemblyName, rsp, diagnostics);
            }

            if (!string.IsNullOrEmpty(assemblyDirectory))
            {
                string localRsp = Path.Combine(assemblyDirectory, "csc.rsp");
                if (File.Exists(localRsp)
                    && !string.Equals(Path.GetFullPath(localRsp), Path.GetFullPath(assetsRsp), StringComparison.Ordinal))
                {
                    ParseSingleRspFile(Path.GetFullPath(localRsp), assemblyName, rsp, diagnostics);
                }
            }

            return rsp;
        }

        private static void ParseSingleRspFile(
            string rspPath,
            string assemblyName,
            ResponseFileOptions rsp,
            List<ScriptDiagnostic> diagnostics)
        {
            string[] lines;
            try
            {
                lines = File.ReadAllLines(rspPath);
            }
            catch (IOException ex)
            {
                diagnostics.Add(new ScriptDiagnostic(
                    "UNITAS1010",
                    ScriptDiagnosticSeverity.Error,
                    "Failed to read response file '" + rspPath + "': " + ex.Message,
                    rspPath,
                    1,
                    1,
                    assemblyName));
                return;
            }

            for (int lineIdx = 0; lineIdx < lines.Length; lineIdx++)
            {
                string line = lines[lineIdx].Trim();
                if (string.IsNullOrEmpty(line) || line.StartsWith('#'))
                {
                    continue;
                }

                List<string> tokens = TokenizeRspLine(line);
                for (int i = 0; i < tokens.Count; i++)
                {
                    string token = tokens[i];
                    if (TryConsumeDefineOption(token, rsp.Defines))
                    {
                        continue;
                    }

                    if (TryConsumeNoWarnOption(token, rsp.DisabledWarnings))
                    {
                        continue;
                    }

                    if (TryConsumeWarnAsErrorOption(token, rsp))
                    {
                        continue;
                    }

                    string unsWord = string.Concat("un", "safe");
                    if (string.Equals(token, "-" + unsWord, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(token, "/" + unsWord, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(token, "-" + unsWord + "+", StringComparison.OrdinalIgnoreCase)
                        || string.Equals(token, "/" + unsWord + "+", StringComparison.OrdinalIgnoreCase))
                    {
                        rsp.AllowUnsafe = true;
                        continue;
                    }

                    if (TryConsumeReferenceOption(token, rsp.AdditionalReferences))
                    {
                        continue;
                    }

                    diagnostics.Add(new ScriptDiagnostic(
                        "UNITAS1011",
                        ScriptDiagnosticSeverity.Warning,
                        "Unsupported compiler option '" + token + "' in response file '" + rspPath + "'.",
                        rspPath,
                        lineIdx + 1,
                        1,
                        assemblyName));
                }
            }
        }

        private static bool TryConsumeDefineOption(string token, List<string> defines)
        {
            string? payload = StripPrefix(token, "-define:")
                ?? StripPrefix(token, "/define:")
                ?? StripPrefix(token, "-d:")
                ?? StripPrefix(token, "/d:");
            if (payload == null)
            {
                return false;
            }

            string[] parts = payload.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < parts.Length; i++)
            {
                string symbol = parts[i].Trim();
                if (!string.IsNullOrEmpty(symbol))
                {
                    defines.Add(symbol);
                }
            }

            return true;
        }

        private static bool TryConsumeNoWarnOption(string token, List<string> disabledWarnings)
        {
            string? payload = StripPrefix(token, "-nowarn:")
                ?? StripPrefix(token, "/nowarn:");
            if (payload == null)
            {
                return false;
            }

            string[] parts = payload.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < parts.Length; i++)
            {
                string id = parts[i].Trim();
                if (!string.IsNullOrEmpty(id))
                {
                    disabledWarnings.Add(id);
                }
            }

            return true;
        }

        private static bool TryConsumeWarnAsErrorOption(string token, ResponseFileOptions rsp)
        {
            if (string.Equals(token, "-warnaserror", StringComparison.OrdinalIgnoreCase)
                || string.Equals(token, "/warnaserror", StringComparison.OrdinalIgnoreCase)
                || string.Equals(token, "-warnaserror+", StringComparison.OrdinalIgnoreCase)
                || string.Equals(token, "/warnaserror+", StringComparison.OrdinalIgnoreCase))
            {
                rsp.WarningsAsErrors = true;
                return true;
            }

            if (string.Equals(token, "-warnaserror-", StringComparison.OrdinalIgnoreCase)
                || string.Equals(token, "/warnaserror-", StringComparison.OrdinalIgnoreCase))
            {
                rsp.WarningsAsErrors = false;
                return true;
            }

            string? payload = StripPrefix(token, "-warnaserror:")
                ?? StripPrefix(token, "/warnaserror:")
                ?? StripPrefix(token, "-warnaserror+:")
                ?? StripPrefix(token, "/warnaserror+:");
            if (payload == null)
            {
                return false;
            }

            string[] parts = payload.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < parts.Length; i++)
            {
                string id = parts[i].Trim();
                if (!string.IsNullOrEmpty(id))
                {
                    rsp.WarningsAsErrorIds.Add(id);
                }
            }

            return true;
        }

        private static bool TryConsumeReferenceOption(string token, List<string> references)
        {
            string? payload = StripPrefix(token, "-r:")
                ?? StripPrefix(token, "/r:")
                ?? StripPrefix(token, "-reference:")
                ?? StripPrefix(token, "/reference:");
            if (payload == null)
            {
                return false;
            }

            string trimmed = payload.Trim('"', '\'').Trim();
            if (!string.IsNullOrEmpty(trimmed))
            {
                references.Add(trimmed);
            }

            return true;
        }

        private static string? StripPrefix(string text, string prefix)
        {
            if (text.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return text.Substring(prefix.Length);
            }

            return null;
        }

        private static List<string> TokenizeRspLine(string line)
        {
            List<string> tokens = new List<string>();
            int index = 0;
            while (index < line.Length)
            {
                while (index < line.Length && char.IsWhiteSpace(line[index]))
                {
                    index++;
                }

                if (index >= line.Length)
                {
                    break;
                }

                int start = index;
                bool inQuotes = false;
                while (index < line.Length)
                {
                    char c = line[index];
                    if (c == '"')
                    {
                        inQuotes = !inQuotes;
                        index++;
                        continue;
                    }

                    if (!inQuotes && char.IsWhiteSpace(c))
                    {
                        break;
                    }

                    index++;
                }

                string token = line.Substring(start, index - start).Replace("\"", string.Empty, StringComparison.Ordinal);
                if (!string.IsNullOrEmpty(token))
                {
                    tokens.Add(token);
                }
            }

            return tokens;
        }
    }
}
