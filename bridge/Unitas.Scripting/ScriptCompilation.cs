using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Emit;
using Unitas.Bridge.Scripting;

namespace Unitas.Scripting
{
    public static class RoslynAssemblyResolver
    {
        private static bool _installed;
        private static string? _bincoreDir;

        [ModuleInitializer]
        public static void EnsureInstalled()
        {
            if (_installed)
            {
                return;
            }

            _installed = true;
            AssemblyLoadContext.Default.Resolving += (context, name) =>
            {
                string simple = name.Name ?? string.Empty;
                if (!simple.StartsWith("Microsoft.CodeAnalysis", StringComparison.OrdinalIgnoreCase))
                {
                    return null;
                }

                string? bincore = GetBincoreDir();
                if (!string.IsNullOrEmpty(bincore))
                {
                    string candidate = Path.Combine(bincore, simple + ".dll");
                    if (File.Exists(candidate))
                    {
                        return context.LoadFromAssemblyPath(candidate);
                    }
                }

                return null;
            };
        }

        private static string? GetBincoreDir()
        {
            if (_bincoreDir != null)
            {
                return _bincoreDir;
            }

            List<string> roots = new List<string>();
            string? envRoot = Environment.GetEnvironmentVariable("DOTNET_ROOT");
            if (!string.IsNullOrEmpty(envRoot))
            {
                roots.Add(envRoot);
            }

            string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            if (!string.IsNullOrEmpty(home))
            {
                roots.Add(Path.Combine(home, ".dotnet"));
            }

            roots.Add("/Users/mistlight/.dotnet");
            roots.Add("/usr/local/share/dotnet");
            roots.Add("/opt/homebrew/share/dotnet");

            string? runtimeDir = Path.GetDirectoryName(typeof(string).Assembly.Location);
            if (!string.IsNullOrEmpty(runtimeDir))
            {
                DirectoryInfo? dir = new DirectoryInfo(runtimeDir);
                if (dir.Parent?.Parent?.Parent != null)
                {
                    roots.Add(dir.Parent.Parent.Parent.FullName);
                }
            }

            for (int i = 0; i < roots.Count; i++)
            {
                string sdkDir = Path.Combine(roots[i], "sdk");
                if (!Directory.Exists(sdkDir))
                {
                    continue;
                }

                string[] versions = Directory.GetDirectories(sdkDir);
                Array.Sort(versions, StringComparer.Ordinal);
                for (int s = versions.Length - 1; s >= 0; s--)
                {
                    string bincore = Path.Combine(versions[s], "Roslyn", "bincore");
                    if (File.Exists(Path.Combine(bincore, "Microsoft.CodeAnalysis.CSharp.dll")))
                    {
                        _bincoreDir = bincore;
                        return bincore;
                    }
                }
            }

            _bincoreDir = string.Empty;
            return _bincoreDir;
        }
    }

    public static class ScriptCompilation
    {
        public const string DefaultOutputFolderName = "UnitasScriptAssemblies";
        public const string GeneratedGlueAssemblyName = "Unitas.Generated";
        private const string ManifestFileName = ".unitas_compile_state.txt";

        private static readonly Regex CscDiagnosticRegex = new Regex(
            @"^(?:(?<file>.+?)\((?<line>\d+),(?<col>\d+)\)|(?<file2>[^:]+?))\s*:\s*(?<sev>error|warning|info)\s+(?<id>[A-Za-z]+\d+)\s*:\s*(?<msg>.+)$",
            RegexOptions.Compiled);

        private static string? _cachedDotnetExecutable;
        private static string? _cachedCscDllPath;
        private static IReadOnlyList<string>? _cachedNetReferenceDlls;
        private static readonly Dictionary<string, ScriptCompilationResult> LastValidResultByOutputDir =
            new Dictionary<string, ScriptCompilationResult>(StringComparer.Ordinal);
        private static readonly object LastValidGate = new object();

        static ScriptCompilation()
        {
            RoslynAssemblyResolver.EnsureInstalled();
        }

        public static DiagnoseReport DiagnoseProject(string projectRoot, ScriptCompilationOptions? options = null)
        {
            ScriptCompilationResult result = CompileProject(projectRoot, options);
            return DiagnoseReport.FromCompilationResult(result);
        }

        public static ScriptCompilationResult? GetLastValidCompilation(string projectRootOrOutputDirectory)
        {
            if (string.IsNullOrEmpty(projectRootOrOutputDirectory))
            {
                return null;
            }

            string fullPath = Path.GetFullPath(projectRootOrOutputDirectory);
            string candidateOutDir = Directory.Exists(Path.Combine(fullPath, "Library", DefaultOutputFolderName))
                ? Path.Combine(fullPath, "Library", DefaultOutputFolderName)
                : fullPath;

            lock (LastValidGate)
            {
                if (LastValidResultByOutputDir.TryGetValue(candidateOutDir, out ScriptCompilationResult? cached))
                {
                    return cached;
                }

                if (LastValidResultByOutputDir.TryGetValue(fullPath, out ScriptCompilationResult? cachedDirect))
                {
                    return cachedDirect;
                }
            }

            List<string> existingDlls = GetExistingDllsInDirectory(candidateOutDir);
            if (existingDlls.Count == 0)
            {
                return null;
            }

            string candidateGlue = Path.Combine(candidateOutDir, GeneratedGlueAssemblyName + ".dll");
            string? existingGlue = File.Exists(candidateGlue) ? candidateGlue : null;
            string candidateGlueSrc = Path.Combine(candidateOutDir, "obj", "Unitas.Generated.g.cs");
            string? existingGlueSrc = File.Exists(candidateGlueSrc) ? candidateGlueSrc : null;

            return new ScriptCompilationResult(
                true,
                ActiveAssemblyState.Current,
                candidateOutDir,
                existingGlue,
                existingGlueSrc,
                Array.Empty<CompiledAssemblyRecord>(),
                Array.Empty<ScriptDiagnostic>(),
                Array.Empty<string>(),
                existingDlls,
                Array.Empty<ScriptGuidBinding>(),
                Array.Empty<DiscoveredPluginRecord>());
        }

        public static ScriptCompilationResult CompileProject(string projectRoot, ScriptCompilationOptions? options = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            ScriptCompilationOptions effectiveOptions = options ?? new ScriptCompilationOptions();
            string outputDirectory = !string.IsNullOrEmpty(effectiveOptions.OutputDirectory)
                ? Path.GetFullPath(effectiveOptions.OutputDirectory)
                : Path.Combine(fullProjectRoot, "Library", DefaultOutputFolderName);

            Directory.CreateDirectory(outputDirectory);

            List<ScriptDiagnostic> diagnostics = new List<ScriptDiagnostic>();

            if (!effectiveOptions.IsProjectTrusted)
            {
                diagnostics.Add(new ScriptDiagnostic(
                    "UNITAS0001",
                    ScriptDiagnosticSeverity.Error,
                    "Project is untrusted; script compilation, analyzers, and source generators were not run.",
                    fullProjectRoot,
                    0,
                    0,
                    string.Empty));
                return BuildFailedResult(outputDirectory, new List<CompiledAssemblyRecord>(), diagnostics);
            }

            Dictionary<string, string> guidToPath = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            Dictionary<string, string> pathToGuid = new Dictionary<string, string>(StringComparer.Ordinal);
            List<ParsedPluginMeta> parsedPlugins = new List<ParsedPluginMeta>();

            ProjectMetadataIndex.ScanProjectMeta(fullProjectRoot, guidToPath, pathToGuid, parsedPlugins);

            HashSet<string> baseDefines = DefinesAndResponseParser.BuildBaseDefines(effectiveOptions);

            List<DiscoveredPluginRecord> pluginRecords = ProjectMetadataIndex.EvaluatePlugins(
                parsedPlugins,
                effectiveOptions,
                baseDefines,
                out Dictionary<string, DiscoveredPluginRecord> nativePluginsByName,
                out List<string> autoReferencedManagedDlls,
                out Dictionary<string, string> allCompatibleManagedDllsByFileName);

            List<AssemblyDefinitionModel> customAssemblies = AssemblyDiscovery.DiscoverAssemblies(
                fullProjectRoot,
                guidToPath,
                pathToGuid,
                diagnostics);

            Dictionary<string, List<string>> scriptsByAssembly = AssemblyDiscovery.AssignScriptsToAssemblies(
                fullProjectRoot,
                customAssemblies,
                out List<AssemblyDefinitionModel> predefinedAssemblies);

            List<AssemblyDefinitionModel> allAssemblies = new List<AssemblyDefinitionModel>();
            allAssemblies.AddRange(customAssemblies);
            allAssemblies.AddRange(predefinedAssemblies);

            EvaluateAssembliesAndDependencies(
                fullProjectRoot,
                allAssemblies,
                baseDefines,
                effectiveOptions,
                diagnostics,
                out Dictionary<string, AssemblyDefinitionModel> modelsByName,
                out Dictionary<string, HashSet<string>> assemblyDefinesMap,
                out Dictionary<string, ResponseFileOptions> assemblyRspMap,
                out Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap,
                out Dictionary<string, List<string>> resolvedDeps);

            List<string> topoOrder = TopologicalSortAndDetectCycles(allAssemblies, resolvedDeps, modelsByName, diagnostics);

            ValidateAssemblyDllImports(
                allAssemblies,
                exclusionMap,
                scriptsByAssembly,
                assemblyDefinesMap,
                nativePluginsByName,
                effectiveOptions,
                diagnostics);

            if (HasErrorDiagnostics(diagnostics))
            {
                List<CompiledAssemblyRecord> failedRecords = BuildRecordsWithoutCompile(
                    allAssemblies,
                    outputDirectory,
                    scriptsByAssembly,
                    assemblyDefinesMap,
                    resolvedDeps,
                    exclusionMap);
                return BuildFailedResult(outputDirectory, failedRecords, diagnostics);
            }

            return ExecuteStagedCompilation(
                outputDirectory,
                effectiveOptions,
                topoOrder,
                modelsByName,
                scriptsByAssembly,
                assemblyDefinesMap,
                assemblyRspMap,
                exclusionMap,
                resolvedDeps,
                autoReferencedManagedDlls,
                allCompatibleManagedDllsByFileName,
                pathToGuid,
                pluginRecords,
                diagnostics);
        }

