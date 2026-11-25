using System;
using System.Collections.Generic;
using System.Globalization;

namespace Unitas.Scripting
{
    public enum ScriptDiagnosticSeverity
    {
        Info = 0,
        Warning = 1,
        Error = 2
    }

    public enum ActiveAssemblyState
    {
        None = 0,
        Current = 1,
        LastValidFallback = 2
    }

    public enum ScriptTargetPlatform
    {
        StandaloneOSX = 0,
        StandaloneWindows64 = 1,
        StandaloneLinux64 = 2,
        DedicatedServer = 3,
        Android = 4,
        iOS = 5,
        WebGL = 6
    }

    public sealed class ScriptCompilationOptions
    {
        public bool IsEditor { get; set; } = true;

        public bool IsDevelopmentBuild { get; set; }

        public ScriptTargetPlatform TargetPlatform { get; set; } = DetectDefaultHostPlatform();

        public string TargetArchitecture { get; set; } = DetectDefaultHostArchitecture();

        public bool IsProjectTrusted { get; set; } = true;

        public string? OutputDirectory { get; set; }

        public List<string> AdditionalDefines { get; } = new List<string>();

        public List<string> AdditionalReferencePaths { get; } = new List<string>();

        public Dictionary<string, string> PackageVersions { get; } =
            new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        public string UnityVersion { get; set; } = "2022.3.20f1";

        public bool EmitGeneratedGlue { get; set; } = true;

        public bool AutoLoadGeneratedGlue { get; set; }

        private static ScriptTargetPlatform DetectDefaultHostPlatform()
        {
            if (OperatingSystem.IsWindows())
            {
                return ScriptTargetPlatform.StandaloneWindows64;
            }

            if (OperatingSystem.IsLinux())
            {
                return ScriptTargetPlatform.StandaloneLinux64;
            }

            return ScriptTargetPlatform.StandaloneOSX;
        }

        private static string DetectDefaultHostArchitecture()
        {
            string arch = System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture.ToString();
            if (string.Equals(arch, "Arm64", StringComparison.OrdinalIgnoreCase))
            {
                return "ARM64";
            }

            return "x86_64";
        }
    }

    public sealed class ScriptDiagnostic
    {
        public ScriptDiagnostic(
            string id,
            ScriptDiagnosticSeverity severity,
            string message,
            string file,
            int line,
            int column,
            string assembly)
        {
            Id = id ?? string.Empty;
            Severity = severity;
            Message = message ?? string.Empty;
            File = file ?? string.Empty;
            Line = line;
            Column = column;
            Assembly = assembly ?? string.Empty;
        }

        public string Id { get; }

        public ScriptDiagnosticSeverity Severity { get; }

        public string Message { get; }

        public string File { get; }

        public int Line { get; }

        public int Column { get; }

        public string Assembly { get; }

        public string FormatCli()
        {
            string severityLabel = Severity switch
            {
                ScriptDiagnosticSeverity.Error => "error",
                ScriptDiagnosticSeverity.Warning => "warning",
                _ => "info"
            };
            string filePart = string.IsNullOrEmpty(File) ? "unitas" : File;
            string location = Line > 0
                ? string.Concat(
                    filePart,
                    "(",
                    Line.ToString(CultureInfo.InvariantCulture),
                    ",",
                    Math.Max(1, Column).ToString(CultureInfo.InvariantCulture),
                    ")")
                : filePart;
            string assemblySuffix = string.IsNullOrEmpty(Assembly)
                ? string.Empty
                : string.Concat(" [", Assembly, "]");
            return string.Concat(location, ": ", severityLabel, " ", Id, ": ", Message, assemblySuffix);
        }

        public override string ToString() => FormatCli();
    }

    public sealed class VersionDefineRule
    {
        public VersionDefineRule(string name, string expression, string define)
        {
            Name = name ?? string.Empty;
            Expression = expression ?? string.Empty;
            Define = define ?? string.Empty;
        }

        public string Name { get; }

        public string Expression { get; }

        public string Define { get; }
    }

    public sealed class AssemblyDefinitionModel
    {
        public string Name { get; set; } = string.Empty;

        public string RootNamespace { get; set; } = string.Empty;

        public string FilePath { get; set; } = string.Empty;

