using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json;

namespace Unitas.Scripting
{
    public static class AssemblyDiscovery
    {
        public const string AssemblyCSharpFirstPass = "Assembly-CSharp-firstpass";
        public const string AssemblyCSharpEditorFirstPass = "Assembly-CSharp-Editor-firstpass";
        public const string AssemblyCSharp = "Assembly-CSharp";
        public const string AssemblyCSharpEditor = "Assembly-CSharp-Editor";

        public static List<AssemblyDefinitionModel> DiscoverAssemblies(
            string projectRoot,
            Dictionary<string, string> guidToPath,
            Dictionary<string, string> pathToGuid,
            List<ScriptDiagnostic> diagnostics)
        {
            List<string> asmdefFiles = new List<string>();
            List<string> asmrefFiles = new List<string>();

            string assetsDir = Path.Combine(projectRoot, "Assets");
            string packagesDir = Path.Combine(projectRoot, "Packages");
            if (Directory.Exists(assetsDir))
            {
                CollectFilesByExtension(assetsDir, ".asmdef", asmdefFiles);
                CollectFilesByExtension(assetsDir, ".asmref", asmrefFiles);
            }

            if (Directory.Exists(packagesDir))
            {
                CollectFilesByExtension(packagesDir, ".asmdef", asmdefFiles);
                CollectFilesByExtension(packagesDir, ".asmref", asmrefFiles);
            }

            asmdefFiles.Sort(StringComparer.Ordinal);
            asmrefFiles.Sort(StringComparer.Ordinal);

            List<AssemblyDefinitionModel> models = new List<AssemblyDefinitionModel>();
            Dictionary<string, AssemblyDefinitionModel> byName =
                new Dictionary<string, AssemblyDefinitionModel>(StringComparer.Ordinal);
            Dictionary<string, AssemblyDefinitionModel> byGuid =
                new Dictionary<string, AssemblyDefinitionModel>(StringComparer.OrdinalIgnoreCase);

            for (int i = 0; i < asmdefFiles.Count; i++)
            {
                string asmdefPath = Path.GetFullPath(asmdefFiles[i]);
                AssemblyDefinitionModel? parsed = ParseAsmdefFile(asmdefPath, pathToGuid, diagnostics);
                if (parsed == null)
                {
                    continue;
                }

                if (byName.TryGetValue(parsed.Name, out AssemblyDefinitionModel? existing))
                {
                    diagnostics.Add(new ScriptDiagnostic(
                        "UNITAS1001",
                        ScriptDiagnosticSeverity.Error,
                        "Duplicate assembly definition name '" + parsed.Name + "' in '" + parsed.FilePath + "' and '" + existing.FilePath + "'.",
                        parsed.FilePath,
                        1,
                        1,
                        parsed.Name));
                    continue;
                }

                byName[parsed.Name] = parsed;
                if (!string.IsNullOrEmpty(parsed.Guid))
                {
                    byGuid[parsed.Guid] = parsed;
                    guidToPath[parsed.Guid] = parsed.FilePath;
                }

                models.Add(parsed);
            }

            for (int i = 0; i < asmrefFiles.Count; i++)
            {
                string asmrefPath = Path.GetFullPath(asmrefFiles[i]);
                string? referenceToken = ParseAsmrefFile(asmrefPath, diagnostics);
                if (string.IsNullOrEmpty(referenceToken))
                {
                    continue;
                }

                AssemblyDefinitionModel? target = ResolveReferenceToken(referenceToken, byName, byGuid);
                if (target == null)
                {
                    diagnostics.Add(new ScriptDiagnostic(
                        "UNITAS1002",
                        ScriptDiagnosticSeverity.Error,
                        "Assembly definition reference '" + asmrefPath + "' references unknown assembly '" + referenceToken + "'.",
                        asmrefPath,
                        1,
                        1,
                        string.Empty));
                    continue;
                }

                string? folder = Path.GetDirectoryName(asmrefPath);
                if (!string.IsNullOrEmpty(folder))
                {
                    target.AdditionalSourceFolders.Add(Path.GetFullPath(folder));
                }
            }

            return models;
        }

