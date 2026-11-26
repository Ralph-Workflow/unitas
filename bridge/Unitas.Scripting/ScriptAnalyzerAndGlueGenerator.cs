using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;

namespace Unitas.Scripting
{
    public sealed class ParsedDllImportCall
    {
        public ParsedDllImportCall(string libraryName, string methodName, string filePath, int line)
        {
            LibraryName = libraryName;
            MethodName = methodName;
            FilePath = filePath;
            Line = line;
        }

        public string LibraryName { get; }

        public string MethodName { get; }

        public string FilePath { get; }

        public int Line { get; }
    }

    public sealed class EditorCallableMethodBinding
    {
        public EditorCallableMethodBinding(string fullTypeName, string methodName, bool returnsVoid)
        {
            FullTypeName = fullTypeName ?? string.Empty;
            MethodName = methodName ?? string.Empty;
            ReturnsVoid = returnsVoid;
        }

        public string FullTypeName { get; }

        public string MethodName { get; }

        public bool ReturnsVoid { get; }
    }

    public static class ScriptAnalyzerAndGlueGenerator
    {
        private static readonly Regex NamespaceRegex = new Regex(
            @"\bnamespace\s+([A-Za-z_][A-Za-z0-9_.]*)",
            RegexOptions.Compiled);

        private static readonly Regex ClassRegex = new Regex(
            @"(?:(public|internal|private|protected)\s+)?(?:(abstract|sealed|static|partial)\s+)*class\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*:\s*([^{]+))?",
            RegexOptions.Compiled);

        private static readonly Regex TypeDeclRegex = new Regex(
            @"((?:\[[^\]]*\]\s*)*)(?:(public|internal|private|protected)\s+)?(?:(abstract|sealed|static|partial|readonly)\s+)*(class|struct)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*:\s*([^{]+))?",
            RegexOptions.Compiled);

        private static readonly Regex MethodRegex = new Regex(
            @"(?:^|[;{}])\s*(?:(\[[^\]]*\]\s*)*)(?:(public|private|protected|internal)\s+)?(?:(virtual|override|new|sealed|async)\s+)*(void|IEnumerator|System\.Collections\.IEnumerator)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?:\{|=>)",
            RegexOptions.Compiled | RegexOptions.Multiline);

        private static readonly Regex RuntimeInitializeMethodRegex = new Regex(
            @"\[\s*(?:UnityEngine\.)?RuntimeInitializeOnLoadMethod(?:Attribute)?\s*(?:\(\s*(?:(?:UnityEngine\.)?RuntimeInitializeLoadType\.)?(?<loadType>[A-Za-z_][A-Za-z0-9_]*)\s*\))?\s*\]\s*(?:(public|private|protected|internal)\s+)?static\s+void\s+(?<method>[A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)",
            RegexOptions.Compiled | RegexOptions.Singleline);

        private static readonly Regex EditorCallableMethodRegex = new Regex(
            @"\[\s*(?:UnityEngine\.)?EditorCallableMethod(?:Attribute)?\s*\]\s*(?:(public|private|protected|internal)\s+)?static\s+(?<returnType>void|[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^<>]*>)?(?:\s*\[\s*\])?)\s+(?<method>[A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)",
            RegexOptions.Compiled | RegexOptions.Singleline);

        private static readonly Regex FieldDeclRegex = new Regex(
            @"^\s*((?:\[[^\]]*\]\s*)*)(?:(public|private|protected|internal)\s+)?((?:(?:static|const|readonly|volatile|new)\s+)*)([A-Za-z_][A-Za-z0-9_<>,.\[\]\s]*?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:=[^;]*)?;\s*$",
            RegexOptions.Compiled);

        private static readonly Regex DllImportRegex = new Regex(
            @"\[\s*(?:System\.Runtime\.InteropServices\.)?DllImport(?:Attribute)?\s*\(\s*""([^""]+)""[^\]]*\]\s*(?:[A-Za-z_][A-Za-z0-9_<>,.\[\]\s]*?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
            RegexOptions.Compiled | RegexOptions.Singleline);

        private static readonly HashSet<string> ReservedMethodNames = new HashSet<string>(StringComparer.Ordinal)
        {
            "Main", "ToString", "Equals", "GetHashCode", "GetType", "MemberwiseClone", "Finalize"
        };

        public static List<ParsedDllImportCall> ScanDllImports(string sourceFilePath, HashSet<string> activeDefines)
        {
            List<ParsedDllImportCall> results = new List<ParsedDllImportCall>();
            if (!File.Exists(sourceFilePath))
            {
                return results;
            }

            string rawText;
            try
            {
                rawText = File.ReadAllText(sourceFilePath);
            }
            catch (IOException)
            {
                return results;
            }

            string activeSource = FilterConditionalCompilation(rawText, activeDefines);
            MatchCollection matches = DllImportRegex.Matches(activeSource);
            for (int i = 0; i < matches.Count; i++)
            {
                Match m = matches[i];
                string lib = m.Groups[1].Value.Trim();
                string method = m.Groups[2].Value.Trim();
                int line = CountLinesBefore(activeSource, m.Index);
                results.Add(new ParsedDllImportCall(lib, method, sourceFilePath, line));
            }

            return results;
        }

        public static List<ScriptGuidBinding> DiscoverScriptBindingsForAssembly(
            string assemblyName,
            IReadOnlyList<string> sourceFiles,
            HashSet<string> activeDefines,
            Dictionary<string, string> pathToGuid)
        {
            List<ScriptGuidBinding> bindings = new List<ScriptGuidBinding>();
            Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)> classMap =
                new Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)>(StringComparer.Ordinal);

            for (int i = 0; i < sourceFiles.Count; i++)
            {
                string filePath = Path.GetFullPath(sourceFiles[i]);
                if (!File.Exists(filePath))
                {
                    continue;
                }

                string rawText;
                try
                {
                    rawText = File.ReadAllText(filePath);
                }
                catch (IOException)
                {
                    continue;
                }

                string activeText = FilterConditionalCompilation(rawText, activeDefines);
                string ns = ExtractNamespace(activeText);
                MatchCollection classMatches = ClassRegex.Matches(activeText);
                for (int cIdx = 0; cIdx < classMatches.Count; cIdx++)
                {
                    Match cm = classMatches[cIdx];
                    string modifier = cm.Groups[2].Value;
                    if (string.Equals(modifier, "abstract", StringComparison.Ordinal)
                        || string.Equals(modifier, "static", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string className = cm.Groups[3].Value;
                    string baseList = cm.Groups[4].Success ? cm.Groups[4].Value : string.Empty;
                    string body = ExtractClassBody(activeText, cm.Index + cm.Length);
                    classMap[className] = (ns, baseList, body, filePath);
                }
            }

            foreach (KeyValuePair<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)> kv in classMap)
            {
                string className = kv.Key;
                (string ns, string _, string bodyText, string filePath) = kv.Value;
                if (!InheritsFromMonoBehaviour(className, classMap, new HashSet<string>(StringComparer.Ordinal)))
                {
                    continue;
                }

                if (!pathToGuid.TryGetValue(filePath, out string? guid) || string.IsNullOrEmpty(guid))
                {
                    guid = AssemblyDiscovery.TryReadGuidFromMetaFile(filePath + ".meta");
                }

                if (string.IsNullOrEmpty(guid) || !ProjectMetadataIndex.TrySplitGuid(guid, out ulong hi, out ulong lo))
                {
                    continue;
                }

                string expectedScriptName = Path.GetFileNameWithoutExtension(filePath);
                if (!string.Equals(expectedScriptName, className, StringComparison.Ordinal)
                    && classMap.ContainsKey(expectedScriptName))
                {
                    continue;
                }

                List<ScriptMethodSignature> methods = ParseMethodsFromClassHierarchy(className, classMap);
                List<ScriptSerializedField> fields = ParseFieldsFromClassHierarchy(className, classMap);
                bindings.Add(new ScriptGuidBinding(
                    guid,
                    hi,
                    lo,
                    filePath,
                    className,
                    ns,
                    assemblyName,
                    methods,
                    fields));
            }

            return bindings;
        }

