using System;
using System.Collections.Generic;
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

    public static class DefinesAndResponseParser
    {
        public static HashSet<string> BuildBaseDefines(ScriptCompilationOptions options)
        {
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
                "NET_STANDARD_2_1",
                "NET_STANDARD",
                "NETSTANDARD2_1",
                "ENABLE_MONO",
                "CSHARP_7_OR_LATER",
                "CSHARP_7_3_OR_NEWER"
            };

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

            switch (options.TargetPlatform)
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