        public string DirectoryPath { get; set; } = string.Empty;

        public string Guid { get; set; } = string.Empty;

        public List<string> References { get; } = new List<string>();

        public List<string> IncludePlatforms { get; } = new List<string>();

        public List<string> ExcludePlatforms { get; } = new List<string>();

        public bool AllowUnsafeCode { get; set; }

        public bool OverrideReferences { get; set; }

        public List<string> PrecompiledReferences { get; } = new List<string>();

        public bool AutoReferenced { get; set; } = true;

        public bool NoEngineReferences { get; set; }

        public List<string> DefineConstraints { get; } = new List<string>();

        public List<VersionDefineRule> VersionDefines { get; } = new List<VersionDefineRule>();

        public List<string> AdditionalSourceFolders { get; } = new List<string>();

        public bool IsPredefined { get; set; }

        public bool IsEditorOnly { get; set; }
    }

    public sealed class CompiledAssemblyRecord
    {
        public CompiledAssemblyRecord(
            string name,
            string outputPath,
            IReadOnlyList<string> sourceFiles,
            IReadOnlyList<string> defines,
            IReadOnlyList<string> references,
            IReadOnlyList<string> assemblyReferences,
            bool wasRecompiled,
            bool isExcluded,
            string exclusionReason)
        {
            Name = name;
            OutputPath = outputPath;
            SourceFiles = sourceFiles;
            Defines = defines;
            References = references;
            AssemblyReferences = assemblyReferences;
            WasRecompiled = wasRecompiled;
            IsExcluded = isExcluded;
            ExclusionReason = exclusionReason;
        }

        public string Name { get; }

        public string OutputPath { get; }

        public IReadOnlyList<string> SourceFiles { get; }

        public IReadOnlyList<string> Defines { get; }

        public IReadOnlyList<string> References { get; }

        public IReadOnlyList<string> AssemblyReferences { get; }

        public bool WasRecompiled { get; }

        public bool IsExcluded { get; }

        public string ExclusionReason { get; }
    }

    public sealed class ScriptGuidBinding
    {
        public ScriptGuidBinding(
            string guid,
            ulong guidHi,
            ulong guidLo,
            string scriptPath,
            string className,
            string namespaceName,
            string assemblyName,
            IReadOnlyList<ScriptMethodSignature> methods,
            IReadOnlyList<ScriptSerializedField>? serializedFields = null)
        {
            Guid = guid;
            GuidHi = guidHi;
            GuidLo = guidLo;
            ScriptPath = scriptPath;
            ClassName = className;
            NamespaceName = namespaceName;
            AssemblyName = assemblyName;
            Methods = methods;
            SerializedFields = serializedFields ?? Array.Empty<ScriptSerializedField>();
        }

        public string Guid { get; }

        public ulong GuidHi { get; }

        public ulong GuidLo { get; }

        public string ScriptPath { get; }

        public string ClassName { get; }

        public string NamespaceName { get; }

        public string FullTypeName => string.IsNullOrEmpty(NamespaceName)
            ? ClassName
            : string.Concat(NamespaceName, ".", ClassName);

        public string AssemblyName { get; }

        public IReadOnlyList<ScriptMethodSignature> Methods { get; }

        public IReadOnlyList<ScriptSerializedField> SerializedFields { get; }
    }

    public sealed class ScriptSerializedField
    {
        public ScriptSerializedField(
            string name,
            string typeName,
            bool isPublic,
            bool hasSerializeField)
        {
            Name = name;
            TypeName = typeName;
            IsPublic = isPublic;
            HasSerializeField = hasSerializeField;
        }

        public string Name { get; }

        public string TypeName { get; }

        public bool IsPublic { get; }

        public bool HasSerializeField { get; }
    }

    public sealed class SerializableTypeBinding
    {
        public SerializableTypeBinding(
            string typeName,
            string namespaceName,
            string assemblyName,
            bool isValueType,
            IReadOnlyList<ScriptSerializedField> fields)
        {
            TypeName = typeName;
            NamespaceName = namespaceName;
            AssemblyName = assemblyName;
            IsValueType = isValueType;
            Fields = fields;
        }

        public string TypeName { get; }

        public string NamespaceName { get; }

