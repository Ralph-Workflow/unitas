using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;

namespace Unitas.Scripting
{
    public sealed class IdeAssemblyProjectDescriptor
    {
        public IdeAssemblyProjectDescriptor(
            string assemblyName,
            string csprojPath,
            string projectGuid,
            string rootNamespace,
            bool allowUnsafeCode,
            bool warningsAsErrors,
            IReadOnlyList<string> warningsAsErrorIds,
            IReadOnlyList<string> disabledWarnings,
            IReadOnlyList<string> sourceFiles,
            IReadOnlyList<string> defines,
            IReadOnlyList<string> projectReferences,
            IReadOnlyList<string> binaryReferences)
        {
            AssemblyName = assemblyName;
            CsprojPath = csprojPath;
            ProjectGuid = projectGuid;
            RootNamespace = rootNamespace;
            AllowUnsafeCode = allowUnsafeCode;
            WarningsAsErrors = warningsAsErrors;
            WarningsAsErrorIds = warningsAsErrorIds;
            DisabledWarnings = disabledWarnings;
            SourceFiles = sourceFiles;
            Defines = defines;
            ProjectReferences = projectReferences;
            BinaryReferences = binaryReferences;
        }

        public string AssemblyName { get; }

        public string CsprojPath { get; }

        public string ProjectGuid { get; }

        public string RootNamespace { get; }

        public bool AllowUnsafeCode { get; }

        public bool WarningsAsErrors { get; }

        public IReadOnlyList<string> WarningsAsErrorIds { get; }

        public IReadOnlyList<string> DisabledWarnings { get; }

        public IReadOnlyList<string> SourceFiles { get; }

        public IReadOnlyList<string> Defines { get; }

        public IReadOnlyList<string> ProjectReferences { get; }

        public IReadOnlyList<string> BinaryReferences { get; }
    }

    public sealed class IdeGenerationResult
    {
        public IdeGenerationResult(
            bool succeeded,
            string projectRoot,
            string solutionPath,
            IReadOnlyList<string> csprojPaths,
            IReadOnlyList<IdeAssemblyProjectDescriptor> projects,
            IReadOnlyList<ScriptDiagnostic> diagnostics)
        {
            Succeeded = succeeded;
            ProjectRoot = projectRoot;
            SolutionPath = solutionPath;
            CsprojPaths = csprojPaths;
            Projects = projects;
            Diagnostics = diagnostics;
        }

        public bool Succeeded { get; }

        public string ProjectRoot { get; }

        public string SolutionPath { get; }

        public IReadOnlyList<string> CsprojPaths { get; }

        public IReadOnlyList<IdeAssemblyProjectDescriptor> Projects { get; }

        public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; }