        public static Dictionary<string, List<string>> AssignScriptsToAssemblies(
            string projectRoot,
            List<AssemblyDefinitionModel> customAssemblies,
            out List<AssemblyDefinitionModel> predefinedAssemblies)
        {
            predefinedAssemblies = new List<AssemblyDefinitionModel>();
            Dictionary<string, List<string>> assigned =
                new Dictionary<string, List<string>>(StringComparer.Ordinal);

            List<(string Folder, AssemblyDefinitionModel Model)> folderClaims =
                new List<(string Folder, AssemblyDefinitionModel Model)>();

            for (int i = 0; i < customAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = customAssemblies[i];
                assigned[model.Name] = new List<string>();
                if (!string.IsNullOrEmpty(model.DirectoryPath))
                {
                    folderClaims.Add((NormalizeDirPrefix(model.DirectoryPath), model));
                }

                for (int j = 0; j < model.AdditionalSourceFolders.Count; j++)
                {
                    folderClaims.Add((NormalizeDirPrefix(model.AdditionalSourceFolders[j]), model));
                }
            }

            folderClaims.Sort((a, b) => b.Folder.Length.CompareTo(a.Folder.Length));

            List<string> allScripts = new List<string>();
            string assetsDir = Path.Combine(projectRoot, "Assets");
            string packagesDir = Path.Combine(projectRoot, "Packages");
            if (Directory.Exists(assetsDir))
            {
                CollectFilesByExtension(assetsDir, ".cs", allScripts);
            }

            if (Directory.Exists(packagesDir))
            {
                CollectFilesByExtension(packagesDir, ".cs", allScripts);
            }

            allScripts.Sort(StringComparer.Ordinal);

            List<string> firstPassScripts = new List<string>();
            List<string> editorFirstPassScripts = new List<string>();
            List<string> mainScripts = new List<string>();
            List<string> editorScripts = new List<string>();

            string normalizedAssetsPrefix = NormalizeDirPrefix(assetsDir);

            for (int i = 0; i < allScripts.Count; i++)
            {
                string scriptPath = Path.GetFullPath(allScripts[i]);
                string normalizedScriptDir = NormalizeDirPrefix(Path.GetDirectoryName(scriptPath) ?? string.Empty);

                AssemblyDefinitionModel? matchedCustom = null;
                for (int j = 0; j < folderClaims.Count; j++)
                {
                    if (normalizedScriptDir.StartsWith(folderClaims[j].Folder, StringComparison.Ordinal))
                    {
                        matchedCustom = folderClaims[j].Model;
                        break;
                    }
                }

                if (matchedCustom != null)
                {
                    assigned[matchedCustom.Name].Add(scriptPath);
                    continue;
                }

                if (!normalizedScriptDir.StartsWith(normalizedAssetsPrefix, StringComparison.Ordinal))
                {
                    continue;
                }

                string relativeInsideAssets = scriptPath.Substring(normalizedAssetsPrefix.Length).Replace('\\', '/');
                ClassifyPredefinedScript(
                    relativeInsideAssets,
                    scriptPath,
                    firstPassScripts,
                    editorFirstPassScripts,
                    mainScripts,
                    editorScripts);
            }

            List<string> autoReferencedNames = new List<string>();
            for (int i = 0; i < customAssemblies.Count; i++)
            {
                if (customAssemblies[i].AutoReferenced)
                {
                    autoReferencedNames.Add(customAssemblies[i].Name);
                }
            }

            if (firstPassScripts.Count > 0)
            {
                AssemblyDefinitionModel fp = CreatePredefinedModel(
                    AssemblyCSharpFirstPass,
                    projectRoot,
                    false,
                    new List<string>());
                predefinedAssemblies.Add(fp);
                assigned[fp.Name] = firstPassScripts;
            }

            if (editorFirstPassScripts.Count > 0)
            {
                List<string> refs = new List<string>();
                if (firstPassScripts.Count > 0)
                {
                    refs.Add(AssemblyCSharpFirstPass);
                }

                AssemblyDefinitionModel efp = CreatePredefinedModel(
                    AssemblyCSharpEditorFirstPass,
                    projectRoot,
                    true,
                    refs);
                predefinedAssemblies.Add(efp);
                assigned[efp.Name] = editorFirstPassScripts;
            }

            if (mainScripts.Count > 0)
            {
                List<string> refs = new List<string>(autoReferencedNames);
                if (firstPassScripts.Count > 0)
                {
                    refs.Add(AssemblyCSharpFirstPass);
                }

                AssemblyDefinitionModel main = CreatePredefinedModel(
                    AssemblyCSharp,
                    projectRoot,
                    false,
                    refs);
                predefinedAssemblies.Add(main);
                assigned[main.Name] = mainScripts;
            }

            if (editorScripts.Count > 0)
            {
                List<string> refs = new List<string>(autoReferencedNames);
                if (firstPassScripts.Count > 0)
                {
                    refs.Add(AssemblyCSharpFirstPass);
                }

                if (editorFirstPassScripts.Count > 0)
                {
                    refs.Add(AssemblyCSharpEditorFirstPass);
                }

                if (mainScripts.Count > 0)
                {
                    refs.Add(AssemblyCSharp);
                }

                AssemblyDefinitionModel ed = CreatePredefinedModel(
                    AssemblyCSharpEditor,
                    projectRoot,
                    true,
                    refs);
                predefinedAssemblies.Add(ed);
                assigned[ed.Name] = editorScripts;
            }

            return assigned;
        }