        public string FullTypeName => string.IsNullOrEmpty(NamespaceName)
            ? TypeName
            : string.Concat(NamespaceName, ".", TypeName);

        public string AssemblyName { get; }

        public bool IsValueType { get; }

        public IReadOnlyList<ScriptSerializedField> Fields { get; }
    }

    public sealed class ScriptMethodSignature
    {
        public ScriptMethodSignature(
            string name,
            bool isPublic,
            int parameterCount,
            string? parameterType,
            bool returnsVoidOrEnumerator)
        {
            Name = name;
            IsPublic = isPublic;
            ParameterCount = parameterCount;
            ParameterType = parameterType;
            ReturnsVoidOrEnumerator = returnsVoidOrEnumerator;
        }

        public string Name { get; }

        public bool IsPublic { get; }

        public int ParameterCount { get; }

        public string? ParameterType { get; }

        public bool ReturnsVoidOrEnumerator { get; }
    }

    public sealed class DiscoveredPluginRecord
    {
        public DiscoveredPluginRecord(
            string guid,
            ulong guidHi,
            ulong guidLo,
            string path,
            string relativePath,
            string libraryName,
            bool isNative,
            bool isCompatible,
            bool isIncludedForTarget,
            string? incompatibilityDiagnostic)
        {
            Guid = guid;
            GuidHi = guidHi;
            GuidLo = guidLo;
            Path = path;
            RelativePath = relativePath;
            LibraryName = libraryName;
            IsNative = isNative;
            IsCompatible = isCompatible;
            IsIncludedForTarget = isIncludedForTarget;
            IncompatibilityDiagnostic = incompatibilityDiagnostic;
        }

        public string Guid { get; }

        public ulong GuidHi { get; }

        public ulong GuidLo { get; }

        public string Path { get; }

        public string RelativePath { get; }

        public string LibraryName { get; }

        public bool IsNative { get; }

        public bool IsCompatible { get; }

        public bool IsIncludedForTarget { get; }

        public string? IncompatibilityDiagnostic { get; }
    }

    public sealed class ScriptCompilationResult
    {
        public ScriptCompilationResult(
            bool succeeded,
            ActiveAssemblyState activeState,
            string outputDirectory,
            string? generatedGlueAssemblyPath,
            string? generatedGlueSourcePath,
            IReadOnlyList<CompiledAssemblyRecord> assemblies,
            IReadOnlyList<ScriptDiagnostic> diagnostics,
            IReadOnlyList<string> recompiledAssemblies,
            IReadOnlyList<string> activeAssemblyPaths,
            IReadOnlyList<ScriptGuidBinding> scriptBindings,
            IReadOnlyList<DiscoveredPluginRecord> plugins)
        {
            Succeeded = succeeded;
            ActiveState = activeState;
            OutputDirectory = outputDirectory;
            GeneratedGlueAssemblyPath = generatedGlueAssemblyPath;
            GeneratedGlueSourcePath = generatedGlueSourcePath;
            Assemblies = assemblies;
            Diagnostics = diagnostics;
            RecompiledAssemblies = recompiledAssemblies;
            ActiveAssemblyPaths = activeAssemblyPaths;
            ScriptBindings = scriptBindings;
            Plugins = plugins;
        }

        public bool Succeeded { get; }

        public ActiveAssemblyState ActiveState { get; }

        public string OutputDirectory { get; }

        public string? GeneratedGlueAssemblyPath { get; }

        public string? GeneratedGlueSourcePath { get; }

        public IReadOnlyList<CompiledAssemblyRecord> Assemblies { get; }

        public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; }

        public IReadOnlyList<string> RecompiledAssemblies { get; }

        public IReadOnlyList<string> ActiveAssemblyPaths { get; }

        public IReadOnlyList<ScriptGuidBinding> ScriptBindings { get; }

        public IReadOnlyList<DiscoveredPluginRecord> Plugins { get; }

        public CompiledAssemblyRecord? FindAssembly(string name)
        {
            for (int index = 0; index < Assemblies.Count; index++)
            {
                if (string.Equals(Assemblies[index].Name, name, StringComparison.Ordinal))
                {
                    return Assemblies[index];
                }
            }

            return null;
        }
    }
}
