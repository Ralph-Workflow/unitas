using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Unitas.Scripting;

namespace Unitas.Testing
{
    public static class TestDiscoveryAndGlueGenerator
    {
        private static readonly Regex NamespaceRegex = new Regex(
            @"\bnamespace\s+([A-Za-z_][A-Za-z0-9_.]*)",
            RegexOptions.Compiled);

        private static readonly Regex ClassHeaderRegex = new Regex(
            @"((?:\s*\[[^\]]+\]\s*)*)(?:(public|internal|private|protected)\s+)?(?:(abstract|sealed|static|partial)\s+)*class\s+([A-Za-z_][A-Za-z0-9_]*)",
            RegexOptions.Compiled);

        private static readonly Regex MethodHeaderRegex = new Regex(
            @"((?:\s*\[[^\]]+\]\s*)+)(?:(public|private|protected|internal)\s+)?(?:(virtual|override|new|sealed|static)\s+)*(void|IEnumerator|System\.Collections\.IEnumerator)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
            RegexOptions.Compiled);

        private static readonly Regex CategoryAttrRegex = new Regex(
            @"\bCategory(?:Attribute)?\s*\(\s*""([^""]*)""\s*\)",
            RegexOptions.Compiled);

        private static readonly Regex IgnoreAttrRegex = new Regex(
            @"\bIgnore(?:Attribute)?\s*\(\s*""([^""]*)""\s*\)",
            RegexOptions.Compiled);

