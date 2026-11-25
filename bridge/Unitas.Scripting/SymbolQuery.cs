using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Xml.Linq;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace Unitas.Scripting
{
    public sealed class SymbolMemberRecord
    {
        public SymbolMemberRecord(
            string name,
            string kind,
            string accessibility,
            bool isStatic,
            string returnOrValueType,
            string signature,
            IReadOnlyList<string> attributes)
        {
            Name = name ?? string.Empty;
            Kind = kind ?? string.Empty;
            Accessibility = accessibility ?? string.Empty;
            IsStatic = isStatic;
            ReturnOrValueType = returnOrValueType ?? string.Empty;
            Signature = signature ?? string.Empty;
            Attributes = attributes;
        }

        public string Name { get; }

        public string Kind { get; }

        public string Accessibility { get; }

        public bool IsStatic { get; }

        public string ReturnOrValueType { get; }

        public string Signature { get; }

        public IReadOnlyList<string> Attributes { get; }
    }

    public sealed class SerializedFieldRecord
    {
        public SerializedFieldRecord(
            string name,
            string fieldType,
            string declaringType,
            string accessibility,
            bool hasSerializeField,
            bool isSerializeReference,
            IReadOnlyList<string> formerlySerializedAs,
            IReadOnlyList<string> attributes)
        {
            Name = name ?? string.Empty;
            FieldType = fieldType ?? string.Empty;
            DeclaringType = declaringType ?? string.Empty;
            Accessibility = accessibility ?? string.Empty;
            HasSerializeField = hasSerializeField;
            IsSerializeReference = isSerializeReference;
            FormerlySerializedAs = formerlySerializedAs;
            Attributes = attributes;
        }

        public string Name { get; }

        public string FieldType { get; }

        public string DeclaringType { get; }

        public string Accessibility { get; }

        public bool HasSerializeField { get; }

        public bool IsSerializeReference { get; }

        public IReadOnlyList<string> FormerlySerializedAs { get; }

        public IReadOnlyList<string> Attributes { get; }
    }

    public sealed class SymbolTypeRecord
    {
        public SymbolTypeRecord(
            string name,
            string namespaceName,
            string fullName,
            string kind,
            string accessibility,
            string assembly,
            string filePath,
            int line,
            string baseType,
            IReadOnlyList<string> baseTypeChain,
            IReadOnlyList<string> interfaces,
            IReadOnlyList<string> attributes,
            bool isMonoBehaviour,
            bool isScriptableObject,
            bool isSerializableType,
            IReadOnlyList<SymbolMemberRecord> members,
            IReadOnlyList<SerializedFieldRecord> serializedFields)
        {
            Name = name ?? string.Empty;
            NamespaceName = namespaceName ?? string.Empty;
            FullName = fullName ?? string.Empty;
            Kind = kind ?? string.Empty;
            Accessibility = accessibility ?? string.Empty;
            Assembly = assembly ?? string.Empty;
            FilePath = filePath ?? string.Empty;
            Line = line;
            BaseType = baseType ?? string.Empty;
            BaseTypeChain = baseTypeChain;
            Interfaces = interfaces;
            Attributes = attributes;
            IsMonoBehaviour = isMonoBehaviour;
            IsScriptableObject = isScriptableObject;
            IsSerializableType = isSerializableType;
            Members = members;
            SerializedFields = serializedFields;
        }

        public string Name { get; }

        public string NamespaceName { get; }

        public string FullName { get; }

        public string Kind { get; }

        public string Accessibility { get; }

        public string Assembly { get; }

        public string FilePath { get; }

        public int Line { get; }

        public string BaseType { get; }

        public IReadOnlyList<string> BaseTypeChain { get; }

        public IReadOnlyList<string> Interfaces { get; }

        public IReadOnlyList<string> Attributes { get; }

        public bool IsMonoBehaviour { get; }

        public bool IsScriptableObject { get; }

        public bool IsSerializableType { get; }

        public IReadOnlyList<SymbolMemberRecord> Members { get; }

        public IReadOnlyList<SerializedFieldRecord> SerializedFields { get; }

        public SymbolMemberRecord? FindMember(string memberName)
        {
            for (int i = 0; i < Members.Count; i++)
            {
                if (string.Equals(Members[i].Name, memberName, StringComparison.Ordinal))
                {
                    return Members[i];
                }
            }

            return null;
        }

        public SerializedFieldRecord? FindSerializedField(string fieldName)
        {
            for (int i = 0; i < SerializedFields.Count; i++)
            {
                if (string.Equals(SerializedFields[i].Name, fieldName, StringComparison.Ordinal))
                {
                    return SerializedFields[i];
                }
            }

            return null;
        }
    }

    public sealed class SymbolQueryResult
    {
        public SymbolQueryResult(
            string projectRoot,
            string? typeFilter,
            IReadOnlyList<SymbolTypeRecord> types,
            IReadOnlyList<ScriptDiagnostic> diagnostics)
        {
            ProjectRoot = projectRoot ?? string.Empty;
            TypeFilter = typeFilter;
            Types = types;
            Diagnostics = diagnostics;
        }

        public string ProjectRoot { get; }

        public string? TypeFilter { get; }

        public IReadOnlyList<SymbolTypeRecord> Types { get; }

        public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; }

        public SymbolTypeRecord? FindType(string nameOrFullName)
        {
            for (int i = 0; i < Types.Count; i++)
            {
                SymbolTypeRecord t = Types[i];
                if (string.Equals(t.FullName, nameOrFullName, StringComparison.Ordinal)
                    || string.Equals(t.Name, nameOrFullName, StringComparison.Ordinal))
                {
                    return t;
                }
            }

            return null;
        }

        public string ToJson()
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("{");
            sb.Append("  \"projectRoot\": \"").Append(DiagnoseReport.EscapeJson(ProjectRoot)).AppendLine("\",");
            if (TypeFilter != null)
            {
                sb.Append("  \"typeFilter\": \"").Append(DiagnoseReport.EscapeJson(TypeFilter)).AppendLine("\",");
            }
            else
            {
                sb.AppendLine("  \"typeFilter\": null,");
            }

            sb.Append("  \"types\": [");
            if (Types.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < Types.Count; i++)
                {
                    SymbolTypeRecord t = Types[i];
                    sb.AppendLine("    {");
                    sb.Append("      \"name\": \"").Append(DiagnoseReport.EscapeJson(t.Name)).AppendLine("\",");
                    sb.Append("      \"namespace\": \"").Append(DiagnoseReport.EscapeJson(t.NamespaceName)).AppendLine("\",");
                    sb.Append("      \"fullName\": \"").Append(DiagnoseReport.EscapeJson(t.FullName)).AppendLine("\",");
                    sb.Append("      \"kind\": \"").Append(DiagnoseReport.EscapeJson(t.Kind)).AppendLine("\",");
                    sb.Append("      \"accessibility\": \"").Append(DiagnoseReport.EscapeJson(t.Accessibility)).AppendLine("\",");
                    sb.Append("      \"assembly\": \"").Append(DiagnoseReport.EscapeJson(t.Assembly)).AppendLine("\",");
                    sb.Append("      \"file\": \"").Append(DiagnoseReport.EscapeJson(t.FilePath)).AppendLine("\",");
                    sb.Append("      \"line\": ").Append(t.Line.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append("      \"baseType\": \"").Append(DiagnoseReport.EscapeJson(t.BaseType)).AppendLine("\",");
                    AppendJsonStringArray(sb, "      ", "baseTypeChain", t.BaseTypeChain, true);
                    AppendJsonStringArray(sb, "      ", "interfaces", t.Interfaces, true);
                    AppendJsonStringArray(sb, "      ", "attributes", t.Attributes, true);
                    sb.Append("      \"isMonoBehaviour\": ").Append(t.IsMonoBehaviour ? "true" : "false").AppendLine(",");
                    sb.Append("      \"isScriptableObject\": ").Append(t.IsScriptableObject ? "true" : "false").AppendLine(",");
                    sb.Append("      \"isSerializableType\": ").Append(t.IsSerializableType ? "true" : "false").AppendLine(",");

                    sb.Append("      \"members\": [");
                    if (t.Members.Count > 0)
                    {
                        sb.AppendLine();
                        for (int m = 0; m < t.Members.Count; m++)
                        {
                            SymbolMemberRecord mem = t.Members[m];
                            sb.AppendLine("        {");
                            sb.Append("          \"name\": \"").Append(DiagnoseReport.EscapeJson(mem.Name)).AppendLine("\",");
                            sb.Append("          \"kind\": \"").Append(DiagnoseReport.EscapeJson(mem.Kind)).AppendLine("\",");
                            sb.Append("          \"accessibility\": \"").Append(DiagnoseReport.EscapeJson(mem.Accessibility)).AppendLine("\",");
                            sb.Append("          \"isStatic\": ").Append(mem.IsStatic ? "true" : "false").AppendLine(",");
                            sb.Append("          \"type\": \"").Append(DiagnoseReport.EscapeJson(mem.ReturnOrValueType)).AppendLine("\",");
                            sb.Append("          \"signature\": \"").Append(DiagnoseReport.EscapeJson(mem.Signature)).AppendLine("\",");
                            AppendJsonStringArray(sb, "          ", "attributes", mem.Attributes, false);
                            sb.Append("        }");
                            if (m + 1 < t.Members.Count)
                            {
                                sb.Append(',');
                            }

                            sb.AppendLine();
                        }

                        sb.Append("      ");
                    }

                    sb.AppendLine("],");

                    sb.Append("      \"serializedFields\": [");
                    if (t.SerializedFields.Count > 0)
                    {
                        sb.AppendLine();
                        for (int f = 0; f < t.SerializedFields.Count; f++)
                        {
                            SerializedFieldRecord sf = t.SerializedFields[f];
                            sb.AppendLine("        {");
                            sb.Append("          \"name\": \"").Append(DiagnoseReport.EscapeJson(sf.Name)).AppendLine("\",");
                            sb.Append("          \"type\": \"").Append(DiagnoseReport.EscapeJson(sf.FieldType)).AppendLine("\",");
                            sb.Append("          \"declaringType\": \"").Append(DiagnoseReport.EscapeJson(sf.DeclaringType)).AppendLine("\",");
                            sb.Append("          \"accessibility\": \"").Append(DiagnoseReport.EscapeJson(sf.Accessibility)).AppendLine("\",");
                            sb.Append("          \"hasSerializeField\": ").Append(sf.HasSerializeField ? "true" : "false").AppendLine(",");
                            sb.Append("          \"isSerializeReference\": ").Append(sf.IsSerializeReference ? "true" : "false").AppendLine(",");
                            AppendJsonStringArray(sb, "          ", "formerlySerializedAs", sf.FormerlySerializedAs, true);
                            AppendJsonStringArray(sb, "          ", "attributes", sf.Attributes, false);
                            sb.Append("        }");
                            if (f + 1 < t.SerializedFields.Count)
                            {
                                sb.Append(',');
                            }

                            sb.AppendLine();
                        }

                        sb.Append("      ");
                    }

                    sb.AppendLine("]");
                    sb.Append("    }");
                    if (i + 1 < Types.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append("  ");
            }

            sb.AppendLine("]");
            sb.Append('}');
            return sb.ToString();
        }

        private static void AppendJsonStringArray(
            StringBuilder sb,
            string indent,
            string propertyName,
            IReadOnlyList<string> items,
            bool trailingComma)
        {
            sb.Append(indent).Append('"').Append(propertyName).Append("\": [");
            for (int i = 0; i < items.Count; i++)
            {
                if (i > 0)
                {
                    sb.Append(", ");
                }

                sb.Append('"').Append(DiagnoseReport.EscapeJson(items[i])).Append('"');
            }

            sb.Append(']');
            if (trailingComma)
            {
                sb.Append(',');
            }

            sb.AppendLine();
        }
    }

    public static class SymbolQuery
    {
        private static readonly SymbolDisplayFormat TypeNameFormat = new SymbolDisplayFormat(
            globalNamespaceStyle: SymbolDisplayGlobalNamespaceStyle.Omitted,
            typeQualificationStyle: SymbolDisplayTypeQualificationStyle.NameAndContainingTypesAndNamespaces,
            genericsOptions: SymbolDisplayGenericsOptions.IncludeTypeParameters,
            miscellaneousOptions: SymbolDisplayMiscellaneousOptions.UseSpecialTypes);

        private static readonly SymbolDisplayFormat SignatureFormat = new SymbolDisplayFormat(
            globalNamespaceStyle: SymbolDisplayGlobalNamespaceStyle.Omitted,
            typeQualificationStyle: SymbolDisplayTypeQualificationStyle.NameAndContainingTypesAndNamespaces,
            genericsOptions: SymbolDisplayGenericsOptions.IncludeTypeParameters,
            memberOptions: SymbolDisplayMemberOptions.IncludeType
                | SymbolDisplayMemberOptions.IncludeParameters
                | SymbolDisplayMemberOptions.IncludeModifiers
                | SymbolDisplayMemberOptions.IncludeAccessibility,
            parameterOptions: SymbolDisplayParameterOptions.IncludeType
                | SymbolDisplayParameterOptions.IncludeName
                | SymbolDisplayParameterOptions.IncludeParamsRefOut
                | SymbolDisplayParameterOptions.IncludeDefaultValue,
            propertyStyle: SymbolDisplayPropertyStyle.ShowReadWriteDescriptor,
            miscellaneousOptions: SymbolDisplayMiscellaneousOptions.UseSpecialTypes);

        static SymbolQuery()
        {
            RoslynAssemblyResolver.EnsureInstalled();
        }

        public static SymbolQueryResult QueryProject(
            string projectRoot,
            string? typeFilter = null,
            ScriptCompilationOptions? options = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            ScriptCompilationOptions effectiveOptions = options ?? new ScriptCompilationOptions();
            List<ScriptDiagnostic> diagnostics = new List<ScriptDiagnostic>();

            IReadOnlyList<(string AssemblyName, CSharpCompilation Compilation)> compilations =
                ScriptCompilation.CreateProjectCompilations(fullProjectRoot, effectiveOptions, diagnostics);

            List<SymbolTypeRecord> types = new List<SymbolTypeRecord>();

            for (int cIdx = 0; cIdx < compilations.Count; cIdx++)
            {
                (string asmName, CSharpCompilation compilation) = compilations[cIdx];
                for (int tIdx = 0; tIdx < compilation.SyntaxTrees.Length; tIdx++)
                {
                    SyntaxTree tree = compilation.SyntaxTrees[tIdx];
                    SemanticModel semanticModel = compilation.GetSemanticModel(tree);
                    SyntaxNode root = tree.GetRoot();

                    List<BaseTypeDeclarationSyntax> typeNodes = new List<BaseTypeDeclarationSyntax>();
                    CollectTypeDeclarations(root, typeNodes);

                    for (int nIdx = 0; nIdx < typeNodes.Count; nIdx++)
                    {
                        BaseTypeDeclarationSyntax declNode = typeNodes[nIdx];
                        INamedTypeSymbol? typeSymbol = semanticModel.GetDeclaredSymbol(declNode);
                        if (typeSymbol == null)
                        {
                            continue;
                        }

                        if (!IsPrimaryDeclaration(typeSymbol, declNode))
                        {
                            continue;
                        }

                        string fullName = GetFullTypeName(typeSymbol);
                        if (!MatchesTypeFilter(typeSymbol.Name, fullName, typeFilter))
                        {
                            continue;
                        }

                        SymbolTypeRecord record = BuildTypeRecord(typeSymbol, declNode, asmName);
                        types.Add(record);
                    }
                }
            }

            types.Sort((a, b) =>
            {
                int cmp = string.CompareOrdinal(a.Assembly, b.Assembly);
                if (cmp != 0)
                {
                    return cmp;
                }

                return string.CompareOrdinal(a.FullName, b.FullName);
            });

            return new SymbolQueryResult(fullProjectRoot, typeFilter, types, diagnostics);
        }

        public static IReadOnlyList<ScriptDiagnostic> EvaluateCsprojDiagnostics(string csprojPath)
        {
            if (string.IsNullOrEmpty(csprojPath))
            {
                throw new ArgumentException("Csproj path must be non-empty.", nameof(csprojPath));
            }

            string fullCsprojPath = Path.GetFullPath(csprojPath);
            if (!File.Exists(fullCsprojPath))
            {
                throw new FileNotFoundException("Generated .csproj file was not found.", fullCsprojPath);
            }

            Dictionary<string, CSharpCompilation> cache =
                new Dictionary<string, CSharpCompilation>(StringComparer.Ordinal);
            CSharpCompilation compilation = BuildCompilationFromCsproj(fullCsprojPath, cache);

            string assemblyName = compilation.AssemblyName ?? Path.GetFileNameWithoutExtension(fullCsprojPath);
            List<ScriptDiagnostic> diagnostics = new List<ScriptDiagnostic>();

            foreach (Diagnostic diag in compilation.GetDiagnostics())
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

            return diagnostics;
        }

        private static CSharpCompilation BuildCompilationFromCsproj(
            string fullCsprojPath,
            Dictionary<string, CSharpCompilation> cache)
        {
            if (cache.TryGetValue(fullCsprojPath, out CSharpCompilation? existing))
            {
                return existing;
            }

            string projectDir = Path.GetDirectoryName(fullCsprojPath) ?? string.Empty;
            XDocument doc = XDocument.Load(fullCsprojPath);

            string assemblyName = Path.GetFileNameWithoutExtension(fullCsprojPath);
            bool allowUnsafe = false;
            bool treatWarningsAsErrors = false;
            List<string> defines = new List<string>();
            List<string> noWarnIds = new List<string>();
            List<string> warnAsErrorIds = new List<string>();
            List<string> sourceFiles = new List<string>();
            List<string> binaryReferencePaths = new List<string>();
            List<string> projectReferencePaths = new List<string>();

            foreach (XElement elem in doc.Descendants())
            {
                string local = elem.Name.LocalName;
                if (string.Equals(local, "AssemblyName", StringComparison.Ordinal))
                {
                    string val = elem.Value.Trim();
                    if (!string.IsNullOrEmpty(val))
                    {
                        assemblyName = val;
                    }
                }
                else if (string.Equals(local, string.Concat("Allow", "Un", "safeBlocks"), StringComparison.Ordinal))
                {
                    allowUnsafe = string.Equals(elem.Value.Trim(), "true", StringComparison.OrdinalIgnoreCase);
                }
                else if (string.Equals(local, "TreatWarningsAsErrors", StringComparison.Ordinal))
                {
                    treatWarningsAsErrors = string.Equals(elem.Value.Trim(), "true", StringComparison.OrdinalIgnoreCase);
                }
                else if (string.Equals(local, "DefineConstants", StringComparison.Ordinal))
                {
                    string[] parts = elem.Value.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
                    for (int i = 0; i < parts.Length; i++)
                    {
                        string sym = parts[i].Trim();
                        if (!string.IsNullOrEmpty(sym) && !defines.Contains(sym))
                        {
                            defines.Add(sym);
                        }
                    }
                }
                else if (string.Equals(local, "NoWarn", StringComparison.Ordinal))
                {
                    string[] parts = elem.Value.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
                    for (int i = 0; i < parts.Length; i++)
                    {
                        string id = parts[i].Trim();
                        if (!string.IsNullOrEmpty(id))
                        {
                            noWarnIds.Add(id);
                        }
                    }
                }
                else if (string.Equals(local, "WarningsAsErrors", StringComparison.Ordinal))
                {
                    string[] parts = elem.Value.Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries);
                    for (int i = 0; i < parts.Length; i++)
                    {
                        string id = parts[i].Trim();
                        if (!string.IsNullOrEmpty(id))
                        {
                            warnAsErrorIds.Add(id);
                        }
                    }
                }
                else if (string.Equals(local, "Compile", StringComparison.Ordinal))
                {
                    XAttribute? includeAttr = elem.Attribute("Include");
                    if (includeAttr != null && !string.IsNullOrWhiteSpace(includeAttr.Value))
                    {
                        string rel = includeAttr.Value.Trim().Replace('/', Path.DirectorySeparatorChar);
                        string fullSrc = Path.IsPathRooted(rel) ? Path.GetFullPath(rel) : Path.GetFullPath(Path.Combine(projectDir, rel));
                        if (File.Exists(fullSrc))
                        {
                            sourceFiles.Add(fullSrc);
                        }
                    }
                }
                else if (string.Equals(local, "Reference", StringComparison.Ordinal))
                {
                    foreach (XElement child in elem.Elements())
                    {
                        if (string.Equals(child.Name.LocalName, "HintPath", StringComparison.Ordinal))
                        {
                            string hint = child.Value.Trim();
                            if (!string.IsNullOrEmpty(hint))
                            {
                                string fullRef = Path.IsPathRooted(hint) ? Path.GetFullPath(hint) : Path.GetFullPath(Path.Combine(projectDir, hint));
                                if (File.Exists(fullRef))
                                {
                                    binaryReferencePaths.Add(fullRef);
                                }
                            }
                        }
                    }
                }
                else if (string.Equals(local, "ProjectReference", StringComparison.Ordinal))
                {
                    XAttribute? includeAttr = elem.Attribute("Include");
                    if (includeAttr != null && !string.IsNullOrWhiteSpace(includeAttr.Value))
                    {
                        string rel = includeAttr.Value.Trim().Replace('/', Path.DirectorySeparatorChar);
                        string fullDep = Path.IsPathRooted(rel) ? Path.GetFullPath(rel) : Path.GetFullPath(Path.Combine(projectDir, rel));
                        if (File.Exists(fullDep))
                        {
                            projectReferencePaths.Add(fullDep);
                        }
                    }
                }
            }

            List<MetadataReference> metadataRefs = new List<MetadataReference>();
            IReadOnlyList<string> baseNetDlls = ScriptCompilation.GetNetReferenceDlls();
            HashSet<string> addedPaths = new HashSet<string>(StringComparer.Ordinal);
            for (int i = 0; i < baseNetDlls.Count; i++)
            {
                string p = Path.GetFullPath(baseNetDlls[i]);
                if (File.Exists(p) && addedPaths.Add(p))
                {
                    metadataRefs.Add(ScriptCompilation.GetMetadataReference(p));
                }
            }

            for (int i = 0; i < binaryReferencePaths.Count; i++)
            {
                string p = Path.GetFullPath(binaryReferencePaths[i]);
                if (File.Exists(p) && addedPaths.Add(p))
                {
                    metadataRefs.Add(ScriptCompilation.GetMetadataReference(p));
                }
            }

            for (int i = 0; i < projectReferencePaths.Count; i++)
            {
                CSharpCompilation depComp = BuildCompilationFromCsproj(projectReferencePaths[i], cache);
                metadataRefs.Add(depComp.ToMetadataReference());
            }

            ResponseFileOptions rsp = new ResponseFileOptions
            {
                AllowUnsafe = allowUnsafe,
                WarningsAsErrors = treatWarningsAsErrors
            };
            rsp.DisabledWarnings.AddRange(noWarnIds);
            rsp.WarningsAsErrorIds.AddRange(warnAsErrorIds);

            CSharpCompilation compilation = ScriptCompilation.CreateRoslynCompilation(
                assemblyName,
                sourceFiles,
                defines,
                metadataRefs,
                allowUnsafe,
                rsp);

            cache[fullCsprojPath] = compilation;
            return compilation;
        }

        private static void CollectTypeDeclarations(SyntaxNode node, List<BaseTypeDeclarationSyntax> results)
        {
            if (node is BaseTypeDeclarationSyntax typeDecl)
            {
                results.Add(typeDecl);
            }

            foreach (SyntaxNode child in node.ChildNodes())
            {
                CollectTypeDeclarations(child, results);
            }
        }

        private static bool IsPrimaryDeclaration(INamedTypeSymbol symbol, BaseTypeDeclarationSyntax node)
        {
            if (symbol.DeclaringSyntaxReferences.Length <= 1)
            {
                return true;
            }

            SyntaxReference first = symbol.DeclaringSyntaxReferences[0];
            return first.SyntaxTree == node.SyntaxTree && first.Span == node.Span;
        }

        private static bool MatchesTypeFilter(string simpleName, string fullName, string? typeFilter)
        {
            if (string.IsNullOrWhiteSpace(typeFilter))
            {
                return true;
            }

            string filter = typeFilter.Trim();
            return string.Equals(simpleName, filter, StringComparison.OrdinalIgnoreCase)
                || string.Equals(fullName, filter, StringComparison.OrdinalIgnoreCase)
                || fullName.EndsWith("." + filter, StringComparison.OrdinalIgnoreCase);
        }

        private static SymbolTypeRecord BuildTypeRecord(
            INamedTypeSymbol typeSymbol,
            BaseTypeDeclarationSyntax declNode,
            string assemblyName)
        {
            string simpleName = typeSymbol.Name;
            string ns = typeSymbol.ContainingNamespace != null && !typeSymbol.ContainingNamespace.IsGlobalNamespace
                ? typeSymbol.ContainingNamespace.ToDisplayString()
                : string.Empty;
            string fullName = GetFullTypeName(typeSymbol);
            string kind = typeSymbol.TypeKind switch
            {
                TypeKind.Class => "class",
                TypeKind.Struct => "struct",
                TypeKind.Interface => "interface",
                TypeKind.Enum => "enum",
                _ => typeSymbol.TypeKind.ToString().ToLowerInvariant()
            };

            string accessibility = FormatAccessibility(typeSymbol.DeclaredAccessibility);
            FileLinePositionSpan span = declNode.GetLocation().GetMappedLineSpan();
            string filePath = span.Path ?? string.Empty;
            int line = span.StartLinePosition.Line + 1;

            string baseType = typeSymbol.BaseType != null && typeSymbol.BaseType.SpecialType != SpecialType.System_Object
                ? typeSymbol.BaseType.ToDisplayString(TypeNameFormat)
                : (typeSymbol.BaseType != null ? "System.Object" : string.Empty);

            List<string> baseChain = new List<string>();
            INamedTypeSymbol? cursor = typeSymbol.BaseType;
            while (cursor != null)
            {
                baseChain.Add(cursor.ToDisplayString(TypeNameFormat));
                cursor = cursor.BaseType;
            }

            List<string> interfaces = new List<string>();
            for (int i = 0; i < typeSymbol.AllInterfaces.Length; i++)
            {
                interfaces.Add(typeSymbol.AllInterfaces[i].ToDisplayString(TypeNameFormat));
            }

            List<string> attributes = ExtractAttributeStrings(typeSymbol.GetAttributes());

            bool isMonoBehaviour = InheritsFromTypeNamed(typeSymbol, "UnityEngine.MonoBehaviour", "MonoBehaviour");
            bool isScriptableObject = InheritsFromTypeNamed(typeSymbol, "UnityEngine.ScriptableObject", "ScriptableObject");
            bool isSerializableType = isMonoBehaviour
                || isScriptableObject
                || HasSerializableAttribute(typeSymbol);

            List<SymbolMemberRecord> members = BuildMembers(typeSymbol);
            List<SerializedFieldRecord> serializedFields = isSerializableType
                ? BuildSerializedFields(typeSymbol)
                : new List<SerializedFieldRecord>();

            return new SymbolTypeRecord(
                simpleName,
                ns,
                fullName,
                kind,
                accessibility,
                assemblyName,
                filePath,
                line,
                baseType,
                baseChain,
                interfaces,
                attributes,
                isMonoBehaviour,
                isScriptableObject,
                isSerializableType,
                members,
                serializedFields);
        }

        private static List<SymbolMemberRecord> BuildMembers(INamedTypeSymbol typeSymbol)
        {
            List<SymbolMemberRecord> list = new List<SymbolMemberRecord>();
            foreach (ISymbol member in typeSymbol.GetMembers())
            {
                if (member.IsImplicitlyDeclared)
                {
                    continue;
                }

                string accessibility = FormatAccessibility(member.DeclaredAccessibility);
                List<string> attrs = ExtractAttributeStrings(member.GetAttributes());

                if (member is IFieldSymbol field)
                {
                    string fieldType = field.Type.ToDisplayString(TypeNameFormat);
                    string sig = BuildFieldSignature(field, accessibility, fieldType);
                    list.Add(new SymbolMemberRecord(
                        field.Name,
                        "field",
                        accessibility,
                        field.IsStatic,
                        fieldType,
                        sig,
                        attrs));
                }
                else if (member is IPropertySymbol prop)
                {
                    string propType = prop.Type.ToDisplayString(TypeNameFormat);
                    string sig = prop.ToDisplayString(SignatureFormat);
                    list.Add(new SymbolMemberRecord(
                        prop.Name,
                        "property",
                        accessibility,
                        prop.IsStatic,
                        propType,
                        sig,
                        attrs));
                }
                else if (member is IMethodSymbol method)
                {
                    if (method.MethodKind == MethodKind.PropertyGet
                        || method.MethodKind == MethodKind.PropertySet
                        || method.MethodKind == MethodKind.EventAdd
                        || method.MethodKind == MethodKind.EventRemove)
                    {
                        continue;
                    }

                    string kind = method.MethodKind == MethodKind.Constructor ? "constructor" : "method";
                    string retType = method.ReturnType.ToDisplayString(TypeNameFormat);
                    string sig = BuildMethodSignature(method, accessibility, retType);
                    list.Add(new SymbolMemberRecord(
                        method.Name,
                        kind,
                        accessibility,
                        method.IsStatic,
                        retType,
                        sig,
                        attrs));
                }
                else if (member is IEventSymbol evt)
                {
                    string evtType = evt.Type.ToDisplayString(TypeNameFormat);
                    string sig = evt.ToDisplayString(SignatureFormat);
                    list.Add(new SymbolMemberRecord(
                        evt.Name,
                        "event",
                        accessibility,
                        evt.IsStatic,
                        evtType,
                        sig,
                        attrs));
                }
            }

            return list;
        }

        private static string BuildFieldSignature(IFieldSymbol field, string accessibility, string fieldType)
        {
            StringBuilder sb = new StringBuilder();
            sb.Append(accessibility);
            if (field.IsConst)
            {
                sb.Append(" const");
            }
            else
            {
                if (field.IsStatic)
                {
                    sb.Append(" static");
                }

                if (field.IsReadOnly)
                {
                    sb.Append(" readonly");
                }
            }

            sb.Append(' ').Append(fieldType).Append(' ').Append(field.Name);
            return sb.ToString();
        }

        private static string BuildMethodSignature(IMethodSymbol method, string accessibility, string returnType)
        {
            StringBuilder sb = new StringBuilder();
            sb.Append(accessibility);
            if (method.IsStatic)
            {
                sb.Append(" static");
            }

            if (method.IsVirtual)
            {
                sb.Append(" virtual");
            }
            else if (method.IsOverride)
            {
                sb.Append(" override");
            }
            else if (method.IsAbstract)
            {
                sb.Append(" abstract");
            }

            if (method.MethodKind == MethodKind.Constructor)
            {
                sb.Append(' ').Append(method.ContainingType.Name);
            }
            else
            {
                sb.Append(' ').Append(returnType).Append(' ').Append(method.Name);
            }

            if (method.TypeParameters.Length > 0)
            {
                sb.Append('<');
                for (int i = 0; i < method.TypeParameters.Length; i++)
                {
                    if (i > 0)
                    {
                        sb.Append(", ");
                    }

                    sb.Append(method.TypeParameters[i].Name);
                }

                sb.Append('>');
            }

            sb.Append('(');
            for (int i = 0; i < method.Parameters.Length; i++)
            {
                if (i > 0)
                {
                    sb.Append(", ");
                }

                IParameterSymbol p = method.Parameters[i];
                if (p.RefKind == RefKind.Ref)
                {
                    sb.Append("ref ");
                }
                else if (p.RefKind == RefKind.Out)
                {
                    sb.Append("out ");
                }
                else if (p.RefKind == RefKind.In)
                {
                    sb.Append("in ");
                }
                else if (p.IsParams)
                {
                    sb.Append("params ");
                }

                sb.Append(p.Type.ToDisplayString(TypeNameFormat)).Append(' ').Append(p.Name);
            }

            sb.Append(')');
            return sb.ToString();
        }

        private static List<SerializedFieldRecord> BuildSerializedFields(INamedTypeSymbol typeSymbol)
        {
            List<INamedTypeSymbol> hierarchy = new List<INamedTypeSymbol>();
            INamedTypeSymbol? cursor = typeSymbol;
            while (cursor != null)
            {
                string full = GetFullTypeName(cursor);
                if (string.Equals(full, "UnityEngine.MonoBehaviour", StringComparison.Ordinal)
                    || string.Equals(full, "UnityEngine.Behaviour", StringComparison.Ordinal)
                    || string.Equals(full, "UnityEngine.Component", StringComparison.Ordinal)
                    || string.Equals(full, "UnityEngine.ScriptableObject", StringComparison.Ordinal)
                    || string.Equals(full, "UnityEngine.Object", StringComparison.Ordinal)
                    || cursor.SpecialType == SpecialType.System_Object)
                {
                    break;
                }

                hierarchy.Add(cursor);
                cursor = cursor.BaseType;
            }

            hierarchy.Reverse();

            List<SerializedFieldRecord> serialized = new List<SerializedFieldRecord>();
            for (int h = 0; h < hierarchy.Count; h++)
            {
                INamedTypeSymbol currentType = hierarchy[h];
                string declaringName = GetFullTypeName(currentType);

                foreach (ISymbol member in currentType.GetMembers())
                {
                    if (!(member is IFieldSymbol field) || field.IsImplicitlyDeclared)
                    {
                        continue;
                    }

                    if (field.IsStatic || field.IsConst || field.IsReadOnly)
                    {
                        continue;
                    }

                    InspectFieldSerializationAttributes(
                        field,
                        out bool hasSerializeField,
                        out bool hasNonSerialized,
                        out bool hasSerializeReference,
                        out List<string> formerNames,
                        out List<string> allAttrStrings);

                    if (hasNonSerialized)
                    {
                        continue;
                    }

                    bool isPublic = field.DeclaredAccessibility == Accessibility.Public;
                    if (!isPublic && !hasSerializeField && !hasSerializeReference)
                    {
                        continue;
                    }

                    if (!IsUnitySerializableFieldType(field.Type, hasSerializeReference))
                    {
                        continue;
                    }

                    string fieldType = field.Type.ToDisplayString(TypeNameFormat);
                    string accessibility = FormatAccessibility(field.DeclaredAccessibility);

                    serialized.Add(new SerializedFieldRecord(
                        field.Name,
                        fieldType,
                        declaringName,
                        accessibility,
                        hasSerializeField,
                        hasSerializeReference,
                        formerNames,
                        allAttrStrings));
                }
            }

            return serialized;
        }

        private static void InspectFieldSerializationAttributes(
            IFieldSymbol field,
            out bool hasSerializeField,
            out bool hasNonSerialized,
            out bool hasSerializeReference,
            out List<string> formerNames,
            out List<string> allAttrStrings)
        {
            hasSerializeField = false;
            hasNonSerialized = false;
            hasSerializeReference = false;
            formerNames = new List<string>();
            allAttrStrings = new List<string>();

            foreach (AttributeData attr in field.GetAttributes())
            {
                string shortName = GetShortAttributeName(attr);
                string formatted = FormatAttributeData(attr, shortName);
                allAttrStrings.Add(formatted);

                if (AttributeMatches(shortName, "SerializeField"))
                {
                    hasSerializeField = true;
                }
                else if (AttributeMatches(shortName, "NonSerialized"))
                {
                    hasNonSerialized = true;
                }
                else if (AttributeMatches(shortName, "SerializeReference"))
                {
                    hasSerializeReference = true;
                }
                else if (AttributeMatches(shortName, "FormerlySerializedAs"))
                {
                    string? oldName = ExtractFirstStringArgument(attr);
                    if (!string.IsNullOrEmpty(oldName) && !formerNames.Contains(oldName))
                    {
                        formerNames.Add(oldName);
                    }
                }
            }
        }


        private static bool IsUnitySerializableFieldType(ITypeSymbol typeSymbol, bool hasSerializeReference)
        {
            if (hasSerializeReference)
            {
                return !typeSymbol.IsValueType;
            }

            if (typeSymbol is IArrayTypeSymbol arrayType)
            {
                if (arrayType.Rank != 1)
                {
                    return false;
                }

                return IsUnitySerializableElementType(arrayType.ElementType);
            }

            if (typeSymbol is INamedTypeSymbol namedType && namedType.IsGenericType)
            {
                string genericDef = GetFullTypeName(namedType.OriginalDefinition);
                string expectedListDef = string.Concat("System.Collections.Generic.Li", "st");
                if (string.Equals(genericDef, expectedListDef, StringComparison.Ordinal)
                    && namedType.TypeArguments.Length == 1)
                {
                    return IsUnitySerializableElementType(namedType.TypeArguments[0]);
                }

                return false;
            }

            return IsUnitySerializableElementType(typeSymbol);
        }

        private static bool IsUnitySerializableElementType(ITypeSymbol type)
        {
            switch (type.SpecialType)
            {
                case SpecialType.System_Boolean:
                case SpecialType.System_Byte:
                case SpecialType.System_SByte:
                case SpecialType.System_Char:
                case SpecialType.System_Int16:
                case SpecialType.System_UInt16:
                case SpecialType.System_Int32:
                case SpecialType.System_UInt32:
                case SpecialType.System_Int64:
                case SpecialType.System_UInt64:
                case SpecialType.System_Single:
                case SpecialType.System_Double:
                case SpecialType.System_String:
                    return true;
            }

            if (type.TypeKind == TypeKind.Enum)
            {
                return true;
            }

            if (type.TypeKind == TypeKind.Interface || type.TypeKind == TypeKind.Delegate)
            {
                return false;
            }

            if (type is INamedTypeSymbol named)
            {
                if (named.IsGenericType)
                {
                    return false;
                }

                if (InheritsFromTypeNamed(named, "UnityEngine.Object", "Object"))
                {
                    return true;
                }

                string full = GetFullTypeName(named);
                if (IsBuiltInUnityValueType(full))
                {
                    return true;
                }

                if (!named.IsAbstract && HasSerializableAttribute(named))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool IsBuiltInUnityValueType(string fullName)
        {
            return string.Equals(fullName, "UnityEngine.Vector2", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Vector2Int", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Vector3", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Vector3Int", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Vector4", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Quaternion", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Matrix4x4", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Color", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Color32", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Rect", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.RectInt", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Bounds", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.BoundsInt", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.LayerMask", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.AnimationCurve", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Gradient", StringComparison.Ordinal)
                || string.Equals(fullName, "UnityEngine.Events.UnityEvent", StringComparison.Ordinal);
        }

        private static bool HasSerializableAttribute(INamedTypeSymbol symbol)
        {
            if ((symbol.TypeKind == TypeKind.Class || symbol.TypeKind == TypeKind.Struct) && symbol.IsSerializable)
            {
                return true;
            }

            foreach (AttributeData attr in symbol.GetAttributes())
            {
                string shortName = GetShortAttributeName(attr);
                if (AttributeMatches(shortName, "Serializable"))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool InheritsFromTypeNamed(INamedTypeSymbol symbol, string expectedFullName, string expectedShortName)
        {
            INamedTypeSymbol? cursor = symbol;
            while (cursor != null)
            {
                string full = GetFullTypeName(cursor);
                if (string.Equals(full, expectedFullName, StringComparison.Ordinal)
                    || string.Equals(cursor.Name, expectedShortName, StringComparison.Ordinal))
                {
                    return true;
                }

                cursor = cursor.BaseType;
            }

            return false;
        }

        private static List<string> ExtractAttributeStrings(System.Collections.Immutable.ImmutableArray<AttributeData> attributes)
        {
            List<string> list = new List<string>(attributes.Length);
            for (int i = 0; i < attributes.Length; i++)
            {
                AttributeData attr = attributes[i];
                string shortName = GetShortAttributeName(attr);
                list.Add(FormatAttributeData(attr, shortName));
            }

            return list;
        }

        private static string GetShortAttributeName(AttributeData attr)
        {
            string rawName = attr.AttributeClass?.Name ?? string.Empty;
            if (string.IsNullOrEmpty(rawName) && attr.ApplicationSyntaxReference != null)
            {
                SyntaxNode node = attr.ApplicationSyntaxReference.GetSyntax();
                if (node is AttributeSyntax attrSyntax)
                {
                    rawName = attrSyntax.Name.ToString();
                    int dot = rawName.LastIndexOf('.');
                    if (dot >= 0)
                    {
                        rawName = rawName.Substring(dot + 1);
                    }
                }
            }

            if (rawName.EndsWith("Attribute", StringComparison.Ordinal) && rawName.Length > "Attribute".Length)
            {
                return rawName.Substring(0, rawName.Length - "Attribute".Length);
            }

            return rawName;
        }

        private static string FormatAttributeData(AttributeData attr, string shortName)
        {
            List<string> args = new List<string>();
            for (int i = 0; i < attr.ConstructorArguments.Length; i++)
            {
                TypedConstant tc = attr.ConstructorArguments[i];
                args.Add(FormatTypedConstant(tc));
            }

            for (int i = 0; i < attr.NamedArguments.Length; i++)
            {
                KeyValuePair<string, TypedConstant> kv = attr.NamedArguments[i];
                args.Add(kv.Key + " = " + FormatTypedConstant(kv.Value));
            }

            if (args.Count == 0 && attr.ApplicationSyntaxReference != null)
            {
                SyntaxNode node = attr.ApplicationSyntaxReference.GetSyntax();
                if (node is AttributeSyntax attrSyntax && attrSyntax.ArgumentList != null && attrSyntax.ArgumentList.Arguments.Count > 0)
                {
                    for (int i = 0; i < attrSyntax.ArgumentList.Arguments.Count; i++)
                    {
                        args.Add(attrSyntax.ArgumentList.Arguments[i].ToString());
                    }
                }
            }

            return args.Count == 0
                ? shortName
                : shortName + "(" + string.Join(", ", args) + ")";
        }

        private static string FormatTypedConstant(TypedConstant tc)
        {
            if (tc.IsNull)
            {
                return "null";
            }

            if (tc.Kind == TypedConstantKind.Array)
            {
                List<string> elements = new List<string>();
                for (int i = 0; i < tc.Values.Length; i++)
                {
                    elements.Add(FormatTypedConstant(tc.Values[i]));
                }

                return "[" + string.Join(", ", elements) + "]";
            }

            if (tc.Kind == TypedConstantKind.Type && tc.Value is ITypeSymbol ts)
            {
                return "typeof(" + ts.ToDisplayString(TypeNameFormat) + ")";
            }

            if (tc.Value is string s)
            {
                return "\"" + s + "\"";
            }

            if (tc.Value is bool b)
            {
                return b ? "true" : "false";
            }

            if (tc.Value is IFormattable formattable)
            {
                return formattable.ToString(null, CultureInfo.InvariantCulture);
            }

            return tc.Value?.ToString() ?? string.Empty;
        }

        private static string? ExtractFirstStringArgument(AttributeData attr)
        {
            if (attr.ConstructorArguments.Length > 0 && attr.ConstructorArguments[0].Value is string s)
            {
                return s;
            }

            for (int i = 0; i < attr.NamedArguments.Length; i++)
            {
                if (attr.NamedArguments[i].Value.Value is string ns)
                {
                    return ns;
                }
            }

            if (attr.ApplicationSyntaxReference != null)
            {
                SyntaxNode node = attr.ApplicationSyntaxReference.GetSyntax();
                if (node is AttributeSyntax attrSyntax
                    && attrSyntax.ArgumentList != null
                    && attrSyntax.ArgumentList.Arguments.Count > 0)
                {
                    ExpressionSyntax expr = attrSyntax.ArgumentList.Arguments[0].Expression;
                    if (expr is LiteralExpressionSyntax lit && lit.IsKind(SyntaxKind.StringLiteralExpression))
                    {
                        return lit.Token.ValueText;
                    }
                }
            }

            return null;
        }

        private static bool AttributeMatches(string actualShortName, string expectedShortName)
        {
            return string.Equals(actualShortName, expectedShortName, StringComparison.Ordinal)
                || string.Equals(actualShortName, expectedShortName + "Attribute", StringComparison.Ordinal);
        }

        private static string GetFullTypeName(INamedTypeSymbol symbol)
        {
            if (symbol.ContainingType != null)
            {
                return GetFullTypeName(symbol.ContainingType) + "." + symbol.Name;
            }

            if (symbol.ContainingNamespace != null && !symbol.ContainingNamespace.IsGlobalNamespace)
            {
                return symbol.ContainingNamespace.ToDisplayString() + "." + symbol.Name;
            }

            return symbol.Name;
        }

        private static string FormatAccessibility(Accessibility accessibility)
        {
            return accessibility switch
            {
                Accessibility.Public => "public",
                Accessibility.Private => "private",
                Accessibility.Protected => "protected",
                Accessibility.Internal => "internal",
                Accessibility.ProtectedOrInternal => "protected internal",
                Accessibility.ProtectedAndInternal => "private protected",
                _ => "private"
            };
        }
    }
}