        private static void EvaluateAssembliesAndDependencies(
            string fullProjectRoot,
            List<AssemblyDefinitionModel> allAssemblies,
            HashSet<string> baseDefines,
            ScriptCompilationOptions effectiveOptions,
            List<ScriptDiagnostic> diagnostics,
            out Dictionary<string, AssemblyDefinitionModel> modelsByName,
            out Dictionary<string, HashSet<string>> assemblyDefinesMap,
            out Dictionary<string, ResponseFileOptions> assemblyRspMap,
            out Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap,
            out Dictionary<string, List<string>> resolvedDeps)
        {
            modelsByName = new Dictionary<string, AssemblyDefinitionModel>(StringComparer.Ordinal);
            Dictionary<string, AssemblyDefinitionModel> modelsByGuid =
                new Dictionary<string, AssemblyDefinitionModel>(StringComparer.OrdinalIgnoreCase);

            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                modelsByName[model.Name] = model;
                if (!string.IsNullOrEmpty(model.Guid))
                {
                    modelsByGuid[model.Guid] = model;
                }
            }

            assemblyDefinesMap = new Dictionary<string, HashSet<string>>(StringComparer.Ordinal);
            assemblyRspMap = new Dictionary<string, ResponseFileOptions>(StringComparer.Ordinal);
            exclusionMap = new Dictionary<string, (bool IsExcluded, string Reason)>(StringComparer.Ordinal);

            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                HashSet<string> asmDefines = new HashSet<string>(baseDefines, StringComparer.Ordinal);
                AssemblyDiscovery.ApplyVersionDefines(model, effectiveOptions, asmDefines, diagnostics);

                ResponseFileOptions rsp = DefinesAndResponseParser.ParseCscRspFiles(
                    fullProjectRoot,
                    model.IsPredefined ? null : model.DirectoryPath,
                    model.Name,
                    diagnostics);

                for (int d = 0; d < rsp.Defines.Count; d++)
                {
                    asmDefines.Add(rsp.Defines[d]);
                }

                assemblyDefinesMap[model.Name] = asmDefines;
                assemblyRspMap[model.Name] = rsp;

                if (!AssemblyDiscovery.EvaluatePlatformEligibility(model, effectiveOptions, out string platReason))
                {
                    exclusionMap[model.Name] = (true, platReason);
                    continue;
                }

                if (!AssemblyDiscovery.EvaluateDefineConstraints(model.DefineConstraints, asmDefines, out string failedConstraint))
                {
                    exclusionMap[model.Name] = (true, "Excluded by defineConstraints (" + failedConstraint + ").");
                    continue;
                }

                exclusionMap[model.Name] = (false, string.Empty);
            }

            resolvedDeps = new Dictionary<string, List<string>>(StringComparer.Ordinal);
            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                List<string> deps = new List<string>();
                resolvedDeps[model.Name] = deps;

                if (exclusionMap[model.Name].IsExcluded)
                {
                    continue;
                }

