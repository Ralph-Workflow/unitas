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

    public static class ScriptAnalyzerAndGlueGenerator
    {
        private static readonly Regex NamespaceRegex = new Regex(
            @"\bnamespace\s+([A-Za-z_][A-Za-z0-9_.]*)",
            RegexOptions.Compiled);

        private static readonly Regex ClassRegex = new Regex(
            @"(?:(public|internal|private|protected)\s+)?(?:(abstract|sealed|static|partial)\s+)*class\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*:\s*([^{]+))?",
            RegexOptions.Compiled);

        private static readonly Regex MethodRegex = new Regex(
            @"(?:^|[;{}])\s*(?:(\[[^\]]*\]\s*)*)(?:(public|private|protected|internal)\s+)?(?:(virtual|override|new|sealed|async)\s+)*(void|IEnumerator|System\.Collections\.IEnumerator)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?:\{|=>)",
            RegexOptions.Compiled | RegexOptions.Multiline);

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
                bindings.Add(new ScriptGuidBinding(
                    guid,
                    hi,
                    lo,
                    filePath,
                    className,
                    ns,
                    assemblyName,
                    methods));
            }

            return bindings;
        }

        public static string GenerateGlueSource(IReadOnlyList<ScriptGuidBinding> bindings)
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("using System;");
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
                    if (method.ParameterCount == 0 && !method.IsPublic && accessorZeroArg.Add(method.Name))
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

        private static void ParseMethodsFromBody(string bodyText, List<ScriptMethodSignature> methods)
        {
            MatchCollection matches = MethodRegex.Matches(bodyText);
            for (int i = 0; i < matches.Count; i++)
            {
                Match m = matches[i];
                string visibility = m.Groups[2].Success ? m.Groups[2].Value : "private";
                bool isPublic = string.Equals(visibility, "public", StringComparison.Ordinal);
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
            int eqIdx = trimmed.IndexOf('=');
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