        public static AssemblyDefinitionModel? ResolveReferenceToken(
            string token,
            Dictionary<string, AssemblyDefinitionModel> byName,
            Dictionary<string, AssemblyDefinitionModel> byGuid)
        {
            string trimmed = token.Trim();
            if (trimmed.StartsWith("GUID:", StringComparison.OrdinalIgnoreCase))
            {
                string guid = trimmed.Substring(5).Trim();
                if (byGuid.TryGetValue(guid, out AssemblyDefinitionModel? byGuidMatch))
                {
                    return byGuidMatch;
                }

                return null;
            }

            if (byName.TryGetValue(trimmed, out AssemblyDefinitionModel? byNameMatch))
            {
                return byNameMatch;
            }

            if (byGuid.TryGetValue(trimmed, out AssemblyDefinitionModel? rawGuidMatch))
            {
                return rawGuidMatch;
            }

            return null;
        }

        public static bool EvaluatePlatformEligibility(
            AssemblyDefinitionModel model,
            ScriptCompilationOptions options,
            out string reason)
        {
            reason = string.Empty;
            if (model.IsEditorOnly && !options.IsEditor)
            {
                reason = "Editor-only predefined assembly excluded from player build.";
                return false;
            }

            string currentPlatformName = GetUnityPlatformToken(options.TargetPlatform);
            if (model.IncludePlatforms.Count > 0)
            {
                bool matched = false;
                for (int i = 0; i < model.IncludePlatforms.Count; i++)
                {
                    string inc = model.IncludePlatforms[i];
                    if (options.IsEditor && string.Equals(inc, "Editor", StringComparison.OrdinalIgnoreCase))
                    {
                        matched = true;
                        break;
                    }

                    if (!options.IsEditor && PlatformNameMatches(inc, currentPlatformName))
                    {
                        matched = true;
                        break;
                    }
                }

                if (!matched)
                {
                    reason = "Excluded by includePlatforms (" + string.Join(", ", model.IncludePlatforms) + ").";
                    return false;
                }
            }

            if (model.ExcludePlatforms.Count > 0)
            {
                for (int i = 0; i < model.ExcludePlatforms.Count; i++)
                {
                    string exc = model.ExcludePlatforms[i];
                    if (options.IsEditor && string.Equals(exc, "Editor", StringComparison.OrdinalIgnoreCase))
                    {
                        reason = "Excluded by excludePlatforms (Editor).";
                        return false;
                    }

                    if (!options.IsEditor && PlatformNameMatches(exc, currentPlatformName))
                    {
                        reason = "Excluded by excludePlatforms (" + exc + ").";
                        return false;
                    }
                }
            }

            return true;
        }

        public static bool EvaluateDefineConstraints(
            IReadOnlyList<string> constraints,
            HashSet<string> activeDefines,
            out string failedConstraint)
        {
            failedConstraint = string.Empty;
            if (constraints.Count == 0)
            {
                return true;
            }

            for (int i = 0; i < constraints.Count; i++)
            {
                string expr = constraints[i].Trim();
                if (string.IsNullOrEmpty(expr))
                {
                    continue;
                }

                string[] orParts = expr.Split("||", StringSplitOptions.RemoveEmptyEntries);
                bool anyOrTrue = false;
                for (int j = 0; j < orParts.Length; j++)
                {
                    string term = orParts[j].Trim();
                    bool termMatches = EvaluateSingleDefineTerm(term, activeDefines);
                    if (termMatches)
                    {
                        anyOrTrue = true;
                        break;
                    }
                }

                if (!anyOrTrue)
                {
                    failedConstraint = expr;
                    return false;
                }
            }

            return true;
        }