        public static List<SerializableTypeBinding> DiscoverSerializableTypesForAssembly(
            string assemblyName,
            IReadOnlyList<string> sourceFiles,
            HashSet<string> activeDefines)
        {
            List<SerializableTypeBinding> results = new List<SerializableTypeBinding>();
            Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)> classMap =
                new Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)>(StringComparer.Ordinal);
            List<(string TypeName, string NamespaceName, bool IsValueType, string BodyText)> candidates =
                new List<(string TypeName, string NamespaceName, bool IsValueType, string BodyText)>();

            for (int i = 0; i < sourceFiles.Count; i++)
            {
                string filePath = Path.GetFullPath(sourceFiles[i]);
                if (!File.Exists(filePath))
                {
                    continue;
                }

                string rawText;
                try
                {
                    rawText = File.ReadAllText(filePath);
                }
                catch (IOException)
                {
                    continue;
                }

                string activeText = FilterConditionalCompilation(rawText, activeDefines);
                string ns = ExtractNamespace(activeText);
                MatchCollection typeMatches = TypeDeclRegex.Matches(activeText);
                for (int tIdx = 0; tIdx < typeMatches.Count; tIdx++)
                {
                    Match tm = typeMatches[tIdx];
                    string attrs = tm.Groups[1].Value;
                    string visibility = tm.Groups[2].Success ? tm.Groups[2].Value : string.Empty;
                    string modifier = tm.Groups[3].Value;
                    string kind = tm.Groups[4].Value;
                    string typeName = tm.Groups[5].Value;
                    string baseList = tm.Groups[6].Success ? tm.Groups[6].Value : string.Empty;

                    if (string.Equals(modifier, "abstract", StringComparison.Ordinal)
                        || string.Equals(modifier, "static", StringComparison.Ordinal)
                        || string.Equals(visibility, "private", StringComparison.Ordinal)
                        || string.Equals(visibility, "protected", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string body = ExtractClassBody(activeText, tm.Index + tm.Length);
                    bool isStruct = string.Equals(kind, "struct", StringComparison.Ordinal);
                    if (!isStruct)
                    {
                        classMap[typeName] = (ns, baseList, body, filePath);
                    }

                    bool hasSerializableAttr = attrs.Contains("Serializable", StringComparison.Ordinal);
                    if (hasSerializableAttr || string.Equals(visibility, "public", StringComparison.Ordinal))
                    {
                        candidates.Add((typeName, ns, isStruct, body));
                    }
                }
            }

            HashSet<string> emittedFullNames = new HashSet<string>(StringComparer.Ordinal);
            for (int i = 0; i < candidates.Count; i++)
            {
                (string typeName, string ns, bool isValueType, string body) = candidates[i];
                string fullName = string.IsNullOrEmpty(ns) ? typeName : string.Concat(ns, ".", typeName);
                if (!emittedFullNames.Add(fullName))
                {
                    continue;
                }

                List<ScriptSerializedField> fields = isValueType
                    ? ParseFieldsFromBody(body)
                    : ParseFieldsFromClassHierarchy(typeName, classMap);

                results.Add(new SerializableTypeBinding(
                    typeName,
                    ns,
                    assemblyName,
                    isValueType,
                    fields));
            }

            return results;
        }

        public static List<RuntimeInitializeMethodBinding> DiscoverRuntimeInitializeMethodsForAssembly(
            IReadOnlyList<string> sourceFiles,
            HashSet<string> activeDefines)
        {
            List<RuntimeInitializeMethodBinding> results = new List<RuntimeInitializeMethodBinding>();
            List<(string FullTypeName, Match MethodMatch)> matches = DiscoverAttributedMethods(
                sourceFiles,
                activeDefines,
                RuntimeInitializeMethodRegex);
            for (int i = 0; i < matches.Count; i++)
            {
                (string fullTypeName, Match mm) = matches[i];
                string methodVisibility = mm.Groups[1].Success ? mm.Groups[1].Value : "private";
                bool isPublic = string.Equals(methodVisibility, "public", StringComparison.Ordinal);
                if (!isPublic)
                {
                    continue;
                }

                string methodName = mm.Groups["method"].Value.Trim();
                string loadTypeToken = mm.Groups["loadType"].Success
                    ? mm.Groups["loadType"].Value.Trim()
                    : "AfterSceneLoad";
                if (!IsValidRuntimeInitializeLoadTypeToken(loadTypeToken))
                {
                    loadTypeToken = "AfterSceneLoad";
                }

                results.Add(new RuntimeInitializeMethodBinding(fullTypeName, methodName, loadTypeToken));
            }

            return results;
        }

        public static List<EditorCallableMethodBinding> DiscoverEditorCallableMethodsForAssembly(
            IReadOnlyList<string> sourceFiles,
            HashSet<string> activeDefines)
        {
            List<EditorCallableMethodBinding> results = new List<EditorCallableMethodBinding>();
            List<(string FullTypeName, Match MethodMatch)> matches = DiscoverAttributedMethods(
                sourceFiles,
                activeDefines,
                EditorCallableMethodRegex);
            for (int i = 0; i < matches.Count; i++)
            {
                (string fullTypeName, Match mm) = matches[i];
                string methodVisibility = mm.Groups[1].Success ? mm.Groups[1].Value : "private";
                bool isPublic = string.Equals(methodVisibility, "public", StringComparison.Ordinal);
                if (!isPublic)
                {
                    continue;
                }

                string methodName = mm.Groups["method"].Value.Trim();
                string returnType = mm.Groups["returnType"].Value.Trim();
                bool returnsVoid = string.Equals(returnType, "void", StringComparison.Ordinal);
                results.Add(new EditorCallableMethodBinding(fullTypeName, methodName, returnsVoid));
            }

            return results;
        }

        private static List<(string FullTypeName, Match MethodMatch)> DiscoverAttributedMethods(
            IReadOnlyList<string> sourceFiles,
            HashSet<string> activeDefines,
            Regex methodRegex)
        {
            List<(string FullTypeName, Match MethodMatch)> results =
                new List<(string FullTypeName, Match MethodMatch)>();
            for (int i = 0; i < sourceFiles.Count; i++)
            {
                string filePath = Path.GetFullPath(sourceFiles[i]);
                if (!File.Exists(filePath))
                {
                    continue;
                }

                string rawText;
                try
                {
                    rawText = File.ReadAllText(filePath);
                }
                catch (IOException)
                {
                    continue;
                }

                string activeText = FilterConditionalCompilation(rawText, activeDefines);
                string ns = ExtractNamespace(activeText);
                MatchCollection typeMatches = TypeDeclRegex.Matches(activeText);
                for (int tIdx = 0; tIdx < typeMatches.Count; tIdx++)
                {
                    Match tm = typeMatches[tIdx];
                    string visibility = tm.Groups[2].Success ? tm.Groups[2].Value : string.Empty;
                    if (!string.Equals(visibility, "public", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string typeName = tm.Groups[5].Value;
                    string fullTypeName = string.IsNullOrEmpty(ns) ? typeName : string.Concat(ns, ".", typeName);
                    string body = ExtractClassBody(activeText, tm.Index + tm.Length);
                    MatchCollection methodMatches = methodRegex.Matches(body);
                    for (int mIdx = 0; mIdx < methodMatches.Count; mIdx++)
                    {
                        results.Add((fullTypeName, methodMatches[mIdx]));
                    }
                }
            }

            return results;
        }

        private static bool IsValidRuntimeInitializeLoadTypeToken(string token)
        {
            return string.Equals(token, "AfterSceneLoad", StringComparison.Ordinal)
                || string.Equals(token, "BeforeSceneLoad", StringComparison.Ordinal)
                || string.Equals(token, "AfterAssembliesLoaded", StringComparison.Ordinal)
                || string.Equals(token, "BeforeSplashScreen", StringComparison.Ordinal)
                || string.Equals(token, "SubsystemRegistration", StringComparison.Ordinal);
        }

        public static string GenerateGlueSource(IReadOnlyList<ScriptGuidBinding> bindings) =>
            GenerateGlueSource(bindings, Array.Empty<SerializableTypeBinding>(), Array.Empty<RuntimeInitializeMethodBinding>());

        public static string GenerateGlueSource(
            IReadOnlyList<ScriptGuidBinding> bindings,
            IReadOnlyList<SerializableTypeBinding> serializableTypes) =>
            GenerateGlueSource(bindings, serializableTypes, Array.Empty<RuntimeInitializeMethodBinding>());

        public static string GenerateGlueSource(
            IReadOnlyList<ScriptGuidBinding> bindings,
            IReadOnlyList<SerializableTypeBinding> serializableTypes,
            IReadOnlyList<RuntimeInitializeMethodBinding> runtimeInitializeMethods) =>
            GenerateGlueSource(
                bindings,
                serializableTypes,
                runtimeInitializeMethods,
                Array.Empty<EditorCallableMethodBinding>());

        public static string GenerateGlueSource(
            IReadOnlyList<ScriptGuidBinding> bindings,
            IReadOnlyList<SerializableTypeBinding> serializableTypes,
            IReadOnlyList<RuntimeInitializeMethodBinding> runtimeInitializeMethods,
            IReadOnlyList<EditorCallableMethodBinding> editorCallableMethods)
        {
            List<SerializableTypeBinding> allJsonTypes = new List<SerializableTypeBinding>();
            HashSet<string> seenJsonTypes = new HashSet<string>(StringComparer.Ordinal);

            for (int i = 0; i < bindings.Count; i++)
            {
                ScriptGuidBinding b = bindings[i];
                if (seenJsonTypes.Add(b.FullTypeName))
                {
                    allJsonTypes.Add(new SerializableTypeBinding(
                        b.ClassName,
                        b.NamespaceName,
                        b.AssemblyName,
                        false,
                        b.SerializedFields));
                }
            }

            for (int i = 0; i < serializableTypes.Count; i++)
            {
                SerializableTypeBinding st = serializableTypes[i];
                if (seenJsonTypes.Add(st.FullTypeName))
                {
                    allJsonTypes.Add(st);
                }
            }

            StringBuilder sb = new StringBuilder();
            sb.AppendLine("using System;");
            sb.AppendLine("using System.Collections.Generic;");
            sb.AppendLine("using System.Runtime.CompilerServices;");
            sb.AppendLine("using UnityEngine;");
            sb.AppendLine("using Unitas.Bridge;");
            sb.AppendLine("using Unitas.Bridge.Messaging;");
            sb.AppendLine("using Unitas.Bridge.Scripting;");
            sb.AppendLine();
            sb.AppendLine("namespace UnitasGenerated");
            sb.AppendLine("{");

            for (int i = 0; i < bindings.Count; i++)
            {
                ScriptGuidBinding b = bindings[i];
                string handlerName = "GeneratedMessageHandler_" + i.ToString(CultureInfo.InvariantCulture) + "_" + b.ClassName;
                string fullType = "global::" + b.FullTypeName;

                sb.AppendLine("    public sealed class " + handlerName + " : IUnitasMessageHandler");
                sb.AppendLine("    {");
                sb.AppendLine("        private readonly " + fullType + " _target;");
                sb.AppendLine();
                sb.AppendLine("        public " + handlerName + "(" + fullType + " target)");
                sb.AppendLine("        {");
                sb.AppendLine("            _target = target;");
                sb.AppendLine("        }");
                sb.AppendLine();

                HashSet<string> accessorZeroArg = new HashSet<string>(StringComparer.Ordinal);
                for (int mIdx = 0; mIdx < b.Methods.Count; mIdx++)
                {
                    ScriptMethodSignature method = b.Methods[mIdx];
                    if (method.ParameterCount == 0
                        && !method.IsPublic
                        && accessorZeroArg.Add(method.Name))
                    {
                        sb.AppendLine("        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = \"" + method.Name + "\")]");
                        sb.AppendLine("        private static extern void Call0_" + method.Name + "(" + fullType + " target);");
                        sb.AppendLine();
                    }
                }

                HashSet<string> accessorOneArg = new HashSet<string>(StringComparer.Ordinal);
                for (int mIdx = 0; mIdx < b.Methods.Count; mIdx++)
                {
                    ScriptMethodSignature method = b.Methods[mIdx];
                    if (method.ParameterCount == 1
                        && !method.IsPublic
                        && !string.IsNullOrEmpty(method.ParameterType)
                        && accessorOneArg.Add(method.Name))
                    {
                        sb.AppendLine("        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = \"" + method.Name + "\")]");
                        sb.AppendLine("        private static extern void Call1_" + method.Name + "(" + fullType + " target, " + method.ParameterType! + " arg);");
                        sb.AppendLine();
                    }
                }

                sb.AppendLine("        public bool Dispatch(string message)");
                sb.AppendLine("        {");
                sb.AppendLine("            switch (message)");
                sb.AppendLine("            {");

                HashSet<string> emittedZeroArg = new HashSet<string>(StringComparer.Ordinal);
                for (int mIdx = 0; mIdx < b.Methods.Count; mIdx++)
                {
                    ScriptMethodSignature method = b.Methods[mIdx];
                    if (method.ParameterCount == 0 && emittedZeroArg.Add(method.Name))
                    {
                        sb.AppendLine("                case \"" + method.Name + "\":");
                        if (method.IsPublic)
                        {
                            sb.AppendLine("                    _target." + method.Name + "();");
                        }
                        else
                        {
                            sb.AppendLine("                    Call0_" + method.Name + "(_target);");
                        }

                        sb.AppendLine("                    return true;");
                    }
                }

                sb.AppendLine("                default:");
                sb.AppendLine("                    return false;");
                sb.AppendLine("            }");
                sb.AppendLine("        }");
                sb.AppendLine();
                sb.AppendLine("        public bool Dispatch<T>(string message, T argument)");
                sb.AppendLine("        {");
                sb.AppendLine("            switch (message)");
                sb.AppendLine("            {");

                HashSet<string> emittedOneArg = new HashSet<string>(StringComparer.Ordinal);
                for (int mIdx = 0; mIdx < b.Methods.Count; mIdx++)
                {
                    ScriptMethodSignature method = b.Methods[mIdx];
                    if (method.ParameterCount == 1
                        && !string.IsNullOrEmpty(method.ParameterType)
                        && emittedOneArg.Add(method.Name))
                    {
                        string paramType = method.ParameterType!;
                        sb.AppendLine("                case \"" + method.Name + "\":");
                        sb.AppendLine("                    if (argument is " + paramType + " typed_" + mIdx.ToString(CultureInfo.InvariantCulture) + ")");
                        sb.AppendLine("                    {");
                        if (method.IsPublic)
                        {
                            sb.AppendLine("                        _target." + method.Name + "(typed_" + mIdx.ToString(CultureInfo.InvariantCulture) + ");");
                        }
                        else
                        {
                            sb.AppendLine("                        Call1_" + method.Name + "(_target, typed_" + mIdx.ToString(CultureInfo.InvariantCulture) + ");");
                        }

                        sb.AppendLine("                        return true;");
                        sb.AppendLine("                    }");
                        sb.AppendLine("                    return false;");
                    }
                }

                sb.AppendLine("                default:");
                sb.AppendLine("                    return Dispatch(message);");
                sb.AppendLine("            }");
                sb.AppendLine("        }");
                sb.AppendLine("    }");
                sb.AppendLine();
            }

            Dictionary<string, SerializableTypeBinding> jsonTypeBySimpleName =
                new Dictionary<string, SerializableTypeBinding>(StringComparer.Ordinal);
            Dictionary<string, SerializableTypeBinding> jsonTypeByFullName =
                new Dictionary<string, SerializableTypeBinding>(StringComparer.Ordinal);
            for (int i = 0; i < allJsonTypes.Count; i++)
            {
                SerializableTypeBinding st = allJsonTypes[i];
                jsonTypeBySimpleName[st.TypeName] = st;
                jsonTypeByFullName[st.FullTypeName] = st;
            }

            for (int i = 0; i < allJsonTypes.Count; i++)
            {
                SerializableTypeBinding st = allJsonTypes[i];
                string serializerName = "GeneratedJsonSerializer_" + i.ToString(CultureInfo.InvariantCulture) + "_" + st.TypeName;
                string fullType = "global::" + st.FullTypeName;

                sb.AppendLine("    public static class " + serializerName);
                sb.AppendLine("    {");

                for (int fIdx = 0; fIdx < st.Fields.Count; fIdx++)
                {
                    ScriptSerializedField field = st.Fields[fIdx];
                    if (!field.IsPublic
                        && TryResolveFieldTypeExpression(field.TypeName, st.NamespaceName, jsonTypeBySimpleName, jsonTypeByFullName, out string resolvedFieldType, out string _, out string _))
                    {
                        string targetParam = st.IsValueType ? "ref " + fullType + " target" : fullType + " target";
                        sb.AppendLine("        [UnsafeAccessor(UnsafeAccessorKind.Field, Name = \"" + field.Name + "\")]");
                        sb.AppendLine("        private static extern ref " + resolvedFieldType + " FieldRef_" + field.Name + "(" + targetParam + ");");
                        sb.AppendLine();
                    }
                }

                sb.AppendLine("        public static void Write(" + fullType + " value, JsonSerializeWriter writer)");
                sb.AppendLine("        {");
                for (int fIdx = 0; fIdx < st.Fields.Count; fIdx++)
                {
                    ScriptSerializedField field = st.Fields[fIdx];
                    if (!TryResolveFieldTypeExpression(field.TypeName, st.NamespaceName, jsonTypeBySimpleName, jsonTypeByFullName, out string _, out string writeMethod, out string _))
                    {
                        continue;
                    }

                    string accessExpr = field.IsPublic
                        ? "value." + field.Name
                        : (st.IsValueType ? "FieldRef_" + field.Name + "(ref value)" : "FieldRef_" + field.Name + "(value)");
                    sb.AppendLine("            writer." + writeMethod + "(\"" + field.Name + "\", " + accessExpr + ");");
                }

                sb.AppendLine("        }");
                sb.AppendLine();

                sb.AppendLine("        public static void Read(ref " + fullType + " target, JsonParseNode node)");
                sb.AppendLine("        {");
                for (int fIdx = 0; fIdx < st.Fields.Count; fIdx++)
                {
                    ScriptSerializedField field = st.Fields[fIdx];
                    if (!TryResolveFieldTypeExpression(field.TypeName, st.NamespaceName, jsonTypeBySimpleName, jsonTypeByFullName, out string _, out string _, out string readMethod))
                    {
                        continue;
                    }

                    string targetRefExpr = field.IsPublic
                        ? "target." + field.Name
                        : (st.IsValueType ? "FieldRef_" + field.Name + "(ref target)" : "FieldRef_" + field.Name + "(target)");
                    sb.AppendLine("            " + targetRefExpr + " = node." + readMethod + "(\"" + field.Name + "\", " + targetRefExpr + ");");
                }

                sb.AppendLine("        }");
                sb.AppendLine("    }");
                sb.AppendLine();
            }

            sb.AppendLine("    public sealed class GlueInitializer");
            sb.AppendLine("    {");
            sb.AppendLine("        static GlueInitializer()");
            sb.AppendLine("        {");
            sb.AppendLine("            Initialize();");
            sb.AppendLine("        }");
            sb.AppendLine();
            sb.AppendLine("        public GlueInitializer()");
            sb.AppendLine("        {");
            sb.AppendLine("            Initialize();");
            sb.AppendLine("        }");
            sb.AppendLine();
            sb.AppendLine("        [ModuleInitializer]");
            sb.AppendLine("        public static void Initialize()");
            sb.AppendLine("        {");

            for (int i = 0; i < bindings.Count; i++)
            {
                ScriptGuidBinding b = bindings[i];
                string handlerName = "GeneratedMessageHandler_" + i.ToString(CultureInfo.InvariantCulture) + "_" + b.ClassName;
                string fullType = "global::" + b.FullTypeName;
                string hiHex = "0x" + b.GuidHi.ToString("X16", CultureInfo.InvariantCulture) + "UL";
                string loHex = "0x" + b.GuidLo.ToString("X16", CultureInfo.InvariantCulture) + "UL";

                sb.AppendLine("            ScriptGlueRegistry.Register<" + fullType + ">(");
                sb.AppendLine("                " + hiHex + ",");
                sb.AppendLine("                " + loHex + ",");
                sb.AppendLine("                () =>");
                sb.AppendLine("                {");
                sb.AppendLine("                    " + fullType + " inst = new " + fullType + "();");
                sb.AppendLine("                    " + handlerName + " handler = new " + handlerName + "(inst);");
                sb.AppendLine("                    MessageDispatcher.RegisterHandler(inst, handler);");
                sb.AppendLine("                    return inst;");
                sb.AppendLine("                },");
                sb.AppendLine("                inst => new " + handlerName + "(inst));");
            }

            for (int i = 0; i < allJsonTypes.Count; i++)
            {
                SerializableTypeBinding st = allJsonTypes[i];
                string serializerName = "GeneratedJsonSerializer_" + i.ToString(CultureInfo.InvariantCulture) + "_" + st.TypeName;
                string fullType = "global::" + st.FullTypeName;
                sb.AppendLine("            JsonUtility.RegisterType<" + fullType + ">(");
                sb.AppendLine("                () => new " + fullType + "(),");
                sb.AppendLine("                " + serializerName + ".Write,");
                sb.AppendLine("                " + serializerName + ".Read);");
            }

            if (runtimeInitializeMethods != null)
            {
                for (int i = 0; i < runtimeInitializeMethods.Count; i++)
                {
                    RuntimeInitializeMethodBinding rm = runtimeInitializeMethods[i];
                    string fullType = "global::" + rm.FullTypeName;
                    sb.AppendLine("            RuntimeInitializeOnLoadMethodAttribute.Register(");
                    sb.AppendLine("                RuntimeInitializeLoadType." + rm.LoadTypeName + ",");
                    sb.AppendLine("                " + fullType + "." + rm.MethodName + ");");
                }
            }

            if (editorCallableMethods != null)
            {
                for (int i = 0; i < editorCallableMethods.Count; i++)
                {
                    EditorCallableMethodBinding em = editorCallableMethods[i];
                    string fullType = "global::" + em.FullTypeName;
                    sb.AppendLine("            EditorMethodRegistry.Register(");
                    sb.AppendLine("                \"" + em.FullTypeName + "." + em.MethodName + "\",");
                    if (em.ReturnsVoid)
                    {
                        sb.AppendLine("                () =>");
                        sb.AppendLine("                {");
                        sb.AppendLine("                    " + fullType + "." + em.MethodName + "();");
                        sb.AppendLine("                    return null;");
                        sb.AppendLine("                });");
                    }
                    else
                    {
                        sb.AppendLine("                () => " + fullType + "." + em.MethodName + "());");
                    }
                }
            }

            sb.AppendLine("        }");
            sb.AppendLine("    }");
            sb.AppendLine("}");
            return sb.ToString();
        }

        public static string FilterConditionalCompilation(string source, HashSet<string> defines)
        {
            string[] lines = source.Split('\n');
            StringBuilder output = new StringBuilder(source.Length);
            List<(bool ParentActive, bool BranchActive, bool SawTrueBranch)> stack =
                new List<(bool ParentActive, bool BranchActive, bool SawTrueBranch)>();

            bool IsActive()
            {
                if (stack.Count == 0)
                {
                    return true;
                }

                (bool parent, bool branch, bool _) = stack[stack.Count - 1];
                return parent && branch;
            }

            for (int i = 0; i < lines.Length; i++)
            {
                string rawLine = lines[i];
                string trimmed = rawLine.Trim();
                if (trimmed.StartsWith("#if ", StringComparison.Ordinal) || string.Equals(trimmed, "#if", StringComparison.Ordinal))
                {
                    string expr = trimmed.Length > 3 ? trimmed.Substring(3).Trim() : string.Empty;
                    bool parentActive = IsActive();
                    bool cond = parentActive && EvaluatePreprocessorExpression(expr, defines);
                    stack.Add((parentActive, cond, cond));
                    output.Append('\n');
                    continue;
                }

                if (trimmed.StartsWith("#elif ", StringComparison.Ordinal))
                {
                    if (stack.Count > 0)
                    {
                        (bool parentActive, bool _, bool sawTrue) = stack[stack.Count - 1];
                        string expr = trimmed.Substring(5).Trim();
                        bool cond = parentActive && !sawTrue && EvaluatePreprocessorExpression(expr, defines);
                        stack[stack.Count - 1] = (parentActive, cond, sawTrue || cond);
                    }

                    output.Append('\n');
                    continue;
                }

                if (string.Equals(trimmed, "#else", StringComparison.Ordinal)
                    || trimmed.StartsWith("#else ", StringComparison.Ordinal))
                {
                    if (stack.Count > 0)
                    {
                        (bool parentActive, bool _, bool sawTrue) = stack[stack.Count - 1];
                        bool cond = parentActive && !sawTrue;
                        stack[stack.Count - 1] = (parentActive, cond, true);
                    }

                    output.Append('\n');
                    continue;
                }

                if (string.Equals(trimmed, "#endif", StringComparison.Ordinal)
                    || trimmed.StartsWith("#endif ", StringComparison.Ordinal))
                {
                    if (stack.Count > 0)
                    {
                        stack.RemoveAt(stack.Count - 1);
                    }

                    output.Append('\n');
                    continue;
                }

                if (IsActive())
                {
                    output.Append(rawLine).Append('\n');
                }
                else
                {
                    output.Append('\n');
                }
            }

            return output.ToString();
        }

        private static bool EvaluatePreprocessorExpression(string expr, HashSet<string> defines)
        {
            string trimmed = expr.Trim();
            if (string.IsNullOrEmpty(trimmed))
            {
                return false;
            }

            while (trimmed.StartsWith('(') && trimmed.EndsWith(')') && HasMatchingOuterParens(trimmed))
            {
                trimmed = trimmed.Substring(1, trimmed.Length - 2).Trim();
            }

            int orIndex = FindTopLevelOperator(trimmed, "||");
            if (orIndex >= 0)
            {
                string left = trimmed.Substring(0, orIndex);
                string right = trimmed.Substring(orIndex + 2);
                return EvaluatePreprocessorExpression(left, defines) || EvaluatePreprocessorExpression(right, defines);
            }

            int andIndex = FindTopLevelOperator(trimmed, "&&");
            if (andIndex >= 0)
            {
                string left = trimmed.Substring(0, andIndex);
                string right = trimmed.Substring(andIndex + 2);
                return EvaluatePreprocessorExpression(left, defines) && EvaluatePreprocessorExpression(right, defines);
            }

            if (trimmed.StartsWith('!'))
            {
                return !EvaluatePreprocessorExpression(trimmed.Substring(1), defines);
            }

            if (string.Equals(trimmed, "true", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            if (string.Equals(trimmed, "false", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            return defines.Contains(trimmed);
        }

        private static bool HasMatchingOuterParens(string text)
        {
            int depth = 0;
            for (int i = 0; i < text.Length; i++)
            {
                if (text[i] == '(')
                {
                    depth++;
                }
                else if (text[i] == ')')
                {
                    depth--;
                    if (depth == 0 && i < text.Length - 1)
                    {
                        return false;
                    }
                }
            }

            return depth == 0;
        }

        private static int FindTopLevelOperator(string text, string op)
        {
            int depth = 0;
            for (int i = 0; i <= text.Length - op.Length; i++)
            {
                char c = text[i];
                if (c == '(')
                {
                    depth++;
                }
                else if (c == ')')
                {
                    depth--;
                }
                else if (depth == 0 && string.CompareOrdinal(text, i, op, 0, op.Length) == 0)
                {
                    return i;
                }
            }

            return -1;
        }

        private static bool InheritsFromMonoBehaviour(
            string className,
            Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)> classMap,
            HashSet<string> visited)
        {
            if (!visited.Add(className))
            {
                return false;
            }

            if (!classMap.TryGetValue(className, out (string NamespaceName, string BaseList, string BodyText, string FilePath) entry))
            {
                return false;
            }

            if (string.IsNullOrEmpty(entry.BaseList))
            {
                return false;
            }

            string[] bases = entry.BaseList.Split(',');
            for (int i = 0; i < bases.Length; i++)
            {
                string baseType = bases[i].Trim();
                if (string.Equals(baseType, "MonoBehaviour", StringComparison.Ordinal)
                    || string.Equals(baseType, "UnityEngine.MonoBehaviour", StringComparison.Ordinal)
                    || baseType.EndsWith(".MonoBehaviour", StringComparison.Ordinal))
                {
                    return true;
                }

                string simpleBase = baseType;
                int lastDot = simpleBase.LastIndexOf('.');
                if (lastDot >= 0)
                {
                    simpleBase = simpleBase.Substring(lastDot + 1);
                }

                if (InheritsFromMonoBehaviour(simpleBase, classMap, visited))
                {
                    return true;
                }
            }

            return false;
        }

        private static List<ScriptMethodSignature> ParseMethodsFromClassHierarchy(
            string className,
            Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)> classMap)
        {
            List<ScriptMethodSignature> methods = new List<ScriptMethodSignature>();
            HashSet<string> visited = new HashSet<string>(StringComparer.Ordinal);
            string? current = className;

            while (!string.IsNullOrEmpty(current) && visited.Add(current))
            {
                if (!classMap.TryGetValue(current, out (string NamespaceName, string BaseList, string BodyText, string FilePath) entry))
                {
                    break;
                }

                ParseMethodsFromBody(entry.BodyText, methods);

                string? nextBase = null;
                if (!string.IsNullOrEmpty(entry.BaseList))
                {
                    string[] bases = entry.BaseList.Split(',');
                    for (int i = 0; i < bases.Length; i++)
                    {
                        string candidate = bases[i].Trim();
                        int dot = candidate.LastIndexOf('.');
                        string simple = dot >= 0 ? candidate.Substring(dot + 1) : candidate;
                        if (classMap.ContainsKey(simple))
                        {
                            nextBase = simple;
                            break;
                        }
                    }
                }

                current = nextBase;
            }

            return methods;
        }

        private static List<ScriptSerializedField> ParseFieldsFromClassHierarchy(
            string className,
            Dictionary<string, (string NamespaceName, string BaseList, string BodyText, string FilePath)> classMap)
        {
            List<string> hierarchy = new List<string>();
            HashSet<string> visited = new HashSet<string>(StringComparer.Ordinal);
            string? current = className;

            while (!string.IsNullOrEmpty(current) && visited.Add(current))
            {
                if (!classMap.TryGetValue(current, out (string NamespaceName, string BaseList, string BodyText, string FilePath) entry))
                {
                    break;
                }

                hierarchy.Add(current);

                string? nextBase = null;
                if (!string.IsNullOrEmpty(entry.BaseList))
                {
                    string[] bases = entry.BaseList.Split(',');
                    for (int i = 0; i < bases.Length; i++)
                    {
                        string candidate = bases[i].Trim();
                        int dot = candidate.LastIndexOf('.');
                        string simple = dot >= 0 ? candidate.Substring(dot + 1) : candidate;
                        if (classMap.ContainsKey(simple))
                        {
                            nextBase = simple;
                            break;
                        }
                    }
                }

                current = nextBase;
            }

            List<ScriptSerializedField> fields = new List<ScriptSerializedField>();
            HashSet<string> seenNames = new HashSet<string>(StringComparer.Ordinal);
            for (int i = hierarchy.Count - 1; i >= 0; i--)
            {
                string typeKey = hierarchy[i];
                if (!classMap.TryGetValue(typeKey, out (string NamespaceName, string BaseList, string BodyText, string FilePath) entry))
                {
                    continue;
                }

                List<ScriptSerializedField> bodyFields = ParseFieldsFromBody(entry.BodyText);
                for (int f = 0; f < bodyFields.Count; f++)
                {
                    if (seenNames.Add(bodyFields[f].Name))
                    {
                        fields.Add(bodyFields[f]);
                    }
                }
            }

            return fields;
        }

        private static List<ScriptSerializedField> ParseFieldsFromBody(string bodyText)
        {
            List<ScriptSerializedField> fields = new List<ScriptSerializedField>();
            if (string.IsNullOrEmpty(bodyText))
            {
                return fields;
            }

            string[] lines = bodyText.Split('\n');
            int braceDepth = 0;
            StringBuilder pendingAttributes = new StringBuilder();

            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i];
                string trimmed = line.Trim();

                if (braceDepth == 1 && trimmed.StartsWith('[') && trimmed.EndsWith(']') && !trimmed.Contains(';', StringComparison.Ordinal))
                {
                    pendingAttributes.Append(trimmed).Append(' ');
                    continue;
                }

                int depthBefore = braceDepth;
                for (int cIdx = 0; cIdx < line.Length; cIdx++)
                {
                    char c = line[cIdx];
                    if (c == '{')
                    {
                        braceDepth++;
                    }
                    else if (c == '}')
                    {
                        braceDepth--;
                    }
                }

                if (depthBefore == 1 && braceDepth == 1)
                {
                    string candidateLine = pendingAttributes.Length > 0
                        ? pendingAttributes.ToString() + trimmed
                        : trimmed;
                    pendingAttributes.Clear();

                    if (candidateLine.Contains('(', StringComparison.Ordinal) && !candidateLine.Contains('=', StringComparison.Ordinal))
                    {
                        int semiIdx = candidateLine.IndexOf(';', StringComparison.Ordinal);
                        int parenIdx = candidateLine.LastIndexOf(')');
                        if (parenIdx > 0 && semiIdx > parenIdx)
                        {
                            continue;
                        }
                    }

                    // Expression-bodied members ("public int Score => _score;") are properties, not fields.
                    if (candidateLine.Contains("=>", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    Match m = FieldDeclRegex.Match(candidateLine);
                    if (!m.Success)
                    {
                        continue;
                    }

                    string attrs = m.Groups[1].Value;
                    string visibility = m.Groups[2].Success ? m.Groups[2].Value : "private";
                    string modifiers = m.Groups[3].Value;
                    string fieldType = m.Groups[4].Value.Trim();
                    string fieldName = m.Groups[5].Value.Trim();

                    if (string.Equals(fieldType, "event", StringComparison.Ordinal)
                        || fieldType.StartsWith("event ", StringComparison.Ordinal)
                        || string.Equals(fieldType, "return", StringComparison.Ordinal)
                        || string.Equals(fieldType, "throw", StringComparison.Ordinal)
                        || string.Equals(fieldType, "yield", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    if (modifiers.Contains("static", StringComparison.Ordinal)
                        || modifiers.Contains("const", StringComparison.Ordinal)
                        || modifiers.Contains("readonly", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    if (attrs.Contains("NonSerialized", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    bool isPublic = string.Equals(visibility, "public", StringComparison.Ordinal);
                    bool hasSerializeField = attrs.Contains("SerializeField", StringComparison.Ordinal);
                    if (!isPublic && !hasSerializeField)
                    {
                        continue;
                    }

                    fields.Add(new ScriptSerializedField(fieldName, fieldType, isPublic, hasSerializeField));
                }
                else
                {
                    pendingAttributes.Clear();
                }
            }

            return fields;
        }

        private static bool TryResolveFieldTypeExpression(
            string rawTypeName,
            string declaringNamespace,
            Dictionary<string, SerializableTypeBinding> jsonTypeBySimpleName,
            Dictionary<string, SerializableTypeBinding> jsonTypeByFullName,
            out string csharpTypeExpr,
            out string writeMethodName,
            out string readMethodName)
        {
            string cleaned = rawTypeName.Trim();
            if (cleaned.EndsWith('?'))
            {
                cleaned = cleaned.Substring(0, cleaned.Length - 1).Trim();
            }

            csharpTypeExpr = string.Empty;
            writeMethodName = string.Empty;
            readMethodName = string.Empty;

            if (string.Equals(cleaned, "int", StringComparison.Ordinal) || string.Equals(cleaned, "Int32", StringComparison.Ordinal) || string.Equals(cleaned, "System.Int32", StringComparison.Ordinal))
            {
                csharpTypeExpr = "int";
                writeMethodName = "WriteInt";
                readMethodName = "ReadInt";
                return true;
            }

            if (string.Equals(cleaned, "long", StringComparison.Ordinal) || string.Equals(cleaned, "Int64", StringComparison.Ordinal) || string.Equals(cleaned, "System.Int64", StringComparison.Ordinal))
            {
                csharpTypeExpr = "long";
                writeMethodName = "WriteLong";
                readMethodName = "ReadLong";
                return true;
            }

            if (string.Equals(cleaned, "float", StringComparison.Ordinal) || string.Equals(cleaned, "Single", StringComparison.Ordinal) || string.Equals(cleaned, "System.Single", StringComparison.Ordinal))
            {
                csharpTypeExpr = "float";
                writeMethodName = "WriteFloat";
                readMethodName = "ReadFloat";
                return true;
            }

            if (string.Equals(cleaned, "double", StringComparison.Ordinal) || string.Equals(cleaned, "Double", StringComparison.Ordinal) || string.Equals(cleaned, "System.Double", StringComparison.Ordinal))
            {
                csharpTypeExpr = "double";
                writeMethodName = "WriteDouble";
                readMethodName = "ReadDouble";
                return true;
            }

            if (string.Equals(cleaned, "bool", StringComparison.Ordinal) || string.Equals(cleaned, "Boolean", StringComparison.Ordinal) || string.Equals(cleaned, "System.Boolean", StringComparison.Ordinal))
            {
                csharpTypeExpr = "bool";
                writeMethodName = "WriteBool";
                readMethodName = "ReadBool";
                return true;
            }

            if (string.Equals(cleaned, "string", StringComparison.Ordinal) || string.Equals(cleaned, "String", StringComparison.Ordinal) || string.Equals(cleaned, "System.String", StringComparison.Ordinal))
            {
                csharpTypeExpr = "string";
                writeMethodName = "WriteString";
                readMethodName = "ReadString";
                return true;
            }

            if (string.Equals(cleaned, "Vector2", StringComparison.Ordinal) || string.Equals(cleaned, "UnityEngine.Vector2", StringComparison.Ordinal))
            {
                csharpTypeExpr = "global::UnityEngine.Vector2";
                writeMethodName = "WriteVector2";
                readMethodName = "ReadVector2";
                return true;
            }

            if (string.Equals(cleaned, "Vector3", StringComparison.Ordinal) || string.Equals(cleaned, "UnityEngine.Vector3", StringComparison.Ordinal))
            {
                csharpTypeExpr = "global::UnityEngine.Vector3";
                writeMethodName = "WriteVector3";
                readMethodName = "ReadVector3";
                return true;
            }

            if (string.Equals(cleaned, "Vector4", StringComparison.Ordinal) || string.Equals(cleaned, "UnityEngine.Vector4", StringComparison.Ordinal))
            {
                csharpTypeExpr = "global::UnityEngine.Vector4";
                writeMethodName = "WriteVector4";
                readMethodName = "ReadVector4";
                return true;
            }

            if (string.Equals(cleaned, "Quaternion", StringComparison.Ordinal) || string.Equals(cleaned, "UnityEngine.Quaternion", StringComparison.Ordinal))
            {
                csharpTypeExpr = "global::UnityEngine.Quaternion";
                writeMethodName = "WriteQuaternion";
                readMethodName = "ReadQuaternion";
                return true;
            }

            if (string.Equals(cleaned, "Rect", StringComparison.Ordinal) || string.Equals(cleaned, "UnityEngine.Rect", StringComparison.Ordinal))
            {
                csharpTypeExpr = "global::UnityEngine.Rect";
                writeMethodName = "WriteRect";
                readMethodName = "ReadRect";
                return true;
            }

            if (string.Equals(cleaned, "Bounds", StringComparison.Ordinal) || string.Equals(cleaned, "UnityEngine.Bounds", StringComparison.Ordinal))
            {
                csharpTypeExpr = "global::UnityEngine.Bounds";
                writeMethodName = "WriteBounds";
                readMethodName = "ReadBounds";
                return true;
            }

            if (cleaned.EndsWith("[]", StringComparison.Ordinal))
            {
                string elem = cleaned.Substring(0, cleaned.Length - 2).Trim();
                if (elem.Contains('[', StringComparison.Ordinal))
                {
                    return false;
                }

                if (string.Equals(elem, "int", StringComparison.Ordinal) || string.Equals(elem, "Int32", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "int[]";
                    writeMethodName = "WriteIntList";
                    readMethodName = "ReadIntArray";
                    return true;
                }

                if (string.Equals(elem, "float", StringComparison.Ordinal) || string.Equals(elem, "Single", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "float[]";
                    writeMethodName = "WriteFloatList";
                    readMethodName = "ReadFloatArray";
                    return true;
                }

                if (string.Equals(elem, "bool", StringComparison.Ordinal) || string.Equals(elem, "Boolean", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "bool[]";
                    writeMethodName = "WriteBoolList";
                    readMethodName = "ReadBoolArray";
                    return true;
                }

                if (string.Equals(elem, "string", StringComparison.Ordinal) || string.Equals(elem, "String", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "string[]";
                    writeMethodName = "WriteStringList";
                    readMethodName = "ReadStringArray";
                    return true;
                }

                if (TryResolveCustomJsonType(elem, declaringNamespace, jsonTypeBySimpleName, jsonTypeByFullName, out string elemFull))
                {
                    csharpTypeExpr = elemFull + "[]";
                    writeMethodName = "WriteNestedList<" + elemFull + ">";
                    readMethodName = "ReadNestedArray<" + elemFull + ">";
                    return true;
                }

                return false;
            }

            if ((cleaned.StartsWith("List<", StringComparison.Ordinal) || cleaned.StartsWith("System.Collections.Generic.List<", StringComparison.Ordinal))
                && cleaned.EndsWith('>'))
            {
                int openAngle = cleaned.IndexOf('<', StringComparison.Ordinal);
                string elem = cleaned.Substring(openAngle + 1, cleaned.Length - openAngle - 2).Trim();
                if (elem.Contains('<', StringComparison.Ordinal) || elem.Contains('[', StringComparison.Ordinal))
                {
                    return false;
                }

                if (string.Equals(elem, "int", StringComparison.Ordinal) || string.Equals(elem, "Int32", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "global::System.Collections.Generic.List<int>";
                    writeMethodName = "WriteIntList";
                    readMethodName = "ReadIntList";
                    return true;
                }

                if (string.Equals(elem, "float", StringComparison.Ordinal) || string.Equals(elem, "Single", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "global::System.Collections.Generic.List<float>";
                    writeMethodName = "WriteFloatList";
                    readMethodName = "ReadFloatList";
                    return true;
                }

                if (string.Equals(elem, "bool", StringComparison.Ordinal) || string.Equals(elem, "Boolean", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "global::System.Collections.Generic.List<bool>";
                    writeMethodName = "WriteBoolList";
                    readMethodName = "ReadBoolList";
                    return true;
                }

                if (string.Equals(elem, "string", StringComparison.Ordinal) || string.Equals(elem, "String", StringComparison.Ordinal))
                {
                    csharpTypeExpr = "global::System.Collections.Generic.List<string>";
                    writeMethodName = "WriteStringList";
                    readMethodName = "ReadStringList";
                    return true;
                }

                if (TryResolveCustomJsonType(elem, declaringNamespace, jsonTypeBySimpleName, jsonTypeByFullName, out string elemFull))
                {
                    csharpTypeExpr = "global::System.Collections.Generic.List<" + elemFull + ">";
                    writeMethodName = "WriteNestedList<" + elemFull + ">";
                    readMethodName = "ReadNestedList<" + elemFull + ">";
                    return true;
                }

                return false;
            }

            if (TryResolveCustomJsonType(cleaned, declaringNamespace, jsonTypeBySimpleName, jsonTypeByFullName, out string customFull))
            {
                csharpTypeExpr = customFull;
                writeMethodName = "WriteNested<" + customFull + ">";
                readMethodName = "ReadNested<" + customFull + ">";
                return true;
            }

            return false;
        }

        private static bool TryResolveCustomJsonType(
            string rawTypeName,
            string declaringNamespace,
            Dictionary<string, SerializableTypeBinding> jsonTypeBySimpleName,
            Dictionary<string, SerializableTypeBinding> jsonTypeByFullName,
            out string globalFullName)
        {
            globalFullName = string.Empty;
            if (jsonTypeByFullName.TryGetValue(rawTypeName, out SerializableTypeBinding? exactFull))
            {
                globalFullName = "global::" + exactFull.FullTypeName;
                return true;
            }

            if (!string.IsNullOrEmpty(declaringNamespace))
            {
                string scopedCandidate = declaringNamespace + "." + rawTypeName;
                if (jsonTypeByFullName.TryGetValue(scopedCandidate, out SerializableTypeBinding? scopedMatch))
                {
                    globalFullName = "global::" + scopedMatch.FullTypeName;
                    return true;
                }
            }

            if (jsonTypeBySimpleName.TryGetValue(rawTypeName, out SerializableTypeBinding? simpleMatch))
            {
                globalFullName = "global::" + simpleMatch.FullTypeName;
                return true;
            }

            return false;
        }

        private static void ParseMethodsFromBody(string bodyText, List<ScriptMethodSignature> methods)
        {
            MatchCollection matches = MethodRegex.Matches(bodyText);
            for (int i = 0; i < matches.Count; i++)
            {
                Match m = matches[i];
                string visibility = m.Groups[2].Success ? m.Groups[2].Value : "private";
                bool isPublic = string.Equals(visibility, "public", StringComparison.Ordinal);
                if (string.Equals(m.Groups[3].Value, "override", StringComparison.Ordinal))
                {
                    continue;
                }

                string methodName = m.Groups[5].Value.Trim();
                if (ReservedMethodNames.Contains(methodName))
                {
                    continue;
                }

                string rawParams = m.Groups[6].Value.Trim();
                if (string.IsNullOrEmpty(rawParams))
                {
                    methods.Add(new ScriptMethodSignature(methodName, isPublic, 0, null, true));
                    continue;
                }

                if (rawParams.Contains(',', StringComparison.Ordinal))
                {
                    continue;
                }

                string? paramType = ParseSingleParameterType(rawParams);
                if (!string.IsNullOrEmpty(paramType))
                {
                    methods.Add(new ScriptMethodSignature(methodName, isPublic, 1, paramType, true));
                }
            }
        }

        private static string? ParseSingleParameterType(string paramDecl)
        {
            string trimmed = paramDecl.Trim();
            int eqIdx = trimmed.IndexOf('=', StringComparison.Ordinal);
            if (eqIdx >= 0)
            {
                trimmed = trimmed.Substring(0, eqIdx).Trim();
            }

            if (trimmed.StartsWith("ref ", StringComparison.Ordinal)
                || trimmed.StartsWith("out ", StringComparison.Ordinal)
                || trimmed.StartsWith("in ", StringComparison.Ordinal)
                || trimmed.StartsWith("params ", StringComparison.Ordinal))
            {
                return null;
            }

            int lastSpace = trimmed.LastIndexOfAny(new[] { ' ', '\t' });
            if (lastSpace <= 0)
            {
                return null;
            }

            string typePart = trimmed.Substring(0, lastSpace).Trim();
            if (typePart.EndsWith('?'))
            {
                typePart = typePart.Substring(0, typePart.Length - 1).Trim();
            }

            return string.IsNullOrEmpty(typePart) ? null : typePart;
        }

        private static string ExtractNamespace(string source)
        {
            Match m = NamespaceRegex.Match(source);
            return m.Success ? m.Groups[1].Value.Trim() : string.Empty;
        }

        private static string ExtractClassBody(string source, int startSearchIndex)
        {
            int openBrace = source.IndexOf('{', startSearchIndex);
            if (openBrace < 0)
            {
                return string.Empty;
            }

            int depth = 0;
            for (int i = openBrace; i < source.Length; i++)
            {
                char c = source[i];
                if (c == '{')
                {
                    depth++;
                }
                else if (c == '}')
                {
                    depth--;
                    if (depth == 0)
                    {
                        return source.Substring(openBrace, i - openBrace + 1);
                    }
                }
            }

            return source.Substring(openBrace);
        }

        private static int CountLinesBefore(string text, int index)
        {
            int lines = 1;
            int limit = Math.Min(index, text.Length);
            for (int i = 0; i < limit; i++)
            {
                if (text[i] == '\n')
                {
                    lines++;
                }
            }

            return lines;
        }
    }
}