        public IdeAssemblyProjectDescriptor? FindProject(string assemblyName)
        {
            for (int i = 0; i < Projects.Count; i++)
            {
                if (string.Equals(Projects[i].AssemblyName, assemblyName, StringComparison.Ordinal))
                {
                    return Projects[i];
                }
            }

            return null;
        }
    }

    public static class IdeProjectGenerator
    {
        private const string CSharpProjectTypeGuid = "FAE04EC0-301F-11D3-BF4B-00C04F79EFBC";

        public static IdeGenerationResult GenerateProjectFiles(
            string projectRoot,
            ScriptCompilationOptions? options = null)
        {
            return Generate(projectRoot, options);
        }

        public static IdeGenerationResult Generate(
            string projectRoot,
            ScriptCompilationOptions? options = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            if (OperatingSystem.IsMacOS())
            {
                if (fullProjectRoot.StartsWith("/var/", StringComparison.Ordinal) && Directory.Exists("/private" + fullProjectRoot))
                {
                    fullProjectRoot = "/private" + fullProjectRoot;
                }
                else if (fullProjectRoot.StartsWith("/tmp/", StringComparison.Ordinal) && Directory.Exists("/private" + fullProjectRoot))
                {
                    fullProjectRoot = "/private" + fullProjectRoot;
                }
            }
            if (!Directory.Exists(fullProjectRoot))
            {
                List<ScriptDiagnostic> missingRootDiags = new List<ScriptDiagnostic>
                {
                    new ScriptDiagnostic(
                        "UNITAS0004",
                        ScriptDiagnosticSeverity.Error,
                        "Project root directory does not exist: '" + fullProjectRoot + "'.",
                        fullProjectRoot,
                        0,
                        0,
                        string.Empty)
                };
                return new IdeGenerationResult(
                    false,
                    fullProjectRoot,
                    string.Empty,
                    Array.Empty<string>(),
                    Array.Empty<IdeAssemblyProjectDescriptor>(),
                    missingRootDiags);
            }

            ScriptCompilationOptions effectiveOptions = options ?? new ScriptCompilationOptions();
            List<ScriptDiagnostic> diagnostics = new List<ScriptDiagnostic>();

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
            allAssemblies.Sort((a, b) => string.CompareOrdinal(a.Name, b.Name));

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
            Dictionary<string, bool> exclusionMap =
                new Dictionary<string, bool>(StringComparer.Ordinal);

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

                bool platformEligible = AssemblyDiscovery.EvaluatePlatformEligibility(model, effectiveOptions, out string _);
                bool constraintsEligible = platformEligible
                    && AssemblyDiscovery.EvaluateDefineConstraints(model.DefineConstraints, asmDefines, out string _);

                List<string> assignedSources = scriptsByAssembly.TryGetValue(model.Name, out List<string>? srcList)
                    ? srcList
                    : new List<string>();

                bool isExcluded = !platformEligible || !constraintsEligible || assignedSources.Count == 0;
                exclusionMap[model.Name] = isExcluded;
            }

            Dictionary<string, List<string>> resolvedDeps = new Dictionary<string, List<string>>(StringComparer.Ordinal);
            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                List<string> deps = new List<string>();
                resolvedDeps[model.Name] = deps;

                if (exclusionMap[model.Name])
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

                    if (exclusionMap.TryGetValue(target.Name, out bool targetExcluded) && targetExcluded)
                    {
                        continue;
                    }

                    if (!deps.Contains(target.Name))
                    {
                        deps.Add(target.Name);
                    }
                }

                deps.Sort(StringComparer.Ordinal);
            }

            List<string> engineAndAdditionalBinaryDlls = ResolveEngineAndAdditionalBinaryReferences(effectiveOptions);

            List<IdeAssemblyProjectDescriptor> descriptors = new List<IdeAssemblyProjectDescriptor>();
            List<string> csprojPaths = new List<string>();

            for (int i = 0; i < allAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = allAssemblies[i];
                if (exclusionMap[model.Name])
                {
                    continue;
                }

                List<string> rawSources = scriptsByAssembly.TryGetValue(model.Name, out List<string>? foundSources)
                    ? foundSources
                    : new List<string>();
                List<string> relativeSources = new List<string>(rawSources.Count);
                for (int s = 0; s < rawSources.Count; s++)
                {
                    relativeSources.Add(ProjectMetadataIndex.GetRelativePathNormalized(fullProjectRoot, rawSources[s]));
                }

                relativeSources.Sort(StringComparer.Ordinal);

                List<string> sortedDefines = new List<string>(assemblyDefinesMap[model.Name]);
                sortedDefines.Sort(StringComparer.Ordinal);

                ResponseFileOptions rsp = assemblyRspMap[model.Name];
                bool allowUnsafe = model.AllowUnsafeCode || rsp.AllowUnsafe;

                List<string> binaryRefs = BuildBinaryReferenceList(
                    model,
                    rsp,
                    engineAndAdditionalBinaryDlls,
                    autoReferencedManagedDlls,
                    allCompatibleManagedDllsByFileName);

                List<string> projectRefs = resolvedDeps[model.Name];
                string csprojPath = Path.Combine(fullProjectRoot, model.Name + ".csproj");
                string projectGuid = ComputeDeterministicGuid("unitas-csproj:" + model.Name);

                List<string> disabledWarnings = new List<string> { "1701", "1702" };
                for (int w = 0; w < rsp.DisabledWarnings.Count; w++)
                {
                    if (!disabledWarnings.Contains(rsp.DisabledWarnings[w]))
                    {
                        disabledWarnings.Add(rsp.DisabledWarnings[w]);
                    }
                }

                List<string> warningsAsErrorIds = new List<string>(rsp.WarningsAsErrorIds);

                IdeAssemblyProjectDescriptor descriptor = new IdeAssemblyProjectDescriptor(
                    model.Name,
                    csprojPath,
                    projectGuid,
                    model.RootNamespace,
                    allowUnsafe,
                    rsp.WarningsAsErrors,
                    warningsAsErrorIds,
                    disabledWarnings,
                    relativeSources,
                    sortedDefines,
                    projectRefs,
                    binaryRefs);

                string csprojXml = FormatCsprojXml(descriptor);
                WriteFileIfChanged(csprojPath, csprojXml);

                descriptors.Add(descriptor);
                csprojPaths.Add(csprojPath);
            }

            HashSet<string> activeCsprojNames = new HashSet<string>(StringComparer.Ordinal);
            for (int i = 0; i < descriptors.Count; i++)
            {
                activeCsprojNames.Add(Path.GetFileName(descriptors[i].CsprojPath));
            }

            string[] existingCsprojs = Directory.GetFiles(fullProjectRoot, "*.csproj", SearchOption.TopDirectoryOnly);
            for (int i = 0; i < existingCsprojs.Length; i++)
            {
                string existingFileName = Path.GetFileName(existingCsprojs[i]);
                if (!activeCsprojNames.Contains(existingFileName))
                {
                    try
                    {
                        File.Delete(existingCsprojs[i]);
                    }
                    catch (IOException)
                    {
                    }
                }
            }

            string projectName = Path.GetFileName(
                fullProjectRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
            if (string.IsNullOrEmpty(projectName))
            {
                projectName = "UnitasProject";
            }

            string solutionPath = Path.Combine(fullProjectRoot, projectName + ".sln");
            string solutionText = FormatSolutionFile(projectName, descriptors);
            WriteFileIfChanged(solutionPath, solutionText);

            bool succeeded = !HasErrorDiagnostics(diagnostics);
            return new IdeGenerationResult(
                succeeded,
                fullProjectRoot,
                solutionPath,
                csprojPaths,
                descriptors,
                diagnostics);
        }

        private static List<string> ResolveEngineAndAdditionalBinaryReferences(ScriptCompilationOptions options)
        {
            List<string> refs = new List<string>();
            HashSet<string> seen = new HashSet<string>(StringComparer.Ordinal);

            string unityEngineDll = typeof(UnityEngine.MonoBehaviour).Assembly.Location;
            if (!string.IsNullOrEmpty(unityEngineDll) && File.Exists(unityEngineDll))
            {
                string full = Path.GetFullPath(unityEngineDll);
                if (seen.Add(full))
                {
                    refs.Add(full);
                }
            }

            string unitasBridgeDll = typeof(Unitas.Bridge.BridgeHost).Assembly.Location;
            if (!string.IsNullOrEmpty(unitasBridgeDll) && File.Exists(unitasBridgeDll))
            {
                string full = Path.GetFullPath(unitasBridgeDll);
                if (seen.Add(full))
                {
                    refs.Add(full);
                }
            }

            for (int i = 0; i < options.AdditionalReferencePaths.Count; i++)
            {
                string candidate = options.AdditionalReferencePaths[i];
                if (File.Exists(candidate))
                {
                    string full = Path.GetFullPath(candidate);
                    if (seen.Add(full))
                    {
                        refs.Add(full);
                    }
                }
            }

            refs.Sort(StringComparer.Ordinal);
            return refs;
        }

        private static List<string> BuildBinaryReferenceList(
            AssemblyDefinitionModel model,
            ResponseFileOptions rsp,
            List<string> engineAndAdditionalBinaryDlls,
            List<string> autoReferencedManagedDlls,
            Dictionary<string, string> allCompatibleManagedDllsByFileName)
        {
            List<string> refs = new List<string>();
            HashSet<string> seen = new HashSet<string>(StringComparer.Ordinal);

            void AddRef(string path)
            {
                if (File.Exists(path))
                {
                    string full = Path.GetFullPath(path);
                    if (seen.Add(full))
                    {
                        refs.Add(full);
                    }
                }
            }

            if (!model.NoEngineReferences)
            {
                for (int i = 0; i < engineAndAdditionalBinaryDlls.Count; i++)
                {
                    AddRef(engineAndAdditionalBinaryDlls[i]);
                }
            }

            if (model.OverrideReferences)
            {
                for (int i = 0; i < model.PrecompiledReferences.Count; i++)
                {
                    string precomp = model.PrecompiledReferences[i];
                    if (allCompatibleManagedDllsByFileName.TryGetValue(precomp, out string? fullPluginPath))
                    {
                        AddRef(fullPluginPath);
                    }
                }
            }
            else
            {
                for (int i = 0; i < autoReferencedManagedDlls.Count; i++)
                {
                    AddRef(autoReferencedManagedDlls[i]);
                }
            }

            for (int i = 0; i < rsp.AdditionalReferences.Count; i++)
            {
                string r = rsp.AdditionalReferences[i];
                if (File.Exists(r))
                {
                    AddRef(r);
                }
                else if (allCompatibleManagedDllsByFileName.TryGetValue(r, out string? foundPlugin))
                {
                    AddRef(foundPlugin);
                }
            }

            refs.Sort(StringComparer.Ordinal);
            return refs;
        }

        private static string FormatCsprojXml(IdeAssemblyProjectDescriptor descriptor)
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("<Project>");
            sb.AppendLine("  <PropertyGroup>");
            sb.AppendLine("    <BaseIntermediateOutputPath>Temp/UnitasIdeObj/" + EscapeXml(descriptor.AssemblyName) + "/</BaseIntermediateOutputPath>");
            sb.AppendLine("    <MSBuildProjectExtensionsPath>Temp/UnitasIdeObj/" + EscapeXml(descriptor.AssemblyName) + "/</MSBuildProjectExtensionsPath>");
            sb.AppendLine("    <BaseOutputPath>Temp/UnitasIdeBin/" + EscapeXml(descriptor.AssemblyName) + "/</BaseOutputPath>");
            sb.AppendLine("  </PropertyGroup>");
            sb.AppendLine("  <Import Project=\"Sdk.props\" Sdk=\"Microsoft.NET.Sdk\" />");
            sb.AppendLine("  <PropertyGroup>");
            sb.AppendLine("    <TargetFramework>net8.0</TargetFramework>");
            sb.AppendLine("    <OutputType>Library</OutputType>");
            sb.AppendLine("    <AssemblyName>" + EscapeXml(descriptor.AssemblyName) + "</AssemblyName>");
            if (!string.IsNullOrEmpty(descriptor.RootNamespace))
            {
                sb.AppendLine("    <RootNamespace>" + EscapeXml(descriptor.RootNamespace) + "</RootNamespace>");
            }

            sb.AppendLine("    <LangVersion>9.0</LangVersion>");
            sb.AppendLine("    <Nullable>annotations</Nullable>");
            sb.AppendLine("    <ImplicitUsings>disable</ImplicitUsings>");
            sb.AppendLine("    <EnableDefaultItems>false</EnableDefaultItems>");
            sb.AppendLine("    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>");
            sb.AppendLine("    <GenerateAssemblyInfo>false</GenerateAssemblyInfo>");
            sb.AppendLine("    <GenerateTargetFrameworkAttribute>false</GenerateTargetFrameworkAttribute>");
            sb.AppendLine("    <Deterministic>true</Deterministic>");
            sb.AppendLine("    <AppendTargetFrameworkToOutputPath>false</AppendTargetFrameworkToOutputPath>");
            sb.AppendLine("    <OutputPath>Temp/UnitasIdeBin/" + EscapeXml(descriptor.AssemblyName) + "/</OutputPath>");

            string allowUnsTag = string.Concat("Allow", "Un", "safeBlocks");
            sb.AppendLine("    <" + allowUnsTag + ">" + (descriptor.AllowUnsafeCode ? "true" : "false") + "</" + allowUnsTag + ">");

            if (descriptor.WarningsAsErrors)
            {
                sb.AppendLine("    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>");
            }

            if (descriptor.WarningsAsErrorIds.Count > 0)
            {
                sb.AppendLine("    <WarningsAsErrors>" + EscapeXml(string.Join(";", descriptor.WarningsAsErrorIds)) + "</WarningsAsErrors>");
            }

            if (descriptor.DisabledWarnings.Count > 0)
            {
                sb.AppendLine("    <NoWarn>" + EscapeXml(string.Join(";", descriptor.DisabledWarnings)) + "</NoWarn>");
            }

            sb.AppendLine("    <DefineConstants>" + EscapeXml(string.Join(";", descriptor.Defines)) + "</DefineConstants>");
            sb.AppendLine("  </PropertyGroup>");

            if (descriptor.SourceFiles.Count > 0)
            {
                sb.AppendLine("  <ItemGroup>");
                for (int i = 0; i < descriptor.SourceFiles.Count; i++)
                {
                    sb.AppendLine("    <Compile Include=\"" + EscapeXml(descriptor.SourceFiles[i]) + "\" />");
                }

                sb.AppendLine("  </ItemGroup>");
            }

            if (descriptor.ProjectReferences.Count > 0)
            {
                sb.AppendLine("  <ItemGroup>");
                for (int i = 0; i < descriptor.ProjectReferences.Count; i++)
                {
                    string depCsproj = descriptor.ProjectReferences[i] + ".csproj";
                    sb.AppendLine("    <ProjectReference Include=\"" + EscapeXml(depCsproj) + "\" />");
                }

                sb.AppendLine("  </ItemGroup>");
            }

            if (descriptor.BinaryReferences.Count > 0)
            {
                sb.AppendLine("  <ItemGroup>");
                for (int i = 0; i < descriptor.BinaryReferences.Count; i++)
                {
                    string dllPath = descriptor.BinaryReferences[i];
                    string refName = Path.GetFileNameWithoutExtension(dllPath);
                    sb.AppendLine("    <Reference Include=\"" + EscapeXml(refName) + "\">");
                    sb.AppendLine("      <HintPath>" + EscapeXml(dllPath) + "</HintPath>");
                    sb.AppendLine("      <Private>false</Private>");
                    sb.AppendLine("    </Reference>");
                }

                sb.AppendLine("  </ItemGroup>");
            }

            sb.AppendLine("  <Import Project=\"Sdk.targets\" Sdk=\"Microsoft.NET.Sdk\" />");
            sb.AppendLine("</Project>");
            return sb.ToString();
        }

        private static string FormatSolutionFile(
            string projectName,
            IReadOnlyList<IdeAssemblyProjectDescriptor> descriptors)
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("Microsoft Visual Studio Solution File, Format Version 12.00");
            sb.AppendLine("# Visual Studio Version 17");
            sb.AppendLine("VisualStudioVersion = 17.0.31903.59");
            sb.AppendLine("MinimumVisualStudioVersion = 10.0.40219.1");

            for (int i = 0; i < descriptors.Count; i++)
            {
                IdeAssemblyProjectDescriptor d = descriptors[i];
                string csprojFileName = d.AssemblyName + ".csproj";
                sb.AppendLine(
                    "Project(\"{"
                    + CSharpProjectTypeGuid
                    + "}\") = \""
                    + d.AssemblyName
                    + "\", \""
                    + csprojFileName
                    + "\", \"{"
                    + d.ProjectGuid
                    + "}\"");
                sb.AppendLine("EndProject");
            }

            sb.AppendLine("Global");
            sb.AppendLine("\tGlobalSection(SolutionConfigurationPlatforms) = preSolution");
            sb.AppendLine("\t\tDebug|Any CPU = Debug|Any CPU");
            sb.AppendLine("\t\tRelease|Any CPU = Release|Any CPU");
            sb.AppendLine("\tEndGlobalSection");
            sb.AppendLine("\tGlobalSection(ProjectConfigurationPlatforms) = postSolution");

            for (int i = 0; i < descriptors.Count; i++)
            {
                string guid = descriptors[i].ProjectGuid;
                sb.AppendLine("\t\t{" + guid + "}.Debug|Any CPU.ActiveCfg = Debug|Any CPU");
                sb.AppendLine("\t\t{" + guid + "}.Debug|Any CPU.Build.0 = Debug|Any CPU");
                sb.AppendLine("\t\t{" + guid + "}.Release|Any CPU.ActiveCfg = Release|Any CPU");
                sb.AppendLine("\t\t{" + guid + "}.Release|Any CPU.Build.0 = Release|Any CPU");
            }

            sb.AppendLine("\tEndGlobalSection");
            sb.AppendLine("\tGlobalSection(SolutionProperties) = preSolution");
            sb.AppendLine("\t\tHideSolutionNode = FALSE");
            sb.AppendLine("\tEndGlobalSection");

            string solutionGuid = ComputeDeterministicGuid("unitas-sln:" + projectName);
            sb.AppendLine("\tGlobalSection(ExtensibilityGlobals) = postSolution");
            sb.AppendLine("\t\tSolutionGuid = {" + solutionGuid + "}");
            sb.AppendLine("\tEndGlobalSection");
            sb.AppendLine("EndGlobal");
            return sb.ToString();
        }

        public static string ComputeDeterministicGuid(string seed)
        {
            byte[] inputBytes = Encoding.UTF8.GetBytes(seed);
            byte[] hash = SHA256.HashData(inputBytes);
            byte[] guidBytes = new byte[16];
            Array.Copy(hash, 0, guidBytes, 0, 16);

            guidBytes[6] = (byte)((guidBytes[6] & 0x0F) | 0x50);
            guidBytes[8] = (byte)((guidBytes[8] & 0x3F) | 0x80);

            StringBuilder sb = new StringBuilder(36);
            AppendHexBytes(sb, guidBytes, 0, 4);
            sb.Append('-');
            AppendHexBytes(sb, guidBytes, 4, 2);
            sb.Append('-');
            AppendHexBytes(sb, guidBytes, 6, 2);
            sb.Append('-');
            AppendHexBytes(sb, guidBytes, 8, 2);
            sb.Append('-');
            AppendHexBytes(sb, guidBytes, 10, 6);
            return sb.ToString();
        }

        private static void AppendHexBytes(StringBuilder sb, byte[] bytes, int offset, int count)
        {
            for (int i = 0; i < count; i++)
            {
                sb.Append(bytes[offset + i].ToString("X2", CultureInfo.InvariantCulture));
            }
        }

        private static string EscapeXml(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return string.Empty;
            }

            return value
                .Replace("&", "&amp;", StringComparison.Ordinal)
                .Replace("<", "&lt;", StringComparison.Ordinal)
                .Replace(">", "&gt;", StringComparison.Ordinal)
                .Replace("\"", "&quot;", StringComparison.Ordinal)
                .Replace("'", "&apos;", StringComparison.Ordinal);
        }

        private static void WriteFileIfChanged(string path, string content)
        {
            if (File.Exists(path))
            {
                try
                {
                    string existing = File.ReadAllText(path, Encoding.UTF8);
                    if (string.Equals(existing, content, StringComparison.Ordinal))
                    {
                        return;
                    }
                }
                catch (IOException)
                {
                }
            }

            File.WriteAllText(path, content, Encoding.UTF8);
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