                for (int r = 0; r < model.References.Count; r++)
                {
                    string refToken = model.References[r];
                    AssemblyDefinitionModel? target = AssemblyDiscovery.ResolveReferenceToken(refToken, modelsByName, modelsByGuid);
                    if (target == null)
                    {
                        diagnostics.Add(new ScriptDiagnostic(
                            "UNITAS1003",
                            ScriptDiagnosticSeverity.Error,
                            "Assembly '" + model.Name + "' references missing assembly '" + refToken + "'.",
                            model.FilePath,
                            1,
                            1,
                            model.Name));
                        continue;
                    }

                    if (exclusionMap.TryGetValue(target.Name, out (bool IsExcluded, string Reason) targetEx) && targetEx.IsExcluded)
                    {
                        if (!model.IsPredefined)
                        {
                            diagnostics.Add(new ScriptDiagnostic(
                                "UNITAS1004",
                                ScriptDiagnosticSeverity.Error,
                                "Assembly '" + model.Name + "' references assembly '" + target.Name + "' which is excluded on the current target: " + targetEx.Reason,
                                model.FilePath,
                                1,
                                1,
                                model.Name));
                        }

                        continue;
                    }

                    if (!deps.Contains(target.Name))
                    {
                        deps.Add(target.Name);
                    }
                }
            }
        }

        private static void ValidateAssemblyDllImports(
            List<AssemblyDefinitionModel> allAssemblies,
            Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap,
            Dictionary<string, List<string>> scriptsByAssembly,
            Dictionary<string, HashSet<string>> assemblyDefinesMap,
            Dictionary<string, DiscoveredPluginRecord> nativePluginsByName,
            ScriptCompilationOptions effectiveOptions,
            List<ScriptDiagnostic> diagnostics)
        {
            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                if (exclusionMap[model.Name].IsExcluded)
                {
                    continue;
                }

                if (!scriptsByAssembly.TryGetValue(model.Name, out List<string>? sources))
                {
                    continue;
                }

                HashSet<string> asmDefines = assemblyDefinesMap[model.Name];
                for (int s = 0; s < sources.Count; s++)
                {
                    List<ParsedDllImportCall> dllImports = ScriptAnalyzerAndGlueGenerator.ScanDllImports(sources[s], asmDefines);
                    for (int d = 0; d < dllImports.Count; d++)
                    {
                        ParsedDllImportCall call = dllImports[d];
                        if (string.Equals(call.LibraryName, "__Internal", StringComparison.Ordinal))
                        {
                            continue;
                        }

                        if (nativePluginsByName.TryGetValue(call.LibraryName, out DiscoveredPluginRecord? nativePlugin))
                        {
                            if (!nativePlugin.IsCompatible || !nativePlugin.IsIncludedForTarget)
                            {
                                string targetName = effectiveOptions.IsEditor
                                    ? "Editor (" + AssemblyDiscovery.GetUnityPlatformToken(effectiveOptions.TargetPlatform) + ")"
                                    : AssemblyDiscovery.GetUnityPlatformToken(effectiveOptions.TargetPlatform);
                                string detail = nativePlugin.IncompatibilityDiagnostic ?? "Plugin is excluded on the running target.";
                                diagnostics.Add(new ScriptDiagnostic(
                                    "UNITAS1020",
                                    ScriptDiagnosticSeverity.Error,
                                    "DllImport of native plugin '" + nativePlugin.LibraryName + "' in member '" + call.MethodName + "' failed on target '" + targetName + "' (architecture '" + effectiveOptions.TargetArchitecture + "'): " + detail,
                                    call.FilePath,
                                    call.Line,
                                    1,
                                    model.Name));
                            }
                        }
                    }
                }
            }
        }

        private static ScriptCompilationResult ExecuteStagedCompilation(
            string outputDirectory,
            ScriptCompilationOptions effectiveOptions,
            List<string> topoOrder,
            Dictionary<string, AssemblyDefinitionModel> modelsByName,
            Dictionary<string, List<string>> scriptsByAssembly,
            Dictionary<string, HashSet<string>> assemblyDefinesMap,
            Dictionary<string, ResponseFileOptions> assemblyRspMap,
            Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap,
            Dictionary<string, List<string>> resolvedDeps,
            List<string> autoReferencedManagedDlls,
            Dictionary<string, string> allCompatibleManagedDllsByFileName,
            Dictionary<string, string> pathToGuid,
            List<DiscoveredPluginRecord> pluginRecords,
            List<ScriptDiagnostic> diagnostics)
        {
            string stagingDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_script_staging_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(stagingDir);

            try
            {
                Dictionary<string, string> previousHashes = LoadCompilationState(outputDirectory);
                Dictionary<string, string> newHashes = new Dictionary<string, string>(StringComparer.Ordinal);

                IReadOnlyList<string> engineReferenceDlls = ResolveEngineAndBaseReferences(effectiveOptions);

                if (!CompileTopologicalAssemblies(
                    outputDirectory,
                    stagingDir,
                    effectiveOptions,
                    topoOrder,
                    modelsByName,
                    scriptsByAssembly,
                    assemblyDefinesMap,
                    assemblyRspMap,
                    exclusionMap,
                    resolvedDeps,
                    engineReferenceDlls,
                    autoReferencedManagedDlls,
                    allCompatibleManagedDllsByFileName,
                    previousHashes,
                    newHashes,
                    diagnostics,
                    out List<CompiledAssemblyRecord> compiledRecords,
                    out List<string> recompiledNames,
                    out Dictionary<string, string> compiledDllPathByAssembly))
                {
                    return BuildFailedResult(outputDirectory, compiledRecords, diagnostics);
                }

                if (!BuildAndCompileGlue(
                    outputDirectory,
                    stagingDir,
                    effectiveOptions,
                    topoOrder,
                    exclusionMap,
                    scriptsByAssembly,
                    assemblyDefinesMap,
                    pathToGuid,
                    engineReferenceDlls,
                    recompiledNames,
                    previousHashes,
                    newHashes,
                    compiledDllPathByAssembly,
                    diagnostics,
                    out List<ScriptGuidBinding> allBindings,
                    out string? generatedGlueDllPath,
                    out string? generatedGlueSourcePath))
                {
                    return BuildFailedResult(outputDirectory, compiledRecords, diagnostics);
                }

                return FinalizeSuccessfulCompilation(
                    outputDirectory,
                    effectiveOptions,
                    compiledDllPathByAssembly,
                    newHashes,
                    generatedGlueDllPath,
                    generatedGlueSourcePath,
                    compiledRecords,
                    diagnostics,
                    recompiledNames,
                    allBindings,
                    pluginRecords);
            }
            finally
            {
                if (Directory.Exists(stagingDir))
                {
                    try
                    {
                        Directory.Delete(stagingDir, true);
                    }
                    catch (IOException)
                    {
                    }
                }
            }
        }

        private static bool CompileTopologicalAssemblies(
            string outputDirectory,
            string stagingDir,
            ScriptCompilationOptions effectiveOptions,
            List<string> topoOrder,
            Dictionary<string, AssemblyDefinitionModel> modelsByName,
            Dictionary<string, List<string>> scriptsByAssembly,
            Dictionary<string, HashSet<string>> assemblyDefinesMap,
            Dictionary<string, ResponseFileOptions> assemblyRspMap,
            Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap,
            Dictionary<string, List<string>> resolvedDeps,
            IReadOnlyList<string> engineReferenceDlls,
            List<string> autoReferencedManagedDlls,
            Dictionary<string, string> allCompatibleManagedDllsByFileName,
            Dictionary<string, string> previousHashes,
            Dictionary<string, string> newHashes,
            List<ScriptDiagnostic> diagnostics,
            out List<CompiledAssemblyRecord> compiledRecords,
            out List<string> recompiledNames,
            out Dictionary<string, string> compiledDllPathByAssembly)
        {
            HashSet<string> dirtyAssemblies = new HashSet<string>(StringComparer.Ordinal);
            compiledRecords = new List<CompiledAssemblyRecord>();
            recompiledNames = new List<string>();
            compiledDllPathByAssembly = new Dictionary<string, string>(StringComparer.Ordinal);

            for (int i = 0; i < topoOrder.Count; i++)
            {
                string asmName = topoOrder[i];
                AssemblyDefinitionModel model = modelsByName[asmName];
                (bool isExcluded, string exclusionReason) = exclusionMap[asmName];
                List<string> sources = scriptsByAssembly.TryGetValue(asmName, out List<string>? foundSources)
                    ? foundSources
                    : new List<string>();

                List<string> sortedDefines = new List<string>(assemblyDefinesMap[asmName]);
                sortedDefines.Sort(StringComparer.Ordinal);
                List<string> directDeps = resolvedDeps[asmName];
                string finalDllPath = Path.Combine(outputDirectory, asmName + ".dll");

                if (isExcluded || sources.Count == 0)
                {
                    compiledRecords.Add(new CompiledAssemblyRecord(
                        asmName,
                        isExcluded ? string.Empty : finalDllPath,
                        sources,
                        sortedDefines,
                        Array.Empty<string>(),
                        directDeps,
                        false,
                        isExcluded,
                        exclusionReason));
                    continue;
                }

                List<string> externalRefs = BuildAssemblyReferencePaths(
                    model,
                    assemblyRspMap[asmName],
                    engineReferenceDlls,
                    autoReferencedManagedDlls,
                    allCompatibleManagedDllsByFileName,
                    directDeps,
                    compiledDllPathByAssembly);

                string inputHash = ComputeAssemblyInputHash(
                    model,
                    sources,
                    sortedDefines,
                    assemblyRspMap[asmName],
                    directDeps,
                    newHashes);
                newHashes[asmName] = inputHash;

                bool dependencyRecompiled = false;
                for (int depIdx = 0; depIdx < directDeps.Count; depIdx++)
                {
                    if (dirtyAssemblies.Contains(directDeps[depIdx]))
                    {
                        dependencyRecompiled = true;
                        break;
                    }
                }

                bool needsRecompile = dependencyRecompiled
                    || !File.Exists(finalDllPath)
                    || !previousHashes.TryGetValue(asmName, out string? oldHash)
                    || !string.Equals(oldHash, inputHash, StringComparison.Ordinal);

                if (!needsRecompile)
                {
                    compiledDllPathByAssembly[asmName] = finalDllPath;
                    compiledRecords.Add(new CompiledAssemblyRecord(
                        asmName,
                        finalDllPath,
                        sources,
                        sortedDefines,
                        externalRefs,
                        directDeps,
                        false,
                        false,
                        string.Empty));
                    continue;
                }

                dirtyAssemblies.Add(asmName);
                string stagedDllPath = Path.Combine(stagingDir, asmName + ".dll");
                bool compileOk = RunCscCompilation(
                    asmName,
                    stagedDllPath,
                    sources,
                    sortedDefines,
                    externalRefs,
                    model.AllowUnsafeCode || assemblyRspMap[asmName].AllowUnsafe,
                    assemblyRspMap[asmName],
                    !effectiveOptions.IsEditor,
                    diagnostics);

                if (!compileOk || HasErrorDiagnostics(diagnostics))
                {
                    return false;
                }

                compiledDllPathByAssembly[asmName] = stagedDllPath;
                recompiledNames.Add(asmName);
                compiledRecords.Add(new CompiledAssemblyRecord(
                    asmName,
                    finalDllPath,
                    sources,
                    sortedDefines,
                    externalRefs,
                    directDeps,
                    true,
                    false,
                    string.Empty));
            }

            return true;
        }

        private static bool BuildAndCompileGlue(
            string outputDirectory,
            string stagingDir,
            ScriptCompilationOptions effectiveOptions,
            List<string> topoOrder,
            Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap,
            Dictionary<string, List<string>> scriptsByAssembly,
            Dictionary<string, HashSet<string>> assemblyDefinesMap,
            Dictionary<string, string> pathToGuid,
            IReadOnlyList<string> engineReferenceDlls,
            List<string> recompiledNames,
            Dictionary<string, string> previousHashes,
            Dictionary<string, string> newHashes,
            Dictionary<string, string> compiledDllPathByAssembly,
            List<ScriptDiagnostic> diagnostics,
            out List<ScriptGuidBinding> allBindings,
            out string? generatedGlueDllPath,
            out string? generatedGlueSourcePath)
        {
            allBindings = new List<ScriptGuidBinding>();
            List<RuntimeInitializeMethodBinding> allRuntimeInitMethods = new List<RuntimeInitializeMethodBinding>();
            List<EditorCallableMethodBinding> allEditorCallableMethods = new List<EditorCallableMethodBinding>();
            for (int i = 0; i < topoOrder.Count; i++)
            {
                string asmName = topoOrder[i];
                if (exclusionMap[asmName].IsExcluded)
                {
                    continue;
                }

                if (!scriptsByAssembly.TryGetValue(asmName, out List<string>? sources) || sources.Count == 0)
                {
                    continue;
                }

                List<ScriptGuidBinding> asmBindings = ScriptAnalyzerAndGlueGenerator.DiscoverScriptBindingsForAssembly(
                    asmName,
                    sources,
                    assemblyDefinesMap[asmName],
                    pathToGuid);
                allBindings.AddRange(asmBindings);

                List<RuntimeInitializeMethodBinding> asmInitMethods = ScriptAnalyzerAndGlueGenerator.DiscoverRuntimeInitializeMethodsForAssembly(
                    sources,
                    assemblyDefinesMap[asmName]);
                allRuntimeInitMethods.AddRange(asmInitMethods);

                List<EditorCallableMethodBinding> asmEditorCallableMethods = ScriptAnalyzerAndGlueGenerator.DiscoverEditorCallableMethodsForAssembly(
                    sources,
                    assemblyDefinesMap[asmName]);
                allEditorCallableMethods.AddRange(asmEditorCallableMethods);
            }

            generatedGlueDllPath = null;
            generatedGlueSourcePath = null;

            if (!effectiveOptions.EmitGeneratedGlue
                || (allBindings.Count == 0 && allRuntimeInitMethods.Count == 0 && allEditorCallableMethods.Count == 0))
            {
                return true;
            }

            string finalGlueDll = Path.Combine(outputDirectory, GeneratedGlueAssemblyName + ".dll");
            string glueObjDir = Path.Combine(outputDirectory, "obj");
            Directory.CreateDirectory(glueObjDir);
            generatedGlueSourcePath = Path.Combine(glueObjDir, "Unitas.Generated.g.cs");

            string glueSource = ScriptAnalyzerAndGlueGenerator.GenerateGlueSource(
                allBindings,
                Array.Empty<SerializableTypeBinding>(),
                allRuntimeInitMethods,
                allEditorCallableMethods);
            string stagedGlueSource = Path.Combine(stagingDir, "Unitas.Generated.g.cs");
            File.WriteAllText(stagedGlueSource, glueSource, Encoding.UTF8);

            string glueHash = ComputeSha256Text(glueSource);
            newHashes[GeneratedGlueAssemblyName] = glueHash;

            bool glueNeedsCompile = recompiledNames.Count > 0
                || !File.Exists(finalGlueDll)
                || !previousHashes.TryGetValue(GeneratedGlueAssemblyName, out string? prevGlueHash)
                || !string.Equals(prevGlueHash, glueHash, StringComparison.Ordinal);

            if (glueNeedsCompile)
            {
                List<string> glueRefs = new List<string>(engineReferenceDlls);
                foreach (KeyValuePair<string, string> kv in compiledDllPathByAssembly)
                {
                    if (!glueRefs.Contains(kv.Value))
                    {
                        glueRefs.Add(kv.Value);
                    }
                }

                string stagedGlueDll = Path.Combine(stagingDir, GeneratedGlueAssemblyName + ".dll");
                List<ScriptDiagnostic> glueDiagnostics = new List<ScriptDiagnostic>();
                bool glueOk = RunCscCompilation(
                    GeneratedGlueAssemblyName,
                    stagedGlueDll,
                    new[] { stagedGlueSource },
                    Array.Empty<string>(),
                    glueRefs,
                    false,
                    new ResponseFileOptions(),
                    false,
                    glueDiagnostics);

                if (!glueOk || HasErrorDiagnostics(glueDiagnostics))
                {
                    diagnostics.AddRange(glueDiagnostics);
                    return false;
                }

                File.WriteAllText(generatedGlueSourcePath, glueSource, Encoding.UTF8);
                compiledDllPathByAssembly[GeneratedGlueAssemblyName] = stagedGlueDll;
            }

            generatedGlueDllPath = finalGlueDll;
            return true;
        }

        private static ScriptCompilationResult FinalizeSuccessfulCompilation(
            string outputDirectory,
            ScriptCompilationOptions effectiveOptions,
            Dictionary<string, string> compiledDllPathByAssembly,
            Dictionary<string, string> newHashes,
            string? generatedGlueDllPath,
            string? generatedGlueSourcePath,
            List<CompiledAssemblyRecord> compiledRecords,
            List<ScriptDiagnostic> diagnostics,
            List<string> recompiledNames,
            List<ScriptGuidBinding> allBindings,
            List<DiscoveredPluginRecord> pluginRecords)
        {
            foreach (KeyValuePair<string, string> kv in compiledDllPathByAssembly)
            {
                string asmName = kv.Key;
                string stagedOrFinalPath = kv.Value;
                string targetFinalPath = Path.Combine(outputDirectory, asmName + ".dll");
                if (!string.Equals(Path.GetFullPath(stagedOrFinalPath), Path.GetFullPath(targetFinalPath), StringComparison.Ordinal))
                {
                    File.Copy(stagedOrFinalPath, targetFinalPath, true);
                }
            }

            SaveCompilationState(outputDirectory, newHashes);

            List<string> activeDlls = GetExistingDllsInDirectory(outputDirectory);

            if (effectiveOptions.AutoLoadGeneratedGlue
                && !string.IsNullOrEmpty(generatedGlueDllPath)
                && File.Exists(generatedGlueDllPath))
            {
                EditorMethodRegistry.Clear();
                ScriptGlueRegistry.LoadGlueAssembly(generatedGlueDllPath);
            }

            ScriptCompilationResult successResult = new ScriptCompilationResult(
                true,
                ActiveAssemblyState.Current,
                outputDirectory,
                generatedGlueDllPath,
                generatedGlueSourcePath,
                compiledRecords,
                diagnostics,
                recompiledNames,
                activeDlls,
                allBindings,
                pluginRecords);

            lock (LastValidGate)
            {
                LastValidResultByOutputDir[outputDirectory] = successResult;
            }

            return successResult;
        }

        private static ScriptCompilationResult BuildFailedResult(
            string outputDirectory,
            List<CompiledAssemblyRecord> records,
            List<ScriptDiagnostic> diagnostics)
        {
            List<string> existingValidDlls = GetExistingDllsInDirectory(outputDirectory);
            ScriptCompilationResult? previousValid = null;
            lock (LastValidGate)
            {
                LastValidResultByOutputDir.TryGetValue(outputDirectory, out previousValid);
            }

            if (existingValidDlls.Count == 0 && previousValid != null && previousValid.ActiveAssemblyPaths.Count > 0)
            {
                existingValidDlls = new List<string>(previousValid.ActiveAssemblyPaths);
            }

            ActiveAssemblyState state = existingValidDlls.Count > 0
                ? ActiveAssemblyState.LastValidFallback
                : ActiveAssemblyState.None;

            string candidateGlue = Path.Combine(outputDirectory, GeneratedGlueAssemblyName + ".dll");
            string? existingGlue = File.Exists(candidateGlue)
                ? candidateGlue
                : previousValid?.GeneratedGlueAssemblyPath;
            string? existingGlueSource = previousValid?.GeneratedGlueSourcePath;

            IReadOnlyList<ScriptGuidBinding> preservedBindings = previousValid != null
                ? previousValid.ScriptBindings
                : Array.Empty<ScriptGuidBinding>();
            IReadOnlyList<DiscoveredPluginRecord> preservedPlugins = previousValid != null
                ? previousValid.Plugins
                : Array.Empty<DiscoveredPluginRecord>();

            return new ScriptCompilationResult(
                false,
                state,
                outputDirectory,
                existingGlue,
                existingGlueSource,
                records,
                diagnostics,
                Array.Empty<string>(),
                existingValidDlls,
                preservedBindings,
                preservedPlugins);
        }

        private static List<string> TopologicalSortAndDetectCycles(
            List<AssemblyDefinitionModel> allAssemblies,
            Dictionary<string, List<string>> resolvedDeps,
            Dictionary<string, AssemblyDefinitionModel> modelsByName,
            List<ScriptDiagnostic> diagnostics)
        {
            List<string> order = new List<string>();
            Dictionary<string, int> visitState = new Dictionary<string, int>(StringComparer.Ordinal);
            List<string> activeStack = new List<string>();

            void Visit(string name)
            {
                if (visitState.TryGetValue(name, out int state))
                {
                    if (state == 2)
                    {
                        return;
                    }

                    if (state == 1)
                    {
                        int cycleStart = activeStack.IndexOf(name);
                        string otherName = activeStack.Count > 0 ? activeStack[activeStack.Count - 1] : name;
                        if (cycleStart >= 0 && cycleStart + 1 < activeStack.Count && string.Equals(otherName, name, StringComparison.Ordinal))
                        {
                            otherName = activeStack[cycleStart + 1];
                        }

                        string cycleChain = cycleStart >= 0
                            ? string.Join(" -> ", activeStack.GetRange(cycleStart, activeStack.Count - cycleStart)) + " -> " + name
                            : name + " -> " + otherName;

                        string filePath = modelsByName.TryGetValue(name, out AssemblyDefinitionModel? m) ? m.FilePath : string.Empty;
                        diagnostics.Add(new ScriptDiagnostic(
                            "UNITAS1005",
                            ScriptDiagnosticSeverity.Error,
                            "Assembly reference cycle detected involving '" + name + "' and '" + otherName + "' (" + cycleChain + ").",
                            filePath,
                            1,
                            1,
                            name));
                        return;
                    }
                }

                visitState[name] = 1;
                activeStack.Add(name);

                if (resolvedDeps.TryGetValue(name, out List<string>? deps))
                {
                    for (int i = 0; i < deps.Count; i++)
                    {
                        Visit(deps[i]);
                    }
                }

                activeStack.RemoveAt(activeStack.Count - 1);
                visitState[name] = 2;
                order.Add(name);
            }

            for (int i = 0; i < allAssemblies.Count; i++)
            {
                Visit(allAssemblies[i].Name);
            }

            return order;
        }

        private static class MetadataRefCache
        {
            public static readonly Dictionary<string, (DateTime LastWriteUtc, PortableExecutableReference Reference)> Items =
                new Dictionary<string, (DateTime LastWriteUtc, PortableExecutableReference Reference)>(StringComparer.Ordinal);
            public static readonly object Gate = new object();
        }

        internal static PortableExecutableReference GetMetadataReference(string fullPath)
        {
            return GetCachedMetadataReference(fullPath);
        }

        internal static IReadOnlyList<string> GetNetReferenceDlls()
        {
            if (_cachedNetReferenceDlls == null)
            {
                _cachedNetReferenceDlls = DiscoverNetReferencePackDlls();
            }

            return _cachedNetReferenceDlls;
        }

        internal static CSharpCompilation CreateRoslynCompilation(
            string assemblyName,
            IReadOnlyList<string> sourceFiles,
            IReadOnlyList<string> defines,
            IReadOnlyList<MetadataReference> metadataRefs,
            bool allowUnsafe,
            ResponseFileOptions rspOptions)
        {
            CSharpParseOptions parseOptions = new CSharpParseOptions(
                LanguageVersion.CSharp9,
                DocumentationMode.None,
                SourceCodeKind.Regular,
                defines);

            List<SyntaxTree> syntaxTrees = new List<SyntaxTree>(sourceFiles.Count);
            for (int i = 0; i < sourceFiles.Count; i++)
            {
                string fullSourcePath = Path.GetFullPath(sourceFiles[i]);
                if (!File.Exists(fullSourcePath))
                {
                    continue;
                }

                string sourceText = File.ReadAllText(fullSourcePath, Encoding.UTF8);
                syntaxTrees.Add(CSharpSyntaxTree.ParseText(sourceText, parseOptions, fullSourcePath, Encoding.UTF8));
            }

            Dictionary<string, ReportDiagnostic> specificDiagnosticOptions =
                new Dictionary<string, ReportDiagnostic>(StringComparer.OrdinalIgnoreCase)
                {
                    ["CS1701"] = ReportDiagnostic.Suppress,
                    ["CS1702"] = ReportDiagnostic.Suppress
                };

            for (int i = 0; i < rspOptions.DisabledWarnings.Count; i++)
            {
                string w = rspOptions.DisabledWarnings[i].Trim();
                if (!string.IsNullOrEmpty(w))
                {
                    string code = w.StartsWith("CS", StringComparison.OrdinalIgnoreCase) ? w.ToUpperInvariant() : "CS" + w.PadLeft(4, '0');
                    specificDiagnosticOptions[code] = ReportDiagnostic.Suppress;
                    specificDiagnosticOptions[w] = ReportDiagnostic.Suppress;
                }
            }

            for (int i = 0; i < rspOptions.WarningsAsErrorIds.Count; i++)
            {
                string w = rspOptions.WarningsAsErrorIds[i].Trim();
                if (!string.IsNullOrEmpty(w))
                {
                    string code = w.StartsWith("CS", StringComparison.OrdinalIgnoreCase) ? w.ToUpperInvariant() : "CS" + w.PadLeft(4, '0');
                    specificDiagnosticOptions[code] = ReportDiagnostic.Error;
                    specificDiagnosticOptions[w] = ReportDiagnostic.Error;
                }
            }

            CSharpCompilationOptions compilationOptions = new CSharpCompilationOptions(
                OutputKind.DynamicallyLinkedLibrary,
                reportSuppressedDiagnostics: false,
                moduleName: assemblyName + ".dll",
                allowUnsafe: allowUnsafe,
                generalDiagnosticOption: rspOptions.WarningsAsErrors ? ReportDiagnostic.Error : ReportDiagnostic.Default,
                specificDiagnosticOptions: specificDiagnosticOptions,
                concurrentBuild: false,
                deterministic: true,
                nullableContextOptions: NullableContextOptions.Annotations);

            return CSharpCompilation.Create(
                assemblyName,
                syntaxTrees,
                metadataRefs,
                compilationOptions);
        }

        internal static IReadOnlyList<(string AssemblyName, CSharpCompilation Compilation)> CreateProjectCompilations(
            string fullProjectRoot,
            ScriptCompilationOptions effectiveOptions,
            List<ScriptDiagnostic> diagnostics)
        {
            Dictionary<string, string> guidToPath = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            Dictionary<string, string> pathToGuid = new Dictionary<string, string>(StringComparer.Ordinal);
            List<ParsedPluginMeta> parsedPlugins = new List<ParsedPluginMeta>();

            ProjectMetadataIndex.ScanProjectMeta(fullProjectRoot, guidToPath, pathToGuid, parsedPlugins);

            HashSet<string> baseDefines = DefinesAndResponseParser.BuildBaseDefines(effectiveOptions);

            ProjectMetadataIndex.EvaluatePlugins(
                parsedPlugins,
                effectiveOptions,
                baseDefines,
                out Dictionary<string, DiscoveredPluginRecord> _,
                out List<string> autoReferencedManagedDlls,
                out Dictionary<string, string> allCompatibleManagedDllsByFileName);

            List<AssemblyDefinitionModel> customAssemblies = AssemblyDiscovery.DiscoverAssemblies(
                fullProjectRoot,
                guidToPath,
                pathToGuid,
                diagnostics);

            Dictionary<string, List<string>> scriptsByAssembly = AssemblyDiscovery.AssignScriptsToAssemblies(
                fullProjectRoot,
                customAssemblies,
                out List<AssemblyDefinitionModel> predefinedAssemblies);

            List<AssemblyDefinitionModel> allAssemblies = new List<AssemblyDefinitionModel>();
            allAssemblies.AddRange(customAssemblies);
            allAssemblies.AddRange(predefinedAssemblies);

            Dictionary<string, AssemblyDefinitionModel> modelsByName =
                new Dictionary<string, AssemblyDefinitionModel>(StringComparer.Ordinal);
            Dictionary<string, AssemblyDefinitionModel> modelsByGuid =
                new Dictionary<string, AssemblyDefinitionModel>(StringComparer.OrdinalIgnoreCase);

            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                modelsByName[model.Name] = model;
                if (!string.IsNullOrEmpty(model.Guid))
                {
                    modelsByGuid[model.Guid] = model;
                }
            }

            Dictionary<string, HashSet<string>> assemblyDefinesMap =
                new Dictionary<string, HashSet<string>>(StringComparer.Ordinal);
            Dictionary<string, ResponseFileOptions> assemblyRspMap =
                new Dictionary<string, ResponseFileOptions>(StringComparer.Ordinal);
            Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap =
                new Dictionary<string, (bool IsExcluded, string Reason)>(StringComparer.Ordinal);

            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                HashSet<string> asmDefines = new HashSet<string>(baseDefines, StringComparer.Ordinal);
                AssemblyDiscovery.ApplyVersionDefines(model, effectiveOptions, asmDefines, diagnostics);

                ResponseFileOptions rsp = DefinesAndResponseParser.ParseCscRspFiles(
                    fullProjectRoot,
                    model.IsPredefined ? null : model.DirectoryPath,
                    model.Name,
                    diagnostics);

                for (int d = 0; d < rsp.Defines.Count; d++)
                {
                    asmDefines.Add(rsp.Defines[d]);
                }

                assemblyDefinesMap[model.Name] = asmDefines;
                assemblyRspMap[model.Name] = rsp;

                if (!AssemblyDiscovery.EvaluatePlatformEligibility(model, effectiveOptions, out string platReason))
                {
                    exclusionMap[model.Name] = (true, platReason);
                    continue;
                }

                if (!AssemblyDiscovery.EvaluateDefineConstraints(model.DefineConstraints, asmDefines, out string failedConstraint))
                {
                    exclusionMap[model.Name] = (true, "Excluded by defineConstraints (" + failedConstraint + ").");
                    continue;
                }

                exclusionMap[model.Name] = (false, string.Empty);
            }

            Dictionary<string, List<string>> resolvedDeps = new Dictionary<string, List<string>>(StringComparer.Ordinal);
            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                List<string> deps = new List<string>();
                resolvedDeps[model.Name] = deps;

                if (exclusionMap[model.Name].IsExcluded)
                {
                    continue;
                }

                for (int r = 0; r < model.References.Count; r++)
                {
                    string refToken = model.References[r];
                    AssemblyDefinitionModel? target = AssemblyDiscovery.ResolveReferenceToken(refToken, modelsByName, modelsByGuid);
                    if (target == null)
                    {
                        continue;
                    }

                    if (exclusionMap.TryGetValue(target.Name, out (bool IsExcluded, string Reason) targetEx) && targetEx.IsExcluded)
                    {
                        continue;
                    }

                    if (!deps.Contains(target.Name))
                    {
                        deps.Add(target.Name);
                    }
                }
            }

            List<string> topoOrder = TopologicalSortAndDetectCycles(allAssemblies, resolvedDeps, modelsByName, diagnostics);
            IReadOnlyList<string> engineReferenceDlls = ResolveEngineAndBaseReferences(effectiveOptions);

            List<(string AssemblyName, CSharpCompilation Compilation)> results =
                new List<(string AssemblyName, CSharpCompilation Compilation)>();
            Dictionary<string, CSharpCompilation> compilationByAssembly =
                new Dictionary<string, CSharpCompilation>(StringComparer.Ordinal);

            for (int i = 0; i < topoOrder.Count; i++)
            {
                string asmName = topoOrder[i];
                if (exclusionMap[asmName].IsExcluded)
                {
                    continue;
                }

                List<string> sources = scriptsByAssembly.TryGetValue(asmName, out List<string>? foundSources)
                    ? foundSources
                    : new List<string>();
                if (sources.Count == 0)
                {
                    continue;
                }

                AssemblyDefinitionModel model = modelsByName[asmName];
                ResponseFileOptions rsp = assemblyRspMap[asmName];
                List<string> sortedDefines = new List<string>(assemblyDefinesMap[asmName]);
                sortedDefines.Sort(StringComparer.Ordinal);

                List<string> externalRefPaths = BuildAssemblyReferencePaths(
                    model,
                    rsp,
                    engineReferenceDlls,
                    autoReferencedManagedDlls,
                    allCompatibleManagedDllsByFileName,
                    new List<string>(),
                    new Dictionary<string, string>(StringComparer.Ordinal));

                List<MetadataReference> metadataRefs = new List<MetadataReference>();
                HashSet<string> addedRefs = new HashSet<string>(StringComparer.Ordinal);
                for (int r = 0; r < externalRefPaths.Count; r++)
                {
                    string path = externalRefPaths[r];
                    if (File.Exists(path))
                    {
                        string fullRef = Path.GetFullPath(path);
                        if (addedRefs.Add(fullRef))
                        {
                            metadataRefs.Add(GetCachedMetadataReference(fullRef));
                        }
                    }
                }

                List<string> directDeps = resolvedDeps[asmName];
                for (int d = 0; d < directDeps.Count; d++)
                {
                    if (compilationByAssembly.TryGetValue(directDeps[d], out CSharpCompilation? depComp))
                    {
                        metadataRefs.Add(depComp.ToMetadataReference());
                    }
                }

                CSharpCompilation compilation = CreateRoslynCompilation(
                    asmName,
                    sources,
                    sortedDefines,
                    metadataRefs,
                    model.AllowUnsafeCode || rsp.AllowUnsafe,
                    rsp);

                compilationByAssembly[asmName] = compilation;
                results.Add((asmName, compilation));
            }

            return results;
        }

        private static PortableExecutableReference GetCachedMetadataReference(string fullPath)
        {
            DateTime lastWrite = File.GetLastWriteTimeUtc(fullPath);
            lock (MetadataRefCache.Gate)
            {
                if (MetadataRefCache.Items.TryGetValue(fullPath, out (DateTime LastWriteUtc, PortableExecutableReference Reference) cached)
                    && cached.LastWriteUtc == lastWrite)
                {
                    return cached.Reference;
                }

                PortableExecutableReference created = MetadataReference.CreateFromFile(fullPath);
                MetadataRefCache.Items[fullPath] = (lastWrite, created);
                return created;
            }
        }

        private static bool RunCscCompilation(
            string assemblyName,
            string outputDllPath,
            IReadOnlyList<string> sourceFiles,
            IReadOnlyList<string> defines,
            IReadOnlyList<string> referencePaths,
            bool allowUnsafe,
            ResponseFileOptions rspOptions,
            bool rejectUnguardedUnityEditorReferences,
            List<ScriptDiagnostic> diagnostics)
        {
            CSharpParseOptions parseOptions = new CSharpParseOptions(
                LanguageVersion.CSharp9,
                DocumentationMode.None,
                SourceCodeKind.Regular,
                defines);

            List<SyntaxTree> syntaxTrees = new List<SyntaxTree>(sourceFiles.Count);
            for (int i = 0; i < sourceFiles.Count; i++)
            {
                string fullSourcePath = Path.GetFullPath(sourceFiles[i]);
                string sourceText = File.ReadAllText(fullSourcePath, Encoding.UTF8);
                syntaxTrees.Add(CSharpSyntaxTree.ParseText(sourceText, parseOptions, fullSourcePath, Encoding.UTF8));
            }

            if (rejectUnguardedUnityEditorReferences && InspectUnguardedUnityEditorReferences(syntaxTrees, assemblyName, diagnostics))
            {
                if (File.Exists(outputDllPath))
                {
                    try
                    {
                        File.Delete(outputDllPath);
                    }
                    catch (IOException)
                    {
                    }
                }

                return false;
            }

            List<MetadataReference> metadataRefs = new List<MetadataReference>(referencePaths.Count);
            HashSet<string> addedRefs = new HashSet<string>(StringComparer.Ordinal);
            for (int i = 0; i < referencePaths.Count; i++)
            {
                string r = referencePaths[i];
                if (File.Exists(r))
                {
                    string fullRef = Path.GetFullPath(r);
                    if (addedRefs.Add(fullRef))
                    {
                        metadataRefs.Add(GetCachedMetadataReference(fullRef));
                    }
                }
            }

            Dictionary<string, ReportDiagnostic> specificDiagnosticOptions =
                new Dictionary<string, ReportDiagnostic>(StringComparer.OrdinalIgnoreCase)
                {
                    ["CS1701"] = ReportDiagnostic.Suppress,
                    ["CS1702"] = ReportDiagnostic.Suppress
                };

            for (int i = 0; i < rspOptions.DisabledWarnings.Count; i++)
            {
                string w = rspOptions.DisabledWarnings[i].Trim();
                if (!string.IsNullOrEmpty(w))
                {
                    string code = w.StartsWith("CS", StringComparison.OrdinalIgnoreCase) ? w.ToUpperInvariant() : "CS" + w.PadLeft(4, '0');
                    specificDiagnosticOptions[code] = ReportDiagnostic.Suppress;
                    specificDiagnosticOptions[w] = ReportDiagnostic.Suppress;
                }
            }

            for (int i = 0; i < rspOptions.WarningsAsErrorIds.Count; i++)
            {
                string w = rspOptions.WarningsAsErrorIds[i].Trim();
                if (!string.IsNullOrEmpty(w))
                {
                    string code = w.StartsWith("CS", StringComparison.OrdinalIgnoreCase) ? w.ToUpperInvariant() : "CS" + w.PadLeft(4, '0');
                    specificDiagnosticOptions[code] = ReportDiagnostic.Error;
                    specificDiagnosticOptions[w] = ReportDiagnostic.Error;
                }
            }

            CSharpCompilationOptions compilationOptions = new CSharpCompilationOptions(
                OutputKind.DynamicallyLinkedLibrary,
                reportSuppressedDiagnostics: false,
                moduleName: assemblyName + ".dll",
                allowUnsafe: allowUnsafe,
                generalDiagnosticOption: rspOptions.WarningsAsErrors ? ReportDiagnostic.Error : ReportDiagnostic.Default,
                specificDiagnosticOptions: specificDiagnosticOptions,
                concurrentBuild: false,
                deterministic: true,
                nullableContextOptions: NullableContextOptions.Annotations);

            CSharpCompilation compilation = CSharpCompilation.Create(
                assemblyName,
                syntaxTrees,
                metadataRefs,
                compilationOptions);

            string? outDir = Path.GetDirectoryName(outputDllPath);
            if (!string.IsNullOrEmpty(outDir))
            {
                Directory.CreateDirectory(outDir);
            }

            using FileStream peStream = new FileStream(outputDllPath, FileMode.Create, FileAccess.Write, FileShare.None);
            EmitResult emitResult = compilation.Emit(peStream);

            foreach (Diagnostic diag in emitResult.Diagnostics)
            {
                if (diag.Severity == DiagnosticSeverity.Hidden || diag.IsSuppressed)
                {
                    continue;
                }

                ScriptDiagnosticSeverity sev = diag.Severity switch
                {
                    DiagnosticSeverity.Error => ScriptDiagnosticSeverity.Error,
                    DiagnosticSeverity.Warning => ScriptDiagnosticSeverity.Warning,
                    _ => ScriptDiagnosticSeverity.Info
                };

                FileLinePositionSpan span = diag.Location.GetMappedLineSpan();
                string file = span.Path ?? string.Empty;
                int lineNum = diag.Location.IsInSource ? span.StartLinePosition.Line + 1 : 0;
                int colNum = diag.Location.IsInSource ? span.StartLinePosition.Character + 1 : 0;

                diagnostics.Add(new ScriptDiagnostic(
                    diag.Id,
                    sev,
                    diag.GetMessage(CultureInfo.InvariantCulture),
                    file,
                    lineNum,
                    colNum,
                    assemblyName));
            }

            if (!emitResult.Success)
            {
                peStream.Close();
                if (File.Exists(outputDllPath))
                {
                    try
                    {
                        File.Delete(outputDllPath);
                    }
                    catch (IOException)
                    {
                    }
                }

                return false;
            }

            return true;
        }

        private static bool InspectUnguardedUnityEditorReferences(
            List<SyntaxTree> syntaxTrees,
            string assemblyName,
            List<ScriptDiagnostic> diagnostics)
        {
            bool foundAny = false;
            for (int t = 0; t < syntaxTrees.Count; t++)
            {
                SyntaxTree tree = syntaxTrees[t];
                SyntaxNode root = tree.GetRoot();
                string filePath = tree.FilePath ?? string.Empty;

                foreach (SyntaxNode node in root.DescendantNodes())
                {
                    if (node is UsingDirectiveSyntax usingDirective && usingDirective.Name != null)
                    {
                        string usingName = NormalizeNameText(usingDirective.Name.ToString());
                        if (IsUnityEditorName(usingName))
                        {
                            FileLinePositionSpan span = usingDirective.GetLocation().GetMappedLineSpan();
                            int line = span.StartLinePosition.Line + 1;
                            int col = span.StartLinePosition.Character + 1;
                            string enclosingMember = FindEnclosingMemberName(usingDirective);
                            string msg = string.IsNullOrEmpty(enclosingMember)
                                ? "Unguarded reference to '" + usingName + "' in '" + filePath + "' is not permitted in a player build."
                                : "Unguarded reference to '" + usingName + "' in member '" + enclosingMember + "' of '" + filePath + "' is not permitted in a player build.";
                            diagnostics.Add(new ScriptDiagnostic(
                                "UNITAS1030",
                                ScriptDiagnosticSeverity.Error,
                                msg,
                                filePath,
                                line,
                                col,
                                assemblyName));
                            foundAny = true;
                        }

                        continue;
                    }

                    if (node is MemberAccessExpressionSyntax || node is QualifiedNameSyntax)
                    {
                        if (node.Parent is MemberAccessExpressionSyntax
                            || node.Parent is QualifiedNameSyntax
                            || node.Parent is UsingDirectiveSyntax)
                        {
                            continue;
                        }

                        string exprText = NormalizeNameText(node.ToString());
                        if (IsUnityEditorName(exprText))
                        {
                            FileLinePositionSpan span = node.GetLocation().GetMappedLineSpan();
                            int line = span.StartLinePosition.Line + 1;
                            int col = span.StartLinePosition.Character + 1;
                            string enclosingMember = FindEnclosingMemberName(node);
                            string msg = string.IsNullOrEmpty(enclosingMember)
                                ? "Unguarded UnityEditor member reference '" + exprText + "' in '" + filePath + "' is not permitted in a player build."
                                : "Unguarded UnityEditor member reference '" + exprText + "' in member '" + enclosingMember + "' of '" + filePath + "' is not permitted in a player build.";
                            diagnostics.Add(new ScriptDiagnostic(
                                "UNITAS1030",
                                ScriptDiagnosticSeverity.Error,
                                msg,
                                filePath,
                                line,
                                col,
                                assemblyName));
                            foundAny = true;
                        }
                    }
                }
            }

            return foundAny;
        }

        private static string NormalizeNameText(string raw)
        {
            string trimmed = raw.Trim();
            if (trimmed.StartsWith("global::", StringComparison.Ordinal))
            {
                trimmed = trimmed.Substring("global::".Length).Trim();
            }

            return trimmed;
        }

        private static bool IsUnityEditorName(string name)
        {
            return string.Equals(name, "UnityEditor", StringComparison.Ordinal)
                || name.StartsWith("UnityEditor.", StringComparison.Ordinal);
        }

        private static string FindEnclosingMemberName(SyntaxNode node)
        {
            string? memberName = null;
            string? typeName = null;
            SyntaxNode? current = node.Parent;
            while (current != null)
            {
                if (memberName == null)
                {
                    if (current is MethodDeclarationSyntax methodDecl)
                    {
                        memberName = methodDecl.Identifier.ValueText;
                    }
                    else if (current is PropertyDeclarationSyntax propDecl)
                    {
                        memberName = propDecl.Identifier.ValueText;
                    }
                    else if (current is ConstructorDeclarationSyntax ctorDecl)
                    {
                        memberName = ctorDecl.Identifier.ValueText;
                    }
                    else if (current is FieldDeclarationSyntax fieldDecl && fieldDecl.Declaration.Variables.Count > 0)
                    {
                        memberName = fieldDecl.Declaration.Variables[0].Identifier.ValueText;
                    }
                }

                if (typeName == null && current is BaseTypeDeclarationSyntax typeDecl)
                {
                    typeName = typeDecl.Identifier.ValueText;
                }

                current = current.Parent;
            }

            if (!string.IsNullOrEmpty(typeName) && !string.IsNullOrEmpty(memberName))
            {
                return typeName + "." + memberName;
            }

            return memberName ?? typeName ?? string.Empty;
        }

        private static void ParseCscOutput(string output, string assemblyName, List<ScriptDiagnostic> diagnostics)
        {
            if (string.IsNullOrWhiteSpace(output))
            {
                return;
            }

            string[] lines = output.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i].Trim();
                if (string.IsNullOrEmpty(line))
                {
                    continue;
                }

                Match m = CscDiagnosticRegex.Match(line);
                if (!m.Success)
                {
                    continue;
                }

                string file = m.Groups["file"].Success ? m.Groups["file"].Value : m.Groups["file2"].Value;
                int lineNum = 0;
                int colNum = 0;
                if (m.Groups["line"].Success)
                {
                    int.TryParse(m.Groups["line"].Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out lineNum);
                }

                if (m.Groups["col"].Success)
                {
                    int.TryParse(m.Groups["col"].Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out colNum);
                }

                string sevText = m.Groups["sev"].Value;
                ScriptDiagnosticSeverity sev = sevText switch
                {
                    "error" => ScriptDiagnosticSeverity.Error,
                    "warning" => ScriptDiagnosticSeverity.Warning,
                    _ => ScriptDiagnosticSeverity.Info
                };

                string id = m.Groups["id"].Value;
                string msg = m.Groups["msg"].Value;

                diagnostics.Add(new ScriptDiagnostic(id, sev, msg, file, lineNum, colNum, assemblyName));
            }
        }

        private static List<string> ResolveEngineAndBaseReferences(ScriptCompilationOptions options)
        {
            List<string> refs = new List<string>();
            if (_cachedNetReferenceDlls == null)
            {
                _cachedNetReferenceDlls = DiscoverNetReferencePackDlls();
            }

            refs.AddRange(_cachedNetReferenceDlls);

            string unityEngineDll = typeof(UnityEngine.MonoBehaviour).Assembly.Location;
            if (!string.IsNullOrEmpty(unityEngineDll) && File.Exists(unityEngineDll))
            {
                refs.Add(unityEngineDll);
            }

            string unitasBridgeDll = typeof(Unitas.Bridge.BridgeHost).Assembly.Location;
            if (!string.IsNullOrEmpty(unitasBridgeDll) && File.Exists(unitasBridgeDll))
            {
                refs.Add(unitasBridgeDll);
            }

            for (int i = 0; i < options.AdditionalReferencePaths.Count; i++)
            {
                string p = options.AdditionalReferencePaths[i];
                if (File.Exists(p))
                {
                    refs.Add(Path.GetFullPath(p));
                }
            }

            return refs;
        }

        private static List<string> BuildAssemblyReferencePaths(
            AssemblyDefinitionModel model,
            ResponseFileOptions rsp,
            IReadOnlyList<string> engineReferenceDlls,
            List<string> autoReferencedManagedDlls,
            Dictionary<string, string> allCompatibleManagedDllsByFileName,
            List<string> directDeps,
            Dictionary<string, string> compiledDllPathByAssembly)
        {
            List<string> refs = new List<string>();
            if (!model.NoEngineReferences)
            {
                refs.AddRange(engineReferenceDlls);
            }
            else if (_cachedNetReferenceDlls != null)
            {
                refs.AddRange(_cachedNetReferenceDlls);
            }

            if (model.OverrideReferences)
            {
                for (int i = 0; i < model.PrecompiledReferences.Count; i++)
                {
                    string precomp = model.PrecompiledReferences[i];
                    if (allCompatibleManagedDllsByFileName.TryGetValue(precomp, out string? fullPluginPath))
                    {
                        refs.Add(fullPluginPath);
                    }
                }
            }
            else
            {
                refs.AddRange(autoReferencedManagedDlls);
            }

            for (int i = 0; i < rsp.AdditionalReferences.Count; i++)
            {
                string r = rsp.AdditionalReferences[i];
                if (File.Exists(r))
                {
                    refs.Add(Path.GetFullPath(r));
                }
                else if (allCompatibleManagedDllsByFileName.TryGetValue(r, out string? foundPlugin))
                {
                    refs.Add(foundPlugin);
                }
            }

            for (int i = 0; i < directDeps.Count; i++)
            {
                if (compiledDllPathByAssembly.TryGetValue(directDeps[i], out string? depDll))
                {
                    refs.Add(depDll);
                }
            }

            return refs;
        }

        private static bool TryLocateDotnetAndCsc(
            out string dotnetExecutable,
            out string cscDllPath,
            out string? error)
        {
            error = null;
            if (!string.IsNullOrEmpty(_cachedDotnetExecutable)
                && !string.IsNullOrEmpty(_cachedCscDllPath)
                && File.Exists(_cachedDotnetExecutable)
                && File.Exists(_cachedCscDllPath))
            {
                dotnetExecutable = _cachedDotnetExecutable!;
                cscDllPath = _cachedCscDllPath!;
                return true;
            }

            dotnetExecutable = string.Empty;
            cscDllPath = string.Empty;

            List<string> candidateRoots = new List<string>();
            string? envDotnetRoot = Environment.GetEnvironmentVariable("DOTNET_ROOT");
            if (!string.IsNullOrEmpty(envDotnetRoot))
            {
                candidateRoots.Add(envDotnetRoot);
            }

            string userHome = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            if (!string.IsNullOrEmpty(userHome))
            {
                candidateRoots.Add(Path.Combine(userHome, ".dotnet"));
            }

            candidateRoots.Add("/Users/mistlight/.dotnet");
            candidateRoots.Add("/usr/local/share/dotnet");
            candidateRoots.Add("/opt/homebrew/share/dotnet");

            string? runtimeDir = Path.GetDirectoryName(typeof(string).Assembly.Location);
            if (!string.IsNullOrEmpty(runtimeDir))
            {
                DirectoryInfo? dir = new DirectoryInfo(runtimeDir);
                if (dir.Parent?.Parent?.Parent != null)
                {
                    candidateRoots.Add(dir.Parent.Parent.Parent.FullName);
                }
            }

            for (int i = 0; i < candidateRoots.Count; i++)
            {
                string root = candidateRoots[i];
                if (!Directory.Exists(root))
                {
                    continue;
                }

                string exeName = OperatingSystem.IsWindows() ? "dotnet.exe" : "dotnet";
                string candidateExe = Path.Combine(root, exeName);
                string sdkDir = Path.Combine(root, "sdk");
                if (!File.Exists(candidateExe) || !Directory.Exists(sdkDir))
                {
                    continue;
                }

                string[] sdkVersions = Directory.GetDirectories(sdkDir);
                Array.Sort(sdkVersions, StringComparer.Ordinal);
                for (int s = sdkVersions.Length - 1; s >= 0; s--)
                {
                    string candidateCsc = Path.Combine(sdkVersions[s], "Roslyn", "bincore", "csc.dll");
                    if (File.Exists(candidateCsc))
                    {
                        _cachedDotnetExecutable = candidateExe;
                        _cachedCscDllPath = candidateCsc;
                        dotnetExecutable = candidateExe;
                        cscDllPath = candidateCsc;
                        return true;
                    }
                }
            }

            error = "Unable to find dotnet and Roslyn/bincore/csc.dll in DOTNET_ROOT or standard SDK paths.";
            return false;
        }

        private static List<string> DiscoverNetReferencePackDlls()
        {
            List<string> dlls = new List<string>();
            if (TryLocateDotnetAndCsc(out string dotnetExe, out string _, out string? _))
            {
                string? dotnetRoot = Path.GetDirectoryName(dotnetExe);
                if (!string.IsNullOrEmpty(dotnetRoot))
                {
                    string refPackBase = Path.Combine(dotnetRoot, "packs", "Microsoft.NETCore.App.Ref");
                    if (Directory.Exists(refPackBase))
                    {
                        string[] versions = Directory.GetDirectories(refPackBase);
                        Array.Sort(versions, StringComparer.Ordinal);
                        for (int i = versions.Length - 1; i >= 0; i--)
                        {
                            string net8RefDir = Path.Combine(versions[i], "ref", "net8.0");
                            if (Directory.Exists(net8RefDir))
                            {
                                string[] files = Directory.GetFiles(net8RefDir, "*.dll");
                                Array.Sort(files, StringComparer.Ordinal);
                                dlls.AddRange(files);
                                return dlls;
                            }
                        }
                    }
                }
            }

            string? runtimeDir = Path.GetDirectoryName(typeof(string).Assembly.Location);
            if (!string.IsNullOrEmpty(runtimeDir) && Directory.Exists(runtimeDir))
            {
                string[] sysDlls = Directory.GetFiles(runtimeDir, "System.*.dll");
                Array.Sort(sysDlls, StringComparer.Ordinal);
                dlls.AddRange(sysDlls);
                string privCore = Path.Combine(runtimeDir, "System.Private.CoreLib.dll");
                if (File.Exists(privCore))
                {
                    dlls.Add(privCore);
                }

                string netstandard = Path.Combine(runtimeDir, "netstandard.dll");
                if (File.Exists(netstandard))
                {
                    dlls.Add(netstandard);
                }
            }

            return dlls;
        }

        private static string ComputeAssemblyInputHash(
            AssemblyDefinitionModel model,
            List<string> sourceFiles,
            List<string> sortedDefines,
            ResponseFileOptions rsp,
            List<string> directDeps,
            Dictionary<string, string> currentHashes)
        {
            StringBuilder sb = new StringBuilder();
            sb.Append("ASM:").AppendLine(model.Name);
            sb.Append("UNSAFE:").AppendLine((model.AllowUnsafeCode || rsp.AllowUnsafe).ToString(CultureInfo.InvariantCulture));
            sb.Append("DEFINES:").AppendLine(string.Join(";", sortedDefines));
            sb.Append("WARNERR:").AppendLine(rsp.WarningsAsErrors.ToString(CultureInfo.InvariantCulture));

            if (!model.IsPredefined && File.Exists(model.FilePath))
            {
                sb.Append("ASMDEF:").AppendLine(ComputeSha256File(model.FilePath));
            }

            for (int i = 0; i < directDeps.Count; i++)
            {
                string dep = directDeps[i];
                if (currentHashes.TryGetValue(dep, out string? depHash))
                {
                    sb.Append("DEP:").Append(dep).Append('=').AppendLine(depHash);
                }
            }

            for (int i = 0; i < sourceFiles.Count; i++)
            {
                string src = sourceFiles[i];
                sb.Append("SRC:").Append(src).Append('=').AppendLine(ComputeSha256File(src));
            }

            return ComputeSha256Text(sb.ToString());
        }

        private static string ComputeSha256File(string filePath)
        {
            try
            {
                byte[] bytes = File.ReadAllBytes(filePath);
                byte[] hash = SHA256.HashData(bytes);
                return Convert.ToHexString(hash);
            }
            catch (IOException)
            {
                return string.Empty;
            }
        }

        private static string ComputeSha256Text(string text)
        {
            byte[] bytes = Encoding.UTF8.GetBytes(text);
            byte[] hash = SHA256.HashData(bytes);
            return Convert.ToHexString(hash);
        }

        private static Dictionary<string, string> LoadCompilationState(string outputDirectory)
        {
            Dictionary<string, string> map = new Dictionary<string, string>(StringComparer.Ordinal);
            string path = Path.Combine(outputDirectory, ManifestFileName);
            if (!File.Exists(path))
            {
                return map;
            }

            try
            {
                string[] lines = File.ReadAllLines(path);
                for (int i = 0; i < lines.Length; i++)
                {
                    int tab = lines[i].IndexOf('\t', StringComparison.Ordinal);
                    if (tab > 0)
                    {
                        string key = lines[i].Substring(0, tab);
                        string val = lines[i].Substring(tab + 1);
                        map[key] = val;
                    }
                }
            }
            catch (IOException)
            {
            }

            return map;
        }

        private static void SaveCompilationState(string outputDirectory, Dictionary<string, string> hashes)
        {
            string path = Path.Combine(outputDirectory, ManifestFileName);
            List<string> lines = new List<string>();
            foreach (KeyValuePair<string, string> kv in hashes)
            {
                lines.Add(kv.Key + "\t" + kv.Value);
            }

            lines.Sort(StringComparer.Ordinal);
            File.WriteAllLines(path, lines, Encoding.UTF8);
        }

        private static List<string> GetExistingDllsInDirectory(string outputDirectory)
        {
            List<string> dlls = new List<string>();
            if (!Directory.Exists(outputDirectory))
            {
                return dlls;
            }

            string[] files = Directory.GetFiles(outputDirectory, "*.dll", SearchOption.TopDirectoryOnly);
            Array.Sort(files, StringComparer.Ordinal);
            for (int i = 0; i < files.Length; i++)
            {
                dlls.Add(Path.GetFullPath(files[i]));
            }

            return dlls;
        }

        private static List<CompiledAssemblyRecord> BuildRecordsWithoutCompile(
            List<AssemblyDefinitionModel> allAssemblies,
            string outputDirectory,
            Dictionary<string, List<string>> scriptsByAssembly,
            Dictionary<string, HashSet<string>> assemblyDefinesMap,
            Dictionary<string, List<string>> resolvedDeps,
            Dictionary<string, (bool IsExcluded, string Reason)> exclusionMap)
        {
            List<CompiledAssemblyRecord> list = new List<CompiledAssemblyRecord>();
            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                List<string> sources = scriptsByAssembly.TryGetValue(model.Name, out List<string>? s) ? s : new List<string>();
                List<string> defs = assemblyDefinesMap.TryGetValue(model.Name, out HashSet<string>? d) ? new List<string>(d) : new List<string>();
                defs.Sort(StringComparer.Ordinal);
                List<string> deps = resolvedDeps.TryGetValue(model.Name, out List<string>? r) ? r : new List<string>();
                (bool ex, string reason) = exclusionMap.TryGetValue(model.Name, out (bool, string) e) ? e : (false, string.Empty);

                list.Add(new CompiledAssemblyRecord(
                    model.Name,
                    Path.Combine(outputDirectory, model.Name + ".dll"),
                    sources,
                    defs,
                    Array.Empty<string>(),
                    deps,
                    false,
                    ex,
                    reason));
            }

            return list;
        }

        private static bool HasErrorDiagnostics(List<ScriptDiagnostic> diagnostics)
        {
            for (int i = 0; i < diagnostics.Count; i++)
            {
                if (diagnostics[i].Severity == ScriptDiagnosticSeverity.Error)
                {
                    return true;
                }
            }

            return false;
        }
    }
}