        private static readonly Regex TestAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])Test(?:Attribute)?\s*(?:\(\s*\))?(?![A-Za-z0-9_])",
            RegexOptions.Compiled);

        private static readonly Regex UnityTestAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])UnityTest(?:Attribute)?\s*(?:\(\s*\))?(?![A-Za-z0-9_])",
            RegexOptions.Compiled);

        private static readonly Regex SetUpAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])SetUp(?:Attribute)?\s*(?:\(\s*\))?(?![A-Za-z0-9_])",
            RegexOptions.Compiled);

        private static readonly Regex TearDownAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])TearDown(?:Attribute)?\s*(?:\(\s*\))?(?![A-Za-z0-9_])",
            RegexOptions.Compiled);

        private static readonly Regex TestCaseAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])TestCase(?:Attribute)?\s*\(([^]]*?)\)\s*(?=\]|,)",
            RegexOptions.Compiled);

        public static List<DiscoveredTestAssembly> DiscoverTestAssemblies(
            string projectRoot,
            out Dictionary<string, List<string>> scriptsByAssembly,
            out List<ScriptDiagnostic> diagnostics)
        {
            diagnostics = new List<ScriptDiagnostic>();
            Dictionary<string, string> guidToPath = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            Dictionary<string, string> pathToGuid = new Dictionary<string, string>(StringComparer.Ordinal);
            List<ParsedPluginMeta> parsedPlugins = new List<ParsedPluginMeta>();

            ProjectMetadataIndex.ScanProjectMeta(projectRoot, guidToPath, pathToGuid, parsedPlugins);
            List<AssemblyDefinitionModel> customAssemblies = AssemblyDiscovery.DiscoverAssemblies(
                projectRoot,
                guidToPath,
                pathToGuid,
                diagnostics);

            scriptsByAssembly = AssemblyDiscovery.AssignScriptsToAssemblies(
                projectRoot,
                customAssemblies,
                out List<AssemblyDefinitionModel> _);

            List<DiscoveredTestAssembly> testAssemblies = new List<DiscoveredTestAssembly>();
            for (int i = 0; i < customAssemblies.Count; i++)
            {
                AssemblyDefinitionModel model = customAssemblies[i];
                if (!IsTestAssemblyDefinition(model))
                {
                    continue;
                }

                TestExecutionMode mode = DetermineExecutionMode(model);
                List<string> sources = scriptsByAssembly.TryGetValue(model.Name, out List<string>? list)
                    ? list
                    : new List<string>();
                testAssemblies.Add(new DiscoveredTestAssembly(
                    model.Name,
                    model.FilePath,
                    mode,
                    sources,
                    true,
                    null));
            }

            return testAssemblies;
        }

        public static bool IsTestAssemblyDefinition(AssemblyDefinitionModel model)
        {
            for (int i = 0; i < model.DefineConstraints.Count; i++)
            {
                if (model.DefineConstraints[i].Contains("UNITY_INCLUDE_TESTS", StringComparison.Ordinal))
                {
                    return true;
                }
            }

            for (int i = 0; i < model.References.Count; i++)
            {
                string r = model.References[i];
                if (string.Equals(r, "UnityEngine.TestRunner", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(r, "UnityEditor.TestRunner", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(r, "Unitas.Testing.NUnit", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(r, "nunit.framework", StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            for (int i = 0; i < model.PrecompiledReferences.Count; i++)
            {
                string p = model.PrecompiledReferences[i];
                if (p.Contains("nunit.framework", StringComparison.OrdinalIgnoreCase)
                    || p.Contains("Unitas.Testing.NUnit", StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            if (!string.IsNullOrEmpty(model.FilePath) && File.Exists(model.FilePath))
            {
                try
                {
                    using JsonDocument doc = JsonDocument.Parse(File.ReadAllText(model.FilePath));
                    if (doc.RootElement.TryGetProperty("optionalUnityReferences", out JsonElement optElem)
                        && optElem.ValueKind == JsonValueKind.Array)
                    {
                        foreach (JsonElement item in optElem.EnumerateArray())
                        {
                            if (item.ValueKind == JsonValueKind.String
                                && string.Equals(item.GetString(), "TestAssemblies", StringComparison.OrdinalIgnoreCase))
                            {
                                return true;
                            }
                        }
                    }
                }
                catch (Exception ex) when (ex is IOException || ex is JsonException)
                {
                }
            }

            return false;
        }

        public static TestExecutionMode DetermineExecutionMode(AssemblyDefinitionModel model)
        {
            if (model.IncludePlatforms.Count == 1
                && string.Equals(model.IncludePlatforms[0], "Editor", StringComparison.OrdinalIgnoreCase))
            {
                return TestExecutionMode.EditMode;
            }

            bool hasEditor = false;
            bool hasNonEditor = false;
            for (int i = 0; i < model.IncludePlatforms.Count; i++)
            {
                if (string.Equals(model.IncludePlatforms[i], "Editor", StringComparison.OrdinalIgnoreCase))
                {
                    hasEditor = true;
                }
                else
                {
                    hasNonEditor = true;
                }
            }

            if (hasEditor && !hasNonEditor)
            {
                return TestExecutionMode.EditMode;
            }

            return TestExecutionMode.PlayMode;
        }

        public static List<DiscoveredTestCaseDescriptor> DiscoverTestsInAssembly(
            DiscoveredTestAssembly assembly,
            HashSet<string> activeDefines,
            out List<FixtureGenerationPlan> fixturePlans)
        {
            List<DiscoveredTestCaseDescriptor> descriptors = new List<DiscoveredTestCaseDescriptor>();
            fixturePlans = new List<FixtureGenerationPlan>();

            for (int f = 0; f < assembly.SourceFiles.Count; f++)
            {
                string filePath = assembly.SourceFiles[f];
                if (!File.Exists(filePath))
                {
                    continue;
                }

                string rawSource;
                try
                {
                    rawSource = File.ReadAllText(filePath);
                }
                catch (IOException)
                {
                    continue;
                }

                string cleanSource = StripLineAndBlockComments(
                    ScriptAnalyzerAndGlueGenerator.FilterConditionalCompilation(rawSource, activeDefines));
                string ns = ExtractNamespace(cleanSource);

                MatchCollection classMatches = ClassHeaderRegex.Matches(cleanSource);
                for (int cIdx = 0; cIdx < classMatches.Count; cIdx++)
                {
                    Match cm = classMatches[cIdx];
                    string modifier = cm.Groups[3].Value;
                    if (string.Equals(modifier, "abstract", StringComparison.Ordinal)
                        || string.Equals(modifier, "static", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string classAttrs = cm.Groups[1].Value;
                    string className = cm.Groups[4].Value;
                    string classBody = ExtractBraceBody(cleanSource, cm.Index + cm.Length);
                    if (string.IsNullOrEmpty(classBody))
                    {
                        continue;
                    }

                    List<string> classCategories = ExtractCategories(classAttrs);
                    string? classIgnoreReason = ExtractIgnoreReason(classAttrs);

                    List<string> setUpMethods = new List<string>();
                    List<string> tearDownMethods = new List<string>();
                    List<MethodGenerationPlan> methodPlans = new List<MethodGenerationPlan>();

                    MatchCollection methodMatches = MethodHeaderRegex.Matches(classBody);
                    for (int mIdx = 0; mIdx < methodMatches.Count; mIdx++)
                    {
                        Match mm = methodMatches[mIdx];
                        string methodAttrs = mm.Groups[1].Value;
                        string returnType = mm.Groups[4].Value.Trim();
                        string methodName = mm.Groups[5].Value.Trim();

                        if (SetUpAttrRegex.IsMatch(methodAttrs))
                        {
                            setUpMethods.Add(methodName);
                            continue;
                        }

                        if (TearDownAttrRegex.IsMatch(methodAttrs))
                        {
                            tearDownMethods.Add(methodName);
                            continue;
                        }

                        bool isUnityTest = UnityTestAttrRegex.IsMatch(methodAttrs);
                        bool isTest = TestAttrRegex.IsMatch(methodAttrs);
                        MatchCollection testCaseMatches = TestCaseAttrRegex.Matches(methodAttrs);

                        if (!isUnityTest && !isTest && testCaseMatches.Count == 0)
                        {
                            continue;
                        }

                        bool returnsEnumerator = returnType.EndsWith("IEnumerator", StringComparison.Ordinal);
                        List<string> combinedCategories = new List<string>(classCategories);
                        List<string> methodCategories = ExtractCategories(methodAttrs);
                        for (int catIdx = 0; catIdx < methodCategories.Count; catIdx++)
                        {
                            if (!combinedCategories.Contains(methodCategories[catIdx]))
                            {
                                combinedCategories.Add(methodCategories[catIdx]);
                            }
                        }

                        string? methodIgnore = ExtractIgnoreReason(methodAttrs) ?? classIgnoreReason;
                        bool isIgnored = methodIgnore != null;

                        string fullClassName = string.IsNullOrEmpty(ns)
                            ? className
                            : string.Concat(ns, ".", className);

                        if (testCaseMatches.Count > 0)
                        {
                            for (int tcIdx = 0; tcIdx < testCaseMatches.Count; tcIdx++)
                            {
                                string rawArgs = testCaseMatches[tcIdx].Groups[1].Value.Trim();
                                List<string> argExprs = SplitArguments(rawArgs);
                                string displayArgs = string.Join(",", argExprs);
                                string displayName = methodName + "(" + displayArgs + ")";
                                string fullName = fullClassName + "." + displayName;
                                string id = assembly.Name + "::" + fullName + "#" + tcIdx.ToString(CultureInfo.InvariantCulture);

                                DiscoveredTestCaseDescriptor desc = new DiscoveredTestCaseDescriptor(
                                    id,
                                    assembly.Name,
                                    assembly.Mode,
                                    ns,
                                    className,
                                    methodName,
                                    displayName,
                                    fullName,
                                    isUnityTest || returnsEnumerator,
                                    isIgnored,
                                    methodIgnore,
                                    combinedCategories,
                                    argExprs);

                                descriptors.Add(desc);
                                methodPlans.Add(new MethodGenerationPlan(desc, argExprs));
                            }
                        }
                        else
                        {
                            string displayName = methodName;
                            string fullName = fullClassName + "." + methodName;
                            string id = assembly.Name + "::" + fullName;

                            DiscoveredTestCaseDescriptor desc = new DiscoveredTestCaseDescriptor(
                                id,
                                assembly.Name,
                                assembly.Mode,
                                ns,
                                className,
                                methodName,
                                displayName,
                                fullName,
                                isUnityTest || returnsEnumerator,
                                isIgnored,
                                methodIgnore,
                                combinedCategories,
                                Array.Empty<string>());

                            descriptors.Add(desc);
                            methodPlans.Add(new MethodGenerationPlan(desc, Array.Empty<string>()));
                        }
                    }

                    if (methodPlans.Count > 0)
                    {
                        fixturePlans.Add(new FixtureGenerationPlan(
                            ns,
                            className,
                            setUpMethods,
                            tearDownMethods,
                            methodPlans));
                    }
                }
            }

            return descriptors;
        }

        public static string GenerateTestRunnerGlueSource(IReadOnlyList<FixtureGenerationPlan> fixtures)
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("using System;");
            sb.AppendLine("using System.Collections;");
            sb.AppendLine("using System.Runtime.CompilerServices;");
            sb.AppendLine("using Unitas.Testing;");
            sb.AppendLine();
            sb.AppendLine("namespace UnitasGeneratedTests");
            sb.AppendLine("{");
            sb.AppendLine("    public sealed class TestGlueInitializer");
            sb.AppendLine("    {");
            sb.AppendLine("        private static object? _currentFixture;");
            sb.AppendLine();
            sb.AppendLine("        static TestGlueInitializer()");
            sb.AppendLine("        {");
            sb.AppendLine("            Initialize();");
            sb.AppendLine("        }");
            sb.AppendLine();
            sb.AppendLine("        public TestGlueInitializer()");
            sb.AppendLine("        {");
            sb.AppendLine("            Initialize();");
            sb.AppendLine("        }");
            sb.AppendLine();
            sb.AppendLine("        [ModuleInitializer]");
            sb.AppendLine("        public static void Initialize()");
            sb.AppendLine("        {");

            for (int f = 0; f < fixtures.Count; f++)
            {
                FixtureGenerationPlan fixture = fixtures[f];
                string fullType = string.IsNullOrEmpty(fixture.NamespaceName)
                    ? "global::" + fixture.ClassName
                    : "global::" + fixture.NamespaceName + "." + fixture.ClassName;

                for (int m = 0; m < fixture.Methods.Count; m++)
                {
                    MethodGenerationPlan mp = fixture.Methods[m];
                    string idLiteral = EscapeCSharpString(mp.Descriptor.Id);
                    string argCall = string.Join(", ", mp.ArgumentExpressions);

                    Action? emitTearDownLambda = () =>
                    {
                        if (fixture.TearDownMethods.Count == 0)
                        {
                            sb.AppendLine("                null);");
                        }
                        else
                        {
                            sb.AppendLine("                () =>");
                            sb.AppendLine("                {");
                            sb.AppendLine("                    if (_currentFixture is " + fullType + " tdInst)");
                            sb.AppendLine("                    {");
                            for (int td = 0; td < fixture.TearDownMethods.Count; td++)
                            {
                                sb.AppendLine("                        tdInst." + fixture.TearDownMethods[td] + "();");
                            }

                            sb.AppendLine("                    }");
                            sb.AppendLine("                });");
                        }
                    };

                    if (mp.Descriptor.IsUnityTest)
                    {
                        sb.AppendLine("            TestExecutionRegistry.RegisterCoroutine(");
                        sb.AppendLine("                \"" + idLiteral + "\",");
                        sb.AppendLine("                () =>");
                        sb.AppendLine("                {");
                        sb.AppendLine("                    " + fullType + " inst = new " + fullType + "();");
                        sb.AppendLine("                    _currentFixture = inst;");
                        for (int su = 0; su < fixture.SetUpMethods.Count; su++)
                        {
                            sb.AppendLine("                    inst." + fixture.SetUpMethods[su] + "();");
                        }

                        sb.AppendLine("                    return inst." + mp.Descriptor.MethodName + "(" + argCall + ");");
                        sb.AppendLine("                },");
                        emitTearDownLambda();
                    }
                    else
                    {
                        sb.AppendLine("            TestExecutionRegistry.RegisterSync(");
                        sb.AppendLine("                \"" + idLiteral + "\",");
                        sb.AppendLine("                () =>");
                        sb.AppendLine("                {");
                        sb.AppendLine("                    " + fullType + " inst = new " + fullType + "();");
                        sb.AppendLine("                    _currentFixture = inst;");
                        for (int su = 0; su < fixture.SetUpMethods.Count; su++)
                        {
                            sb.AppendLine("                    inst." + fixture.SetUpMethods[su] + "();");
                        }

                        sb.AppendLine("                    inst." + mp.Descriptor.MethodName + "(" + argCall + ");");
                        sb.AppendLine("                },");
                        emitTearDownLambda();
                    }
                }
            }

            sb.AppendLine("        }");
            sb.AppendLine("    }");
            sb.AppendLine("}");
            return sb.ToString();
        }

        private static List<string> ExtractCategories(string attributeBlock)
        {
            List<string> categories = new List<string>();
            if (string.IsNullOrEmpty(attributeBlock))
            {
                return categories;
            }

            MatchCollection matches = CategoryAttrRegex.Matches(attributeBlock);
            for (int i = 0; i < matches.Count; i++)
            {
                string cat = matches[i].Groups[1].Value.Trim();
                if (!string.IsNullOrEmpty(cat) && !categories.Contains(cat))
                {
                    categories.Add(cat);
                }
            }

            return categories;
        }

        private static string? ExtractIgnoreReason(string attributeBlock)
        {
            if (string.IsNullOrEmpty(attributeBlock))
            {
                return null;
            }

            Match m = IgnoreAttrRegex.Match(attributeBlock);
            return m.Success ? m.Groups[1].Value : null;
        }

        private static List<string> SplitArguments(string rawArgs)
        {
            List<string> args = new List<string>();
            if (string.IsNullOrWhiteSpace(rawArgs))
            {
                return args;
            }

            int depth = 0;
            bool inString = false;
            bool inChar = false;
            int start = 0;

            for (int i = 0; i < rawArgs.Length; i++)
            {
                char c = rawArgs[i];
                if (c == '\\' && (inString || inChar))
                {
                    i++;
                    continue;
                }

                if (c == '"' && !inChar)
                {
                    inString = !inString;
                    continue;
                }

                if (c == '\'' && !inString)
                {
                    inChar = !inChar;
                    continue;
                }

                if (inString || inChar)
                {
                    continue;
                }

                if (c == '(' || c == '[' || c == '{')
                {
                    depth++;
                }
                else if (c == ')' || c == ']' || c == '}')
                {
                    depth--;
                }
                else if (c == ',' && depth == 0)
                {
                    string token = rawArgs.Substring(start, i - start).Trim();
                    if (!token.Contains('=', StringComparison.Ordinal))
                    {
                        args.Add(token);
                    }

                    start = i + 1;
                }
            }

            string lastToken = rawArgs.Substring(start).Trim();
            if (!string.IsNullOrEmpty(lastToken) && !lastToken.Contains('=', StringComparison.Ordinal))
            {
                args.Add(lastToken);
            }

            return args;
        }

        private static string ExtractNamespace(string source)
        {
            Match m = NamespaceRegex.Match(source);
            return m.Success ? m.Groups[1].Value.Trim() : string.Empty;
        }

        private static string ExtractBraceBody(string source, int startIndex)
        {
            int openBrace = source.IndexOf('{', startIndex);
            if (openBrace < 0)
            {
                return string.Empty;
            }

            int depth = 0;
            for (int i = openBrace; i < source.Length; i++)
            {
                if (source[i] == '{')
                {
                    depth++;
                }
                else if (source[i] == '}')
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

        private static string StripLineAndBlockComments(string source)
        {
            StringBuilder sb = new StringBuilder(source.Length);
            bool inString = false;
            bool inChar = false;
            int i = 0;
            while (i < source.Length)
            {
                char c = source[i];
                char next = i + 1 < source.Length ? source[i + 1] : '\0';

                if (inString)
                {
                    sb.Append(c);
                    if (c == '\\' && i + 1 < source.Length)
                    {
                        i++;
                        sb.Append(source[i]);
                    }
                    else if (c == '"')
                    {
                        inString = false;
                    }

                    i++;
                    continue;
                }

                if (inChar)
                {
                    sb.Append(c);
                    if (c == '\\' && i + 1 < source.Length)
                    {
                        i++;
                        sb.Append(source[i]);
                    }
                    else if (c == '\'')
                    {
                        inChar = false;
                    }

                    i++;
                    continue;
                }

                if (c == '"')
                {
                    inString = true;
                    sb.Append(c);
                    i++;
                    continue;
                }

                if (c == '\'')
                {
                    inChar = true;
                    sb.Append(c);
                    i++;
                    continue;
                }

                if (c == '/' && next == '/')
                {
                    while (i < source.Length && source[i] != '\n')
                    {
                        i++;
                    }

                    continue;
                }

                if (c == '/' && next == '*')
                {
                    i += 2;
                    while (i + 1 < source.Length && !(source[i] == '*' && source[i + 1] == '/'))
                    {
                        if (source[i] == '\n')
                        {
                            sb.Append('\n');
                        }

                        i++;
                    }

                    i += 2;
                    continue;
                }

                sb.Append(c);
                i++;
            }

            return sb.ToString();
        }

        private static string EscapeCSharpString(string value)
        {
            return value
                .Replace("\\", "\\\\", StringComparison.Ordinal)
                .Replace("\"", "\\\"", StringComparison.Ordinal);
        }
    }

    public sealed class FixtureGenerationPlan
    {
        public FixtureGenerationPlan(
            string namespaceName,
            string className,
            IReadOnlyList<string> setUpMethods,
            IReadOnlyList<string> tearDownMethods,
            IReadOnlyList<MethodGenerationPlan> methods)
        {
            NamespaceName = namespaceName;
            ClassName = className;
            SetUpMethods = setUpMethods;
            TearDownMethods = tearDownMethods;
            Methods = methods;
        }

        public string NamespaceName { get; }

        public string ClassName { get; }

        public IReadOnlyList<string> SetUpMethods { get; }

        public IReadOnlyList<string> TearDownMethods { get; }

        public IReadOnlyList<MethodGenerationPlan> Methods { get; }
    }

    public sealed class MethodGenerationPlan
    {
        public MethodGenerationPlan(
            DiscoveredTestCaseDescriptor descriptor,
            IReadOnlyList<string> argumentExpressions)
        {
            Descriptor = descriptor;
            ArgumentExpressions = argumentExpressions;
        }

        public DiscoveredTestCaseDescriptor Descriptor { get; }

        public IReadOnlyList<string> ArgumentExpressions { get; }
    }
}