        public static void ApplyVersionDefines(
            AssemblyDefinitionModel model,
            ScriptCompilationOptions options,
            HashSet<string> defines,
            List<ScriptDiagnostic> diagnostics)
        {
            for (int i = 0; i < model.VersionDefines.Count; i++)
            {
                VersionDefineRule rule = model.VersionDefines[i];
                if (string.IsNullOrEmpty(rule.Define) || string.IsNullOrEmpty(rule.Expression))
                {
                    continue;
                }

                string? targetVersion = null;
                if (string.IsNullOrEmpty(rule.Name)
                    || string.Equals(rule.Name, "Unity", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(rule.Name, "com.unity.ugui", StringComparison.OrdinalIgnoreCase))
                {
                    if (!options.PackageVersions.TryGetValue(rule.Name, out targetVersion))
                    {
                        if (string.IsNullOrEmpty(rule.Name) || string.Equals(rule.Name, "Unity", StringComparison.OrdinalIgnoreCase))
                        {
                            targetVersion = options.UnityVersion;
                        }
                    }
                }
                else
                {
                    options.PackageVersions.TryGetValue(rule.Name, out targetVersion);
                }

                if (!TryEvaluateVersionExpression(rule.Expression, targetVersion, out bool matched, out string? error))
                {
                    diagnostics.Add(new ScriptDiagnostic(
                        "UNITAS1006",
                        ScriptDiagnosticSeverity.Error,
                        "Invalid versionDefines expression '" + rule.Expression + "' in '" + model.FilePath + "': " + error,
                        model.FilePath,
                        1,
                        1,
                        model.Name));
                    continue;
                }

                if (matched)
                {
                    defines.Add(rule.Define);
                }
            }
        }

        public static string GetUnityPlatformToken(ScriptTargetPlatform platform)
        {
            return platform switch
            {
                ScriptTargetPlatform.StandaloneOSX => "StandaloneOSX",
                ScriptTargetPlatform.StandaloneWindows64 => "StandaloneWindows64",
                ScriptTargetPlatform.StandaloneLinux64 => "StandaloneLinux64",
                ScriptTargetPlatform.DedicatedServer => "DedicatedServer",
                ScriptTargetPlatform.Android => "Android",
                ScriptTargetPlatform.iOS => "iOS",
                ScriptTargetPlatform.WebGL => "WebGL",
                _ => "StandaloneOSX"
            };
        }

        private static bool PlatformNameMatches(string candidate, string targetPlatform)
        {
            if (string.Equals(candidate, targetPlatform, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            if (string.Equals(candidate, "StandaloneOSXUniversal", StringComparison.OrdinalIgnoreCase)
                && string.Equals(targetPlatform, "StandaloneOSX", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            if (string.Equals(candidate, "StandaloneWindows", StringComparison.OrdinalIgnoreCase)
                && string.Equals(targetPlatform, "StandaloneWindows64", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            return false;
        }

        private static bool EvaluateSingleDefineTerm(string term, HashSet<string> activeDefines)
        {
            if (term.StartsWith('!'))
            {
                string symbol = term.Substring(1).Trim();
                return !activeDefines.Contains(symbol);
            }

            return activeDefines.Contains(term);
        }

        private static bool TryEvaluateVersionExpression(
            string expression,
            string? availableVersionText,
            out bool matched,
            out string? error)
        {
            matched = false;
            error = null;
            string trimmed = expression.Trim();
            if (string.IsNullOrEmpty(trimmed))
            {
                error = "Empty version expression.";
                return false;
            }

            if (trimmed[0] == '[' || trimmed[0] == '(')
            {
                char open = trimmed[0];
                char close = trimmed[trimmed.Length - 1];
                if (close != ']' && close != ')')
                {
                    error = "Mismatched range bracket.";
                    return false;
                }

                string inner = trimmed.Substring(1, trimmed.Length - 2).Trim();
                string[] parts = inner.Split(',');
                if (parts.Length == 1)
                {
                    if (open != '[' || close != ']')
                    {
                        error = "Single version range must use [x.y.z].";
                        return false;
                    }

                    if (!TryParseSemVer(parts[0].Trim(), out (int Major, int Minor, int Patch) exact))
                    {
                        error = "Invalid version number '" + parts[0].Trim() + "'.";
                        return false;
                    }

                    if (string.IsNullOrEmpty(availableVersionText)
                        || !TryParseSemVer(availableVersionText, out (int Major, int Minor, int Patch) actual))
                    {
                        matched = false;
                        return true;
                    }

                    matched = CompareSemVer(actual, exact) == 0;
                    return true;
                }

                if (parts.Length == 2)
                {
                    string minText = parts[0].Trim();
                    string maxText = parts[1].Trim();
                    (int Major, int Minor, int Patch)? minVer = null;
                    (int Major, int Minor, int Patch)? maxVer = null;

                    if (!string.IsNullOrEmpty(minText))
                    {
                        if (!TryParseSemVer(minText, out (int Major, int Minor, int Patch) parsedMin))
                        {
                            error = "Invalid lower bound '" + minText + "'.";
                            return false;
                        }

                        minVer = parsedMin;
                    }

                    if (!string.IsNullOrEmpty(maxText))
                    {
                        if (!TryParseSemVer(maxText, out (int Major, int Minor, int Patch) parsedMax))
                        {
                            error = "Invalid upper bound '" + maxText + "'.";
                            return false;
                        }

                        maxVer = parsedMax;
                    }

                    if (minVer == null && maxVer == null)
                    {
                        error = "Range must specify at least one bound.";
                        return false;
                    }

                    if (string.IsNullOrEmpty(availableVersionText)
                        || !TryParseSemVer(availableVersionText, out (int Major, int Minor, int Patch) actual))
                    {
                        matched = false;
                        return true;
                    }

                    if (minVer != null)
                    {
                        int cmp = CompareSemVer(actual, minVer.Value);
                        if (open == '[' ? cmp < 0 : cmp <= 0)
                        {
                            matched = false;
                            return true;
                        }
                    }

                    if (maxVer != null)
                    {
                        int cmp = CompareSemVer(actual, maxVer.Value);
                        if (close == ']' ? cmp > 0 : cmp >= 0)
                        {
                            matched = false;
                            return true;
                        }
                    }

                    matched = true;
                    return true;
                }

                error = "Too many comma-separated bounds in range.";
                return false;
            }

            if (!TryParseSemVer(trimmed, out (int Major, int Minor, int Patch) minimum))
            {
                error = "Invalid version number '" + trimmed + "'.";
                return false;
            }

            if (string.IsNullOrEmpty(availableVersionText)
                || !TryParseSemVer(availableVersionText, out (int Major, int Minor, int Patch) actualVer))
            {
                matched = false;
                return true;
            }

            matched = CompareSemVer(actualVer, minimum) >= 0;
            return true;
        }

        private static bool TryParseSemVer(string text, out (int Major, int Minor, int Patch) version)
        {
            version = (0, 0, 0);
            if (string.IsNullOrEmpty(text))
            {
                return false;
            }

            int suffixIndex = text.IndexOfAny(new[] { '-', 'f', 'a', 'b', 'p' });
            string core = suffixIndex > 0 ? text.Substring(0, suffixIndex) : text;
            string[] segments = core.Split('.');
            if (segments.Length < 1 || segments.Length > 3)
            {
                return false;
            }

            if (!int.TryParse(segments[0], NumberStyles.Integer, CultureInfo.InvariantCulture, out int major) || major < 0)
            {
                return false;
            }

            int minor = 0;
            if (segments.Length >= 2 && (!int.TryParse(segments[1], NumberStyles.Integer, CultureInfo.InvariantCulture, out minor) || minor < 0))
            {
                return false;
            }

            int patch = 0;
            if (segments.Length == 3 && (!int.TryParse(segments[2], NumberStyles.Integer, CultureInfo.InvariantCulture, out patch) || patch < 0))
            {
                return false;
            }

            version = (major, minor, patch);
            return true;
        }

        private static int CompareSemVer((int Major, int Minor, int Patch) a, (int Major, int Minor, int Patch) b)
        {
            int c = a.Major.CompareTo(b.Major);
            if (c != 0)
            {
                return c;
            }

            c = a.Minor.CompareTo(b.Minor);
            if (c != 0)
            {
                return c;
            }

            return a.Patch.CompareTo(b.Patch);
        }

        private static void ClassifyPredefinedScript(
            string relativeInsideAssets,
            string fullPath,
            List<string> firstPass,
            List<string> editorFirstPass,
            List<string> main,
            List<string> editor)
        {
            string[] parts = relativeInsideAssets.Split('/', StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length == 0)
            {
                return;
            }

            string topFolder = parts[0];
            bool isFirstPassRoot = string.Equals(topFolder, "Plugins", StringComparison.OrdinalIgnoreCase)
                || string.Equals(topFolder, "Standard Assets", StringComparison.OrdinalIgnoreCase)
                || string.Equals(topFolder, "Pro Standard Assets", StringComparison.OrdinalIgnoreCase);

            bool hasEditorFolder = false;
            for (int i = 0; i < parts.Length - 1; i++)
            {
                if (string.Equals(parts[i], "Editor", StringComparison.OrdinalIgnoreCase))
                {
                    hasEditorFolder = true;
                    break;
                }
            }

            if (isFirstPassRoot)
            {
                if (hasEditorFolder)
                {
                    editorFirstPass.Add(fullPath);
                }
                else
                {
                    firstPass.Add(fullPath);
                }
            }
            else
            {
                if (hasEditorFolder)
                {
                    editor.Add(fullPath);
                }
                else
                {
                    main.Add(fullPath);
                }
            }
        }

        private static AssemblyDefinitionModel CreatePredefinedModel(
            string name,
            string projectRoot,
            bool isEditorOnly,
            List<string> references)
        {
            AssemblyDefinitionModel model = new AssemblyDefinitionModel
            {
                Name = name,
                FilePath = Path.Combine(projectRoot, name + ".predefined"),
                DirectoryPath = Path.Combine(projectRoot, "Assets"),
                IsPredefined = true,
                IsEditorOnly = isEditorOnly,
                AutoReferenced = false
            };
            for (int i = 0; i < references.Count; i++)
            {
                model.References.Add(references[i]);
            }

            if (isEditorOnly)
            {
                model.IncludePlatforms.Add("Editor");
            }

            return model;
        }

        private static AssemblyDefinitionModel? ParseAsmdefFile(
            string asmdefPath,
            Dictionary<string, string> pathToGuid,
            List<ScriptDiagnostic> diagnostics)
        {
            string jsonText;
            try
            {
                jsonText = File.ReadAllText(asmdefPath);
            }
            catch (IOException ex)
            {
                diagnostics.Add(new ScriptDiagnostic(
                    "UNITAS1000",
                    ScriptDiagnosticSeverity.Error,
                    "Failed to read assembly definition '" + asmdefPath + "': " + ex.Message,
                    asmdefPath,
                    1,
                    1,
                    string.Empty));
                return null;
            }

            JsonDocument doc;
            try
            {
                doc = JsonDocument.Parse(jsonText);
            }
            catch (JsonException ex)
            {
                diagnostics.Add(new ScriptDiagnostic(
                    "UNITAS1000",
                    ScriptDiagnosticSeverity.Error,
                    "Invalid JSON in assembly definition '" + asmdefPath + "': " + ex.Message,
                    asmdefPath,
                    1,
                    1,
                    string.Empty));
                return null;
            }

            using (doc)
            {
                JsonElement root = doc.RootElement;
                string name = ReadString(root, "name");
                if (string.IsNullOrEmpty(name))
                {
                    diagnostics.Add(new ScriptDiagnostic(
                        "UNITAS1000",
                        ScriptDiagnosticSeverity.Error,
                        "Assembly definition '" + asmdefPath + "' is missing a 'name' property.",
                        asmdefPath,
                        1,
                        1,
                        string.Empty));
                    return null;
                }

                string guid = string.Empty;
                if (!pathToGuid.TryGetValue(asmdefPath, out guid!))
                {
                    guid = TryReadGuidFromMetaFile(asmdefPath + ".meta");
                }

                AssemblyDefinitionModel model = new AssemblyDefinitionModel
                {
                    Name = name,
                    RootNamespace = ReadString(root, "rootNamespace"),
                    FilePath = asmdefPath,
                    DirectoryPath = Path.GetDirectoryName(asmdefPath) ?? string.Empty,
                    Guid = guid ?? string.Empty,
                    AllowUnsafeCode = ReadBool(root, "allowUnsafeCode", false),
                    OverrideReferences = ReadBool(root, "overrideReferences", false),
                    AutoReferenced = ReadBool(root, "autoReferenced", true),
                    NoEngineReferences = ReadBool(root, "noEngineReferences", false)
                };

                ReadStringList(root, "references", model.References);
                ReadStringList(root, "includePlatforms", model.IncludePlatforms);
                ReadStringList(root, "excludePlatforms", model.ExcludePlatforms);
                ReadStringList(root, "precompiledReferences", model.PrecompiledReferences);
                ReadStringList(root, "defineConstraints", model.DefineConstraints);

                if (root.TryGetProperty("versionDefines", out JsonElement vdElem) && vdElem.ValueKind == JsonValueKind.Array)
                {
                    foreach (JsonElement item in vdElem.EnumerateArray())
                    {
                        if (item.ValueKind != JsonValueKind.Object)
                        {
                            continue;
                        }

                        string vdName = ReadString(item, "name");
                        string vdExpr = ReadString(item, "expression");
                        string vdDefine = ReadString(item, "define");
                        model.VersionDefines.Add(new VersionDefineRule(vdName, vdExpr, vdDefine));
                    }
                }

                return model;
            }
        }

        private static string? ParseAsmrefFile(string asmrefPath, List<ScriptDiagnostic> diagnostics)
        {
            try
            {
                using JsonDocument doc = JsonDocument.Parse(File.ReadAllText(asmrefPath));
                string reference = ReadString(doc.RootElement, "reference");
                if (string.IsNullOrEmpty(reference))
                {
                    diagnostics.Add(new ScriptDiagnostic(
                        "UNITAS1002",
                        ScriptDiagnosticSeverity.Error,
                        "Assembly definition reference '" + asmrefPath + "' is missing 'reference'.",
                        asmrefPath,
                        1,
                        1,
                        string.Empty));
                    return null;
                }

                return reference;
            }
            catch (Exception ex) when (ex is IOException || ex is JsonException)
            {
                diagnostics.Add(new ScriptDiagnostic(
                    "UNITAS1002",
                    ScriptDiagnosticSeverity.Error,
                    "Failed to parse assembly definition reference '" + asmrefPath + "': " + ex.Message,
                    asmrefPath,
                    1,
                    1,
                    string.Empty));
                return null;
            }
        }

        public static string TryReadGuidFromMetaFile(string metaPath)
        {
            if (!File.Exists(metaPath))
            {
                return string.Empty;
            }

            try
            {
                string[] lines = File.ReadAllLines(metaPath);
                for (int i = 0; i < lines.Length; i++)
                {
                    string trimmed = lines[i].Trim();
                    if (trimmed.StartsWith("guid:", StringComparison.Ordinal))
                    {
                        return trimmed.Substring(5).Trim();
                    }
                }
            }
            catch (IOException)
            {
            }

            return string.Empty;
        }

        private static string ReadString(JsonElement obj, string propName)
        {
            if (obj.TryGetProperty(propName, out JsonElement prop) && prop.ValueKind == JsonValueKind.String)
            {
                return prop.GetString() ?? string.Empty;
            }

            return string.Empty;
        }

        private static bool ReadBool(JsonElement obj, string propName, bool defaultValue)
        {
            if (obj.TryGetProperty(propName, out JsonElement prop))
            {
                if (prop.ValueKind == JsonValueKind.True)
                {
                    return true;
                }

                if (prop.ValueKind == JsonValueKind.False)
                {
                    return false;
                }
            }

            return defaultValue;
        }

        private static void ReadStringList(JsonElement obj, string propName, List<string> target)
        {
            if (!obj.TryGetProperty(propName, out JsonElement prop) || prop.ValueKind != JsonValueKind.Array)
            {
                return;
            }

            foreach (JsonElement item in prop.EnumerateArray())
            {
                if (item.ValueKind == JsonValueKind.String)
                {
                    string? val = item.GetString();
                    if (!string.IsNullOrEmpty(val))
                    {
                        target.Add(val);
                    }
                }
            }
        }

        private static void CollectFilesByExtension(string directory, string extension, List<string> results)
        {
            try
            {
                string[] files = Directory.GetFiles(directory, "*" + extension, SearchOption.AllDirectories);
                for (int i = 0; i < files.Length; i++)
                {
                    if (files[i].EndsWith(extension, StringComparison.OrdinalIgnoreCase))
                    {
                        results.Add(Path.GetFullPath(files[i]));
                    }
                }
            }
            catch (IOException)
            {
            }
        }

        private static string NormalizeDirPrefix(string dir)
        {
            if (string.IsNullOrEmpty(dir))
            {
                return string.Empty;
            }

            string full = Path.GetFullPath(dir).Replace('\\', '/');
            if (!full.EndsWith('/'))
            {
                full += "/";
            }

            return full;
        }
    }
}
