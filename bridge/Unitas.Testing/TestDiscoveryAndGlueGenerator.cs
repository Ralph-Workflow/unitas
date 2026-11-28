using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using NUnit.Framework;
using Unitas.Scripting;

namespace Unitas.Testing
{
    public sealed class ParsedParameterValueAttribute
    {
        public ParsedParameterValueAttribute(
            bool isAutoValues,
            IReadOnlyList<string> explicitValueExpressions,
            string? sourceTypeExpression,
            string? sourceMemberName)
        {
            IsAutoValues = isAutoValues;
            ExplicitValueExpressions = explicitValueExpressions;
            SourceTypeExpression = sourceTypeExpression;
            SourceMemberName = sourceMemberName;
        }

        public bool IsAutoValues { get; }

        public IReadOnlyList<string> ExplicitValueExpressions { get; }

        public string? SourceTypeExpression { get; }

        public string? SourceMemberName { get; }
    }

    public sealed class ParsedMethodParameterPlan
    {
        public ParsedMethodParameterPlan(
            string parameterTypeExpression,
            string parameterName,
            IReadOnlyList<ParsedParameterValueAttribute> valueAttributes)
        {
            ParameterTypeExpression = parameterTypeExpression;
            ParameterName = parameterName;
            ValueAttributes = valueAttributes;
        }

        public string ParameterTypeExpression { get; }

        public string ParameterName { get; }

        public IReadOnlyList<ParsedParameterValueAttribute> ValueAttributes { get; }
    }

    public sealed class ParsedTestCaseSourceAttribute
    {
        public ParsedTestCaseSourceAttribute(
            string? sourceTypeExpression,
            string? sourceMemberName,
            string? methodParamsExpression,
            string? category)
        {
            SourceTypeExpression = sourceTypeExpression;
            SourceMemberName = sourceMemberName;
            MethodParamsExpression = methodParamsExpression;
            Category = category;
        }

        public string? SourceTypeExpression { get; }

        public string? SourceMemberName { get; }

        public string? MethodParamsExpression { get; }

        public string? Category { get; }
    }

    public sealed class DynamicSourceMethodPlan
    {
        public DynamicSourceMethodPlan(
            string key,
            string assemblyName,
            TestExecutionMode mode,
            string namespaceName,
            string className,
            string methodName,
            string returnTypeExpression,
            bool isUnityTest,
            bool isIgnored,
            string? ignoreReason,
            IReadOnlyList<string> baseCategories,
            IReadOnlyList<ParsedMethodParameterPlan> parameters,
            IReadOnlyList<ParsedTestCaseSourceAttribute> testCaseSources)
        {
            Key = key;
            AssemblyName = assemblyName;
            Mode = mode;
            NamespaceName = namespaceName;
            ClassName = className;
            MethodName = methodName;
            ReturnTypeExpression = returnTypeExpression;
            IsUnityTest = isUnityTest;
            IsIgnored = isIgnored;
            IgnoreReason = ignoreReason;
            BaseCategories = baseCategories;
            Parameters = parameters;
            TestCaseSources = testCaseSources;
        }

        public string Key { get; }

        public string AssemblyName { get; }

        public TestExecutionMode Mode { get; }

        public string NamespaceName { get; }

        public string ClassName { get; }

        public string MethodName { get; }

        public string ReturnTypeExpression { get; }

        public bool IsUnityTest { get; }

        public bool IsIgnored { get; }

        public string? IgnoreReason { get; }

        public IReadOnlyList<string> BaseCategories { get; }

        public IReadOnlyList<ParsedMethodParameterPlan> Parameters { get; }

        public IReadOnlyList<ParsedTestCaseSourceAttribute> TestCaseSources { get; }
    }

    public static class TestParameterExpansionHelper
    {
        public static void VerifyExpectedResult(object? expected, object? actual)
        {
            if (ValuesEqual(expected, actual))
            {
                return;
            }

            throw new AssertionException(
                "Expected:<" + FormatAssertionValue(expected) + "> But was:<" + FormatAssertionValue(actual) + ">");
        }

        public static IReadOnlyList<object?> ResolveAutoValues(Type parameterType)
        {
            Type effectiveType = Nullable.GetUnderlyingType(parameterType) ?? parameterType;
            bool isNullable = Nullable.GetUnderlyingType(parameterType) != null;
            List<object?> results = new List<object?>();
            if (isNullable)
            {
                results.Add(null);
            }

            if (effectiveType == typeof(bool))
            {
                results.Add(true);
                results.Add(false);
                return results;
            }

            if (effectiveType.IsEnum)
            {
                Array enumValues = Enum.GetValues(effectiveType);
                for (int i = 0; i < enumValues.Length; i++)
                {
                    results.Add(enumValues.GetValue(i));
                }

                return results;
            }

            return results;
        }

        public static IEnumerator EnumerateSequence<T>(IEnumerable<T>? sequence)
        {
            if (sequence == null)
            {
                yield break;
            }

            foreach (T item in sequence)
            {
                yield return item;
            }
        }

        public static IEnumerator EnumerateSequence(IEnumerator? enumerator)
        {
            if (enumerator == null)
            {
                yield break;
            }

            while (enumerator.MoveNext())
            {
                yield return enumerator.Current;
            }
        }

        public static IReadOnlyList<object?> FlattenValueSource<T>(IEnumerable<T>? rawSource)
        {
            List<object?> items = new List<object?>();
            if (rawSource == null)
            {
                return items;
            }

            foreach (T item in rawSource)
            {
                items.Add(item);
            }

            return items;
        }

        public static IReadOnlyList<object?> FlattenValueSource(IEnumerator? enumerator)
        {
            List<object?> items = new List<object?>();
            if (enumerator == null)
            {
                return items;
            }

            while (enumerator.MoveNext())
            {
                items.Add(enumerator.Current);
            }

            return items;
        }

        public static IReadOnlyList<object?[]> ComputeCartesianProduct(IReadOnlyList<IReadOnlyList<object?>> parameterValueSets)
        {
            List<object?[]> combinations = new List<object?[]>();
            if (parameterValueSets.Count == 0)
            {
                return combinations;
            }

            for (int i = 0; i < parameterValueSets.Count; i++)
            {
                if (parameterValueSets[i].Count == 0)
                {
                    return combinations;
                }
            }

            combinations.Add(Array.Empty<object?>());
            for (int p = 0; p < parameterValueSets.Count; p++)
            {
                IReadOnlyList<object?> values = parameterValueSets[p];
                List<object?[]> next = new List<object?[]>(combinations.Count * values.Count);
                for (int c = 0; c < combinations.Count; c++)
                {
                    object?[] prefix = combinations[c];
                    for (int v = 0; v < values.Count; v++)
                    {
                        object?[] row = new object?[prefix.Length + 1];
                        Array.Copy(prefix, row, prefix.Length);
                        row[prefix.Length] = values[v];
                        next.Add(row);
                    }
                }

                combinations = next;
            }

            return combinations;
        }

        public static T CoerceArgument<T>(object? value)
        {
            if (value == null)
            {
                return default!;
            }

            if (value is T typed)
            {
                return typed;
            }

            Type targetType = Nullable.GetUnderlyingType(typeof(T)) ?? typeof(T);
            object converted = Convert.ChangeType(value, targetType, CultureInfo.InvariantCulture);
            return (T)converted;
        }

        public static void ExpandAndRegisterSourceCases(
            string keyPrefix,
            string assemblyName,
            TestExecutionMode mode,
            string namespaceName,
            string className,
            string methodName,
            bool isUnityTest,
            bool baseIsIgnored,
            string? baseIgnoreReason,
            IReadOnlyList<string> baseCategories,
            string? sourceCategory,
            int parameterCount,
            IEnumerator? enumerator,
            Func<object?[], object?>? syncInvoke,
            Func<object?[], IEnumerator>? asyncInvoke,
            Action? tearDown)
        {
            if (enumerator == null)
            {
                return;
            }

            int index = 0;
            while (enumerator.MoveNext())
            {
                object? item = enumerator.Current;
                ExpandSingleSourceItem(
                    keyPrefix,
                    index,
                    assemblyName,
                    mode,
                    namespaceName,
                    className,
                    methodName,
                    isUnityTest,
                    baseIsIgnored,
                    baseIgnoreReason,
                    baseCategories,
                    sourceCategory,
                    parameterCount,
                    item,
                    syncInvoke,
                    asyncInvoke,
                    tearDown);
                index++;
            }
        }

        public static void ExpandAndRegisterCombinatorialCases(
            string keyPrefix,
            string assemblyName,
            TestExecutionMode mode,
            string namespaceName,
            string className,
            string methodName,
            bool isUnityTest,
            bool baseIsIgnored,
            string? baseIgnoreReason,
            IReadOnlyList<string> baseCategories,
            IReadOnlyList<IReadOnlyList<object?>> parameterValueSets,
            Func<object?[], object?>? syncInvoke,
            Func<object?[], IEnumerator>? asyncInvoke,
            Action? tearDown)
        {
            IReadOnlyList<object?[]> combinations = ComputeCartesianProduct(parameterValueSets);
            for (int i = 0; i < combinations.Count; i++)
            {
                object?[] args = combinations[i];
                RegisterDynamicExpandedCase(
                    keyPrefix,
                    i,
                    assemblyName,
                    mode,
                    namespaceName,
                    className,
                    methodName,
                    customTestName: null,
                    isUnityTest,
                    baseIsIgnored,
                    baseIgnoreReason,
                    baseCategories,
                    args,
                    hasExpectedResult: false,
                    expectedResult: null,
                    syncInvoke,
                    asyncInvoke,
                    tearDown);
            }
        }

        private static void ExpandSingleSourceItem(
            string keyPrefix,
            int index,
            string assemblyName,
            TestExecutionMode mode,
            string namespaceName,
            string className,
            string methodName,
            bool isUnityTest,
            bool baseIsIgnored,
            string? baseIgnoreReason,
            IReadOnlyList<string> baseCategories,
            string? sourceCategory,
            int parameterCount,
            object? item,
            Func<object?[], object?>? syncInvoke,
            Func<object?[], IEnumerator>? asyncInvoke,
            Action? tearDown)
        {
            List<string> categories = new List<string>(baseCategories);
            if (!string.IsNullOrEmpty(sourceCategory) && !categories.Contains(sourceCategory))
            {
                categories.Add(sourceCategory);
            }

            if (item is TestCaseData tcd)
            {
                for (int c = 0; c < tcd.Categories.Count; c++)
                {
                    if (!categories.Contains(tcd.Categories[c]))
                    {
                        categories.Add(tcd.Categories[c]);
                    }
                }

                bool ignored = baseIsIgnored || tcd.IsIgnored;
                string? ignoreReason = tcd.IsIgnored ? tcd.IgnoreReason : baseIgnoreReason;
                RegisterDynamicExpandedCase(
                    keyPrefix,
                    index,
                    assemblyName,
                    mode,
                    namespaceName,
                    className,
                    methodName,
                    tcd.TestName,
                    isUnityTest,
                    ignored,
                    ignoreReason,
                    categories,
                    tcd.Arguments,
                    tcd.HasExpectedResult,
                    tcd.ExpectedResult,
                    syncInvoke,
                    asyncInvoke,
                    tearDown);
                return;
            }

            object?[] extractedArgs = ExtractArgumentsFromItem(item, parameterCount);
            RegisterDynamicExpandedCase(
                keyPrefix,
                index,
                assemblyName,
                mode,
                namespaceName,
                className,
                methodName,
                customTestName: null,
                isUnityTest,
                baseIsIgnored,
                baseIgnoreReason,
                categories,
                extractedArgs,
                hasExpectedResult: false,
                expectedResult: null,
                syncInvoke,
                asyncInvoke,
                tearDown);
        }

        private static object?[] ExtractArgumentsFromItem(object? item, int parameterCount)
        {
            if (item == null)
            {
                return new object?[] { null };
            }

            if (item is object?[] objArray)
            {
                return objArray;
            }

            if (item is Array arr && parameterCount > 1)
            {
                object?[] copy = new object?[arr.Length];
                for (int i = 0; i < arr.Length; i++)
                {
                    copy[i] = arr.GetValue(i);
                }

                return copy;
            }

            if (item is ITuple tuple && parameterCount > 1)
            {
                object?[] tupleArgs = new object?[tuple.Length];
                for (int i = 0; i < tuple.Length; i++)
                {
                    tupleArgs[i] = tuple[i];
                }

                return tupleArgs;
            }

            return new object?[] { item };
        }

        private static void RegisterDynamicExpandedCase(
            string keyPrefix,
            int index,
            string assemblyName,
            TestExecutionMode mode,
            string namespaceName,
            string className,
            string methodName,
            string? customTestName,
            bool isUnityTest,
            bool isIgnored,
            string? ignoreReason,
            IReadOnlyList<string> categories,
            object?[] args,
            bool hasExpectedResult,
            object? expectedResult,
            Func<object?[], object?>? syncInvoke,
            Func<object?[], IEnumerator>? asyncInvoke,
            Action? tearDown)
        {
            List<string> argStrings = new List<string>(args.Length);
            for (int i = 0; i < args.Length; i++)
            {
                argStrings.Add(FormatArgumentForDisplayName(args[i]));
            }

            string displayName = !string.IsNullOrEmpty(customTestName)
                ? customTestName
                : methodName + "(" + string.Join(",", argStrings) + ")";
            string fullClassName = string.IsNullOrEmpty(namespaceName)
                ? className
                : namespaceName + "." + className;
            string fullName = fullClassName + "." + displayName;
            string id = keyPrefix + "#" + index.ToString(CultureInfo.InvariantCulture) + "::" + fullName;

            DiscoveredTestCaseDescriptor descriptor = new DiscoveredTestCaseDescriptor(
                id,
                assemblyName,
                mode,
                namespaceName,
                className,
                methodName,
                displayName,
                fullName,
                isUnityTest,
                isIgnored,
                ignoreReason,
                categories,
                argStrings);

            TestExecutionRegistry.RegisterDynamicDescriptor(descriptor);

            if (isUnityTest && asyncInvoke != null)
            {
                TestExecutionRegistry.RegisterCoroutine(
                    id,
                    () => asyncInvoke(args),
                    tearDown);
            }
            else if (syncInvoke != null)
            {
                TestExecutionRegistry.RegisterSync(
                    id,
                    () =>
                    {
                        object? actual = syncInvoke(args);
                        if (hasExpectedResult)
                        {
                            VerifyExpectedResult(expectedResult, actual);
                        }
                    },
                    tearDown);
            }
        }

        public static string FormatArgumentForDisplayName(object? arg)
        {
            if (arg == null)
            {
                return "null";
            }

            if (arg is string s)
            {
                return "\"" + s + "\"";
            }

            if (arg is char c)
            {
                return "'" + c + "'";
            }

            if (arg is bool b)
            {
                return b ? "True" : "False";
            }

            if (arg is IFormattable formattable)
            {
                return formattable.ToString(null, CultureInfo.InvariantCulture);
            }

            return arg.ToString() ?? string.Empty;
        }

        private static bool ValuesEqual(object? expected, object? actual)
        {
            if (ReferenceEquals(expected, actual))
            {
                return true;
            }

            if (expected == null || actual == null)
            {
                return false;
            }

            if (IsNumericValue(expected) && IsNumericValue(actual))
            {
                double d1 = Convert.ToDouble(expected, CultureInfo.InvariantCulture);
                double d2 = Convert.ToDouble(actual, CultureInfo.InvariantCulture);
                return Math.Abs(d1 - d2) <= 1e-9;
            }

            return Equals(expected, actual);
        }

        private static bool IsNumericValue(object? value)
        {
            return value is byte
                || value is sbyte
                || value is short
                || value is ushort
                || value is int
                || value is uint
                || value is long
                || value is ulong
                || value is float
                || value is double
                || value is decimal;
        }

        private static string FormatAssertionValue(object? value)
        {
            if (value == null)
            {
                return "null";
            }

            if (value is IFormattable formattable)
            {
                return formattable.ToString(null, CultureInfo.InvariantCulture);
            }

            return value.ToString() ?? string.Empty;
        }
    }

    public static class TestDiscoveryAndGlueGenerator
    {
        private static readonly Regex NamespaceRegex = new Regex(
            @"\bnamespace\s+([A-Za-z_][A-Za-z0-9_.]*)",
            RegexOptions.Compiled);

        private const string AttributePattern = @"\[(?:[^\[\]]|\[[^\[\]]*\])*\]";

        private static readonly Regex ClassHeaderRegex = new Regex(
            @"((?:\s*" + AttributePattern + @"\s*)*)(?:(public|internal|private|protected)\s+)?(?:(abstract|sealed|static|partial)\s+)*class\s+([A-Za-z_][A-Za-z0-9_]*)",
            RegexOptions.Compiled);

        private static readonly Regex MethodHeaderRegex = new Regex(
            @"((?:\s*" + AttributePattern + @"\s*)+)(?:(public|private|protected|internal)\s+)?(?:(virtual|override|new|sealed|static)\s+)*([A-Za-z_][A-Za-z0-9_<>,.\[\]?]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\)",
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
            @"(?<![A-Za-z0-9_])TestCase(?:Attribute)?\s*\(((?:[^()""']|""(?:\\.|[^""\\])*""|'(?:\\.|[^'\\])*'|\([^()]*\))*)\)",
            RegexOptions.Compiled);

        private static readonly Regex TestCaseSourceAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])TestCaseSource(?:Attribute)?\s*\(((?:[^()""']|""(?:\\.|[^""\\])*""|'(?:\\.|[^'\\])*'|\([^()]*\))*)\)",
            RegexOptions.Compiled);

        private static readonly Regex ValuesAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])Values(?:Attribute)?\s*(?:\(((?:[^()""']|""(?:\\.|[^""\\])*""|'(?:\\.|[^'\\])*'|\([^()]*\))*)\))?(?![A-Za-z0-9_])",
            RegexOptions.Compiled);

        private static readonly Regex ValueSourceAttrRegex = new Regex(
            @"(?<![A-Za-z0-9_])ValueSource(?:Attribute)?\s*\(((?:[^()""']|""(?:\\.|[^""\\])*""|'(?:\\.|[^'\\])*'|\([^()]*\))*)\)",
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
            HashSet<string> assemblyMemberNames = new HashSet<string>(StringComparer.Ordinal);
            List<(string Ns, List<string> Usings, string ClassAttrs, string ClassName, string ClassBody)> discoveredClasses =
                new List<(string Ns, List<string> Usings, string ClassAttrs, string ClassName, string ClassBody)>();

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
                List<string> fileUsings = ExtractUsingNamespaces(cleanSource);

                MatchCollection classMatches = ClassHeaderRegex.Matches(cleanSource);
                for (int cIdx = 0; cIdx < classMatches.Count; cIdx++)
                {
                    Match cm = classMatches[cIdx];
                    string classAttrs = cm.Groups[1].Value;
                    string modifier = cm.Groups[3].Value;
                    string className = cm.Groups[4].Value;
                    string classBody = ExtractBraceBody(cleanSource, cm.Index + cm.Length);
                    if (string.IsNullOrEmpty(classBody))
                    {
                        continue;
                    }

                    RegisterClassMemberNames(className, classBody, assemblyMemberNames);

                    if (string.Equals(modifier, "abstract", StringComparison.Ordinal)
                        || string.Equals(modifier, "static", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    discoveredClasses.Add((ns, fileUsings, classAttrs, className, classBody));
                }
            }

            for (int i = 0; i < discoveredClasses.Count; i++)
            {
                var item = discoveredClasses[i];
                DiscoverFixtureTests(
                    assembly,
                    item.Ns,
                    item.Usings,
                    assemblyMemberNames,
                    item.ClassAttrs,
                    item.ClassName,
                    item.ClassBody,
                    descriptors,
                    fixturePlans);
            }

            return descriptors;
        }

        private static void DiscoverFixtureTests(
            DiscoveredTestAssembly assembly,
            string ns,
            IReadOnlyList<string> usingNamespaces,
            HashSet<string> assemblyMemberNames,
            string classAttrs,
            string className,
            string classBody,
            List<DiscoveredTestCaseDescriptor> descriptors,
            List<FixtureGenerationPlan> fixturePlans)
        {
            List<string> classCategories = ExtractCategories(classAttrs);
            string? classIgnoreReason = ExtractIgnoreReason(classAttrs);
            HashSet<string> staticMemberNames = new HashSet<string>(assemblyMemberNames, StringComparer.Ordinal);
            foreach (string m in ExtractStaticMemberNames(classBody))
            {
                staticMemberNames.Add(m);
            }

            List<string> setUpMethods = new List<string>();
            List<string> tearDownMethods = new List<string>();
            List<MethodGenerationPlan> methodPlans = new List<MethodGenerationPlan>();
            List<DynamicSourceMethodPlan> dynamicMethodPlans = new List<DynamicSourceMethodPlan>();

            MatchCollection methodMatches = MethodHeaderRegex.Matches(classBody);
            for (int mIdx = 0; mIdx < methodMatches.Count; mIdx++)
            {
                Match mm = methodMatches[mIdx];
                string methodAttrs = mm.Groups[1].Value;
                string returnType = mm.Groups[4].Value.Trim();
                string methodName = mm.Groups[5].Value.Trim();
                string rawParamList = mm.Groups[6].Value.Trim();

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
                MatchCollection testCaseSourceMatches = TestCaseSourceAttrRegex.Matches(methodAttrs);

                if (!isUnityTest && !isTest && testCaseMatches.Count == 0 && testCaseSourceMatches.Count == 0)
                {
                    continue;
                }

                bool returnsEnumerator = returnType.EndsWith("IEnumerator", StringComparison.Ordinal);
                List<string> combinedCategories = CombineCategories(classCategories, ExtractCategories(methodAttrs));
                string? methodIgnore = ExtractIgnoreReason(methodAttrs) ?? classIgnoreReason;
                bool isIgnored = methodIgnore != null;
                string fullClassName = string.IsNullOrEmpty(ns) ? className : string.Concat(ns, ".", className);

                List<ParsedMethodParameterPlan> parameters = ParseMethodParameters(rawParamList);
                bool hasParameterValues = HasAnyParameterValueAttribute(parameters);

                if (testCaseMatches.Count > 0)
                {
                    AddStaticTestCasePlans(
                        assembly,
                        ns,
                        className,
                        fullClassName,
                        methodName,
                        isUnityTest || returnsEnumerator,
                        isIgnored,
                        methodIgnore,
                        combinedCategories,
                        testCaseMatches,
                        descriptors,
                        methodPlans);
                }

                if (testCaseSourceMatches.Count > 0 || hasParameterValues)
                {
                    List<ParsedTestCaseSourceAttribute> parsedSources = ParseTestCaseSources(testCaseSourceMatches);
                    string dynKey = assembly.Name + "::" + fullClassName + "." + methodName + "#dyn" + mIdx.ToString(CultureInfo.InvariantCulture);
                    dynamicMethodPlans.Add(new DynamicSourceMethodPlan(
                        dynKey,
                        assembly.Name,
                        assembly.Mode,
                        ns,
                        className,
                        methodName,
                        returnType,
                        isUnityTest || returnsEnumerator,
                        isIgnored,
                        methodIgnore,
                        combinedCategories,
                        parameters,
                        parsedSources));
                }
                else if (testCaseMatches.Count == 0)
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
                    methodPlans.Add(new MethodGenerationPlan(desc, Array.Empty<string>(), null));
                }
            }

            if (methodPlans.Count > 0 || dynamicMethodPlans.Count > 0)
            {
                fixturePlans.Add(new FixtureGenerationPlan(
                    ns,
                    className,
                    usingNamespaces,
                    staticMemberNames,
                    setUpMethods,
                    tearDownMethods,
                    methodPlans,
                    dynamicMethodPlans));
            }
        }

        private static void AddStaticTestCasePlans(
            DiscoveredTestAssembly assembly,
            string ns,
            string className,
            string fullClassName,
            string methodName,
            bool isCoroutineTest,
            bool baseIsIgnored,
            string? baseIgnoreReason,
            IReadOnlyList<string> baseCategories,
            MatchCollection testCaseMatches,
            List<DiscoveredTestCaseDescriptor> descriptors,
            List<MethodGenerationPlan> methodPlans)
        {
            for (int tcIdx = 0; tcIdx < testCaseMatches.Count; tcIdx++)
            {
                string rawArgs = testCaseMatches[tcIdx].Groups[1].Value.Trim();
                ParseTestCaseArguments(
                    rawArgs,
                    out List<string> argExprs,
                    out string? expectedResultExpr,
                    out string? customTestName,
                    out string? extraCategory,
                    out string? ignoreReasonOverride);

                List<string> caseCategories = new List<string>(baseCategories);
                if (!string.IsNullOrEmpty(extraCategory) && !caseCategories.Contains(extraCategory))
                {
                    caseCategories.Add(extraCategory);
                }

                bool caseIgnored = baseIsIgnored || ignoreReasonOverride != null;
                string? caseIgnoreReason = ignoreReasonOverride ?? baseIgnoreReason;
                string displayArgs = string.Join(",", argExprs);
                string displayName = !string.IsNullOrEmpty(customTestName)
                    ? customTestName
                    : methodName + "(" + displayArgs + ")";
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
                    isCoroutineTest,
                    caseIgnored,
                    caseIgnoreReason,
                    caseCategories,
                    argExprs);

                descriptors.Add(desc);
                methodPlans.Add(new MethodGenerationPlan(desc, argExprs, expectedResultExpr));
            }
        }

        public static string GenerateTestRunnerGlueSource(IReadOnlyList<FixtureGenerationPlan> fixtures)
        {
            StringBuilder sb = new StringBuilder();
            HashSet<string> allUsings = new HashSet<string>(StringComparer.Ordinal)
            {
                "System",
                "System.Collections",
                "System.Collections.Generic",
                "System.Runtime.CompilerServices",
                "NUnit.Framework",
                "Unitas.Testing"
            };

            for (int f = 0; f < fixtures.Count; f++)
            {
                for (int u = 0; u < fixtures[f].UsingNamespaces.Count; u++)
                {
                    allUsings.Add(fixtures[f].UsingNamespaces[u]);
                }

                if (!string.IsNullOrEmpty(fixtures[f].NamespaceName))
                {
                    allUsings.Add(fixtures[f].NamespaceName);
                }
            }

            foreach (string u in allUsings)
            {
                sb.AppendLine("using " + u + ";");
            }

            sb.AppendLine();
            sb.AppendLine("namespace UnitasGeneratedTests");
            sb.AppendLine("{");
            sb.AppendLine("    public sealed class TestGlueInitializer");
            sb.AppendLine("    {");
            sb.AppendLine("        private static object? _currentFixture;");
            sb.AppendLine();
            sb.AppendLine("        static TestGlueInitializer() { Initialize(); }");
            sb.AppendLine("        public TestGlueInitializer() { Initialize(); }");
            sb.AppendLine();
            sb.AppendLine("        [ModuleInitializer]");
            sb.AppendLine("        public static void Initialize()");
            sb.AppendLine("        {");

            for (int f = 0; f < fixtures.Count; f++)
            {
                EmitFixtureRegistrations(sb, fixtures[f]);
            }

            sb.AppendLine("        }");
            sb.AppendLine("    }");
            sb.AppendLine("}");
            return sb.ToString();
        }

        private static void EmitFixtureRegistrations(StringBuilder sb, FixtureGenerationPlan fixture)
        {
            string fullType = string.IsNullOrEmpty(fixture.NamespaceName)
                ? "global::" + fixture.ClassName
                : "global::" + fixture.NamespaceName + "." + fixture.ClassName;

            for (int m = 0; m < fixture.Methods.Count; m++)
            {
                EmitStaticMethodRegistration(sb, fixture, fullType, fixture.Methods[m]);
            }

            for (int d = 0; d < fixture.DynamicMethods.Count; d++)
            {
                EmitDynamicMethodRegistration(sb, fixture, fullType, fixture.DynamicMethods[d]);
            }
        }

        private static void EmitStaticMethodRegistration(
            StringBuilder sb,
            FixtureGenerationPlan fixture,
            string fullType,
            MethodGenerationPlan mp)
        {
            string idLiteral = EscapeCSharpString(mp.Descriptor.Id);
            string argCall = string.Join(", ", mp.ArgumentExpressions);

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
                EmitTearDownArgument(sb, fixture, fullType);
                sb.AppendLine(");");
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

                if (!string.IsNullOrEmpty(mp.ExpectedResultExpression))
                {
                    sb.AppendLine("                    object? __actual = inst." + mp.Descriptor.MethodName + "(" + argCall + ");");
                    sb.AppendLine("                    TestParameterExpansionHelper.VerifyExpectedResult((object?)(" + mp.ExpectedResultExpression + "), __actual);");
                }
                else
                {
                    sb.AppendLine("                    inst." + mp.Descriptor.MethodName + "(" + argCall + ");");
                }

                sb.AppendLine("                },");
                EmitTearDownArgument(sb, fixture, fullType);
                sb.AppendLine(");");
            }
        }

        private static void EmitDynamicMethodRegistration(
            StringBuilder sb,
            FixtureGenerationPlan fixture,
            string fullType,
            DynamicSourceMethodPlan dp)
        {
            string callArgs = BuildCoercedCallArgs(dp.Parameters);
            bool returnsValue = !string.Equals(dp.ReturnTypeExpression, "void", StringComparison.Ordinal) && !dp.IsUnityTest;
            string syncLambda = dp.IsUnityTest
                ? "null"
                : BuildDynamicSyncInvokeLambda(fixture, fullType, dp.MethodName, callArgs, returnsValue);
            string asyncLambda = dp.IsUnityTest
                ? BuildDynamicAsyncInvokeLambda(fixture, fullType, dp.MethodName, callArgs)
                : "null";
            string baseCatsExpr = BuildCategoriesArrayExpression(dp.BaseCategories);
            string nsLit = EscapeCSharpString(dp.NamespaceName);
            string clsLit = EscapeCSharpString(dp.ClassName);
            string mthLit = EscapeCSharpString(dp.MethodName);
            string asmLit = EscapeCSharpString(dp.AssemblyName);
            string modeExpr = dp.Mode == TestExecutionMode.EditMode
                ? "TestExecutionMode.EditMode"
                : "TestExecutionMode.PlayMode";
            string ignExpr = dp.IsIgnored ? "true" : "false";
            string ignReasonExpr = dp.IgnoreReason != null
                ? "\"" + EscapeCSharpString(dp.IgnoreReason) + "\""
                : "null";

            for (int s = 0; s < dp.TestCaseSources.Count; s++)
            {
                ParsedTestCaseSourceAttribute src = dp.TestCaseSources[s];
                string srcKeyLit = EscapeCSharpString(dp.Key + "_src" + s.ToString(CultureInfo.InvariantCulture));
                string sourceDataExpr = BuildSourceAccessExpression(
                    fullType,
                    fixture.StaticMemberNames,
                    src.SourceTypeExpression,
                    src.SourceMemberName,
                    src.MethodParamsExpression);
                string srcCatExpr = !string.IsNullOrEmpty(src.Category)
                    ? "\"" + EscapeCSharpString(src.Category) + "\""
                    : "null";

                sb.AppendLine("            TestParameterExpansionHelper.ExpandAndRegisterSourceCases(");
                sb.AppendLine("                \"" + srcKeyLit + "\",");
                sb.AppendLine("                \"" + asmLit + "\",");
                sb.AppendLine("                " + modeExpr + ",");
                sb.AppendLine("                \"" + nsLit + "\",");
                sb.AppendLine("                \"" + clsLit + "\",");
                sb.AppendLine("                \"" + mthLit + "\",");
                sb.AppendLine("                " + (dp.IsUnityTest ? "true" : "false") + ",");
                sb.AppendLine("                " + ignExpr + ",");
                sb.AppendLine("                " + ignReasonExpr + ",");
                sb.AppendLine("                " + baseCatsExpr + ",");
                sb.AppendLine("                " + srcCatExpr + ",");
                sb.AppendLine("                " + dp.Parameters.Count.ToString(CultureInfo.InvariantCulture) + ",");
                sb.AppendLine("                TestParameterExpansionHelper.EnumerateSequence(" + sourceDataExpr + "),");
                sb.AppendLine("                " + syncLambda + ",");
                sb.AppendLine("                " + asyncLambda + ",");
                EmitTearDownArgument(sb, fixture, fullType);
                sb.AppendLine(");");
            }

            if (HasAnyParameterValueAttribute(dp.Parameters))
            {
                string combKeyLit = EscapeCSharpString(dp.Key + "_comb");
                string paramSetsExpr = BuildParameterValueSetsExpression(fullType, fixture.StaticMemberNames, dp.Parameters);

                sb.AppendLine("            TestParameterExpansionHelper.ExpandAndRegisterCombinatorialCases(");
                sb.AppendLine("                \"" + combKeyLit + "\",");
                sb.AppendLine("                \"" + asmLit + "\",");
                sb.AppendLine("                " + modeExpr + ",");
                sb.AppendLine("                \"" + nsLit + "\",");
                sb.AppendLine("                \"" + clsLit + "\",");
                sb.AppendLine("                \"" + mthLit + "\",");
                sb.AppendLine("                " + (dp.IsUnityTest ? "true" : "false") + ",");
                sb.AppendLine("                " + ignExpr + ",");
                sb.AppendLine("                " + ignReasonExpr + ",");
                sb.AppendLine("                " + baseCatsExpr + ",");
                sb.AppendLine("                " + paramSetsExpr + ",");
                sb.AppendLine("                " + syncLambda + ",");
                sb.AppendLine("                " + asyncLambda + ",");
                EmitTearDownArgument(sb, fixture, fullType);
                sb.AppendLine(");");
            }
        }

        private static void EmitTearDownArgument(StringBuilder sb, FixtureGenerationPlan fixture, string fullType)
        {
            if (fixture.TearDownMethods.Count == 0)
            {
                sb.Append("                null");
                return;
            }

            sb.AppendLine("                () =>");
            sb.AppendLine("                {");
            sb.AppendLine("                    if (_currentFixture is " + fullType + " tdInst)");
            sb.AppendLine("                    {");
            for (int td = 0; td < fixture.TearDownMethods.Count; td++)
            {
                sb.AppendLine("                        tdInst." + fixture.TearDownMethods[td] + "();");
            }

            sb.AppendLine("                    }");
            sb.Append("                }");
        }

        private static string BuildDynamicSyncInvokeLambda(
            FixtureGenerationPlan fixture,
            string fullType,
            string methodName,
            string callArgs,
            bool returnsValue)
        {
            StringBuilder sb = new StringBuilder();
            sb.Append("__args => { ");
            sb.Append(fullType + " inst = new " + fullType + "(); _currentFixture = inst; ");
            for (int su = 0; su < fixture.SetUpMethods.Count; su++)
            {
                sb.Append("inst." + fixture.SetUpMethods[su] + "(); ");
            }

            if (returnsValue)
            {
                sb.Append("return (object?)inst." + methodName + "(" + callArgs + "); }");
            }
            else
            {
                sb.Append("inst." + methodName + "(" + callArgs + "); return null; }");
            }

            return sb.ToString();
        }

        private static string BuildDynamicAsyncInvokeLambda(
            FixtureGenerationPlan fixture,
            string fullType,
            string methodName,
            string callArgs)
        {
            StringBuilder sb = new StringBuilder();
            sb.Append("__args => { ");
            sb.Append(fullType + " inst = new " + fullType + "(); _currentFixture = inst; ");
            for (int su = 0; su < fixture.SetUpMethods.Count; su++)
            {
                sb.Append("inst." + fixture.SetUpMethods[su] + "(); ");
            }

            sb.Append("return inst." + methodName + "(" + callArgs + "); }");
            return sb.ToString();
        }

        private static string BuildCoercedCallArgs(IReadOnlyList<ParsedMethodParameterPlan> parameters)
        {
            if (parameters.Count == 0)
            {
                return string.Empty;
            }

            List<string> exprs = new List<string>(parameters.Count);
            for (int i = 0; i < parameters.Count; i++)
            {
                exprs.Add("TestParameterExpansionHelper.CoerceArgument<" + parameters[i].ParameterTypeExpression + ">(__args[" + i.ToString(CultureInfo.InvariantCulture) + "])");
            }

            return string.Join(", ", exprs);
        }

        private static string BuildCategoriesArrayExpression(IReadOnlyList<string> categories)
        {
            if (categories.Count == 0)
            {
                return "Array.Empty<string>()";
            }

            List<string> literals = new List<string>(categories.Count);
            for (int i = 0; i < categories.Count; i++)
            {
                literals.Add("\"" + EscapeCSharpString(categories[i]) + "\"");
            }

            return "new string[] { " + string.Join(", ", literals) + " }";
        }

        private static string BuildSourceAccessExpression(
            string fixtureFullType,
            HashSet<string> fixtureStaticMembers,
            string? sourceTypeExpression,
            string? sourceMemberName,
            string? methodParamsExpression)
        {
            string targetType = !string.IsNullOrEmpty(sourceTypeExpression)
                ? sourceTypeExpression
                : fixtureFullType;

            if (string.IsNullOrEmpty(sourceMemberName))
            {
                return "(new " + targetType + "()).GetEnumerator()";
            }

            if (!string.IsNullOrEmpty(methodParamsExpression))
            {
                List<string> callArgs = ExtractArrayInitializerElements(methodParamsExpression);
                return targetType + "." + sourceMemberName + "(" + string.Join(", ", callArgs) + ")";
            }

            string typeKey = StripTypeQualification(targetType);
            if (fixtureStaticMembers.Contains(typeKey + "::" + sourceMemberName + "()")
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains(sourceMemberName + "()")))
            {
                return targetType + "." + sourceMemberName + "()";
            }

            if (fixtureStaticMembers.Contains(typeKey + "::" + sourceMemberName)
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains(sourceMemberName)))
            {
                return targetType + "." + sourceMemberName;
            }

            if (fixtureStaticMembers.Contains("inst:" + typeKey + "::" + sourceMemberName + "()")
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains("inst:" + sourceMemberName + "()")))
            {
                return "(new " + targetType + "())." + sourceMemberName + "()";
            }

            if (fixtureStaticMembers.Contains("inst:" + typeKey + "::" + sourceMemberName)
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains("inst:" + sourceMemberName)))
            {
                return "(new " + targetType + "())." + sourceMemberName;
            }

            return targetType + "." + sourceMemberName;
        }

        private static string BuildParameterValueSetsExpression(
            string fixtureFullType,
            HashSet<string> fixtureStaticMembers,
            IReadOnlyList<ParsedMethodParameterPlan> parameters)
        {
            List<string> paramLists = new List<string>(parameters.Count);
            for (int i = 0; i < parameters.Count; i++)
            {
                ParsedMethodParameterPlan param = parameters[i];
                if (param.ValueAttributes.Count == 0)
                {
                    paramLists.Add("new object?[] { default(" + param.ParameterTypeExpression + ") }");
                    continue;
                }

                List<string> segments = new List<string>();
                for (int a = 0; a < param.ValueAttributes.Count; a++)
                {
                    ParsedParameterValueAttribute attr = param.ValueAttributes[a];
                    if (attr.IsAutoValues)
                    {
                        segments.Add("TestParameterExpansionHelper.ResolveAutoValues(typeof(" + param.ParameterTypeExpression + "))");
                    }
                    else if (!string.IsNullOrEmpty(attr.SourceMemberName))
                    {
                        string srcAccess = BuildParameterValueSourceExpression(
                            fixtureFullType,
                            fixtureStaticMembers,
                            attr.SourceTypeExpression,
                            attr.SourceMemberName);
                        segments.Add("TestParameterExpansionHelper.FlattenValueSource(" + srcAccess + ")");
                    }
                    else
                    {
                        segments.Add("new object?[] { " + string.Join(", ", attr.ExplicitValueExpressions) + " }");
                    }
                }

                if (segments.Count == 1)
                {
                    paramLists.Add(segments[0]);
                }
                else
                {
                    StringBuilder combined = new StringBuilder();
                    combined.Append("(() => { var __list = new List<object?>(); ");
                    for (int s = 0; s < segments.Count; s++)
                    {
                        combined.Append("__list.AddRange(" + segments[s] + "); ");
                    }

                    combined.Append("return (IReadOnlyList<object?>)__list; })()");
                    paramLists.Add(combined.ToString());
                }
            }

            return "new IReadOnlyList<object?>[] { " + string.Join(", ", paramLists) + " }";
        }

        private static string BuildParameterValueSourceExpression(
            string fixtureFullType,
            HashSet<string> fixtureStaticMembers,
            string? sourceTypeExpression,
            string sourceMemberName)
        {
            string targetType = !string.IsNullOrEmpty(sourceTypeExpression)
                ? sourceTypeExpression
                : fixtureFullType;

            string typeKey = StripTypeQualification(targetType);
            if (fixtureStaticMembers.Contains(typeKey + "::" + sourceMemberName + "()")
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains(sourceMemberName + "()")))
            {
                return targetType + "." + sourceMemberName + "()";
            }

            if (fixtureStaticMembers.Contains(typeKey + "::" + sourceMemberName)
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains(sourceMemberName)))
            {
                return targetType + "." + sourceMemberName;
            }

            if (fixtureStaticMembers.Contains("inst:" + typeKey + "::" + sourceMemberName + "()")
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains("inst:" + sourceMemberName + "()")))
            {
                return "(new " + targetType + "())." + sourceMemberName + "()";
            }

            if (fixtureStaticMembers.Contains("inst:" + typeKey + "::" + sourceMemberName)
                || (string.IsNullOrEmpty(sourceTypeExpression) && fixtureStaticMembers.Contains("inst:" + sourceMemberName)))
            {
                return "(new " + targetType + "())." + sourceMemberName;
            }

            return targetType + "." + sourceMemberName;
        }

        private static string StripTypeQualification(string typeExpr)
        {
            string trimmed = typeExpr.Trim();
            int lastDot = trimmed.LastIndexOf('.');
            if (lastDot >= 0)
            {
                return trimmed.Substring(lastDot + 1).Trim();
            }

            int lastColon = trimmed.LastIndexOf(':');
            if (lastColon >= 0)
            {
                return trimmed.Substring(lastColon + 1).Trim();
            }

            return trimmed;
        }

        private static List<string> CombineCategories(IReadOnlyList<string> first, IReadOnlyList<string> second)
        {
            List<string> combined = new List<string>(first);
            for (int i = 0; i < second.Count; i++)
            {
                if (!combined.Contains(second[i]))
                {
                    combined.Add(second[i]);
                }
            }

            return combined;
        }

        private static bool HasAnyParameterValueAttribute(IReadOnlyList<ParsedMethodParameterPlan> parameters)
        {
            for (int i = 0; i < parameters.Count; i++)
            {
                if (parameters[i].ValueAttributes.Count > 0)
                {
                    return true;
                }
            }

            return false;
        }

        private static List<string> ExtractUsingNamespaces(string source)
        {
            List<string> usings = new List<string>();
            MatchCollection matches = Regex.Matches(source, @"\busing\s+([A-Za-z_][A-Za-z0-9_.]*)\s*;");
            for (int i = 0; i < matches.Count; i++)
            {
                string ns = matches[i].Groups[1].Value.Trim();
                if (!string.IsNullOrEmpty(ns) && !usings.Contains(ns))
                {
                    usings.Add(ns);
                }
            }

            return usings;
        }

        private static void RegisterClassMemberNames(string className, string classBody, HashSet<string> targetSet)
        {
            foreach (string item in ExtractStaticMemberNames(classBody))
            {
                if (item.StartsWith("inst:", StringComparison.Ordinal))
                {
                    string rest = item.Remove(0, "inst:".Length);
                    targetSet.Add("inst:" + className + "::" + rest);
                }
                else
                {
                    targetSet.Add(className + "::" + item);
                }
            }
        }

        private static HashSet<string> ExtractStaticMemberNames(string classBody)
        {
            HashSet<string> members = new HashSet<string>(StringComparer.Ordinal);
            MatchCollection methodMatches = Regex.Matches(
                classBody,
                @"\b(static\s+)?[A-Za-z_][A-Za-z0-9_<>,.\[\]?]*\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)");
            for (int i = 0; i < methodMatches.Count; i++)
            {
                bool isStatic = methodMatches[i].Groups[1].Success;
                string name = methodMatches[i].Groups[2].Value.Trim();
                members.Add(isStatic ? name + "()" : "inst:" + name + "()");
            }

            MatchCollection propOrFieldMatches = Regex.Matches(
                classBody,
                @"\b(static\s+)?(?:readonly\s+)?[A-Za-z_][A-Za-z0-9_<>,.\[\]?]*\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:=|=>|\{)");
            for (int i = 0; i < propOrFieldMatches.Count; i++)
            {
                bool isStatic = propOrFieldMatches[i].Groups[1].Success;
                string name = propOrFieldMatches[i].Groups[2].Value.Trim();
                members.Add(isStatic ? name : "inst:" + name);
            }

            return members;
        }

        private static List<ParsedMethodParameterPlan> ParseMethodParameters(string rawParamList)
        {
            List<ParsedMethodParameterPlan> result = new List<ParsedMethodParameterPlan>();
            if (string.IsNullOrWhiteSpace(rawParamList))
            {
                return result;
            }

            List<string> tokens = SplitTopLevelCommaTokens(rawParamList);
            for (int i = 0; i < tokens.Count; i++)
            {
                string token = tokens[i].Trim();
                if (string.IsNullOrEmpty(token))
                {
                    continue;
                }

                ExtractParameterAttributesAndDecl(
                    token,
                    out string attrBlock,
                    out string paramType,
                    out string paramName);
                if (string.IsNullOrEmpty(paramType))
                {
                    continue;
                }

                List<ParsedParameterValueAttribute> valueAttrs = ParseParameterValueAttributes(attrBlock);
                result.Add(new ParsedMethodParameterPlan(paramType, paramName, valueAttrs));
            }

            return result;
        }

        private static void ExtractParameterAttributesAndDecl(
            string rawParam,
            out string attrBlock,
            out string paramType,
            out string paramName)
        {
            StringBuilder attrs = new StringBuilder();
            int idx = 0;
            while (idx < rawParam.Length)
            {
                while (idx < rawParam.Length && char.IsWhiteSpace(rawParam[idx]))
                {
                    idx++;
                }

                if (idx < rawParam.Length && rawParam[idx] == '[')
                {
                    int depth = 0;
                    int start = idx;
                    while (idx < rawParam.Length)
                    {
                        if (rawParam[idx] == '[')
                        {
                            depth++;
                        }
                        else if (rawParam[idx] == ']')
                        {
                            depth--;
                            if (depth == 0)
                            {
                                idx++;
                                break;
                            }
                        }

                        idx++;
                    }

                    attrs.Append(rawParam.AsSpan(start, idx - start));
                }
                else
                {
                    break;
                }
            }

            attrBlock = attrs.ToString();
            string decl = rawParam.Substring(idx).Trim();
            int eqIdx = decl.IndexOf('=', StringComparison.Ordinal);
            if (eqIdx >= 0)
            {
                decl = decl.Substring(0, eqIdx).Trim();
            }

            int lastSpace = decl.LastIndexOfAny(new[] { ' ', '\t', '\r', '\n' });
            if (lastSpace <= 0)
            {
                paramType = decl;
                paramName = "arg";
                return;
            }

            paramType = decl.Substring(0, lastSpace).Trim();
            paramName = decl.Substring(lastSpace + 1).Trim();
        }

        private static List<ParsedParameterValueAttribute> ParseParameterValueAttributes(string attrBlock)
        {
            List<ParsedParameterValueAttribute> list = new List<ParsedParameterValueAttribute>();
            if (string.IsNullOrEmpty(attrBlock))
            {
                return list;
            }

            MatchCollection valuesMatches = ValuesAttrRegex.Matches(attrBlock);
            for (int i = 0; i < valuesMatches.Count; i++)
            {
                string rawArgs = valuesMatches[i].Groups[1].Success
                    ? valuesMatches[i].Groups[1].Value.Trim()
                    : string.Empty;
                if (string.IsNullOrEmpty(rawArgs))
                {
                    list.Add(new ParsedParameterValueAttribute(true, Array.Empty<string>(), null, null));
                }
                else
                {
                    List<string> exprs = SplitTopLevelCommaTokens(rawArgs);
                    list.Add(new ParsedParameterValueAttribute(false, exprs, null, null));
                }
            }

            MatchCollection valueSourceMatches = ValueSourceAttrRegex.Matches(attrBlock);
            for (int i = 0; i < valueSourceMatches.Count; i++)
            {
                string rawArgs = valueSourceMatches[i].Groups[1].Value.Trim();
                List<string> parts = SplitTopLevelCommaTokens(rawArgs);
                if (parts.Count == 1)
                {
                    string memberName = ResolveStringOrNameofExpression(parts[0]);
                    list.Add(new ParsedParameterValueAttribute(false, Array.Empty<string>(), null, memberName));
                }
                else if (parts.Count >= 2)
                {
                    string? srcType = ResolveTypeofExpression(parts[0]);
                    string memberName = ResolveStringOrNameofExpression(parts[1]);
                    list.Add(new ParsedParameterValueAttribute(false, Array.Empty<string>(), srcType, memberName));
                }
            }

            return list;
        }

        private static List<ParsedTestCaseSourceAttribute> ParseTestCaseSources(MatchCollection matches)
        {
            List<ParsedTestCaseSourceAttribute> list = new List<ParsedTestCaseSourceAttribute>();
            for (int i = 0; i < matches.Count; i++)
            {
                string rawArgs = matches[i].Groups[1].Value.Trim();
                List<string> tokens = SplitTopLevelCommaTokens(rawArgs);
                List<string> positional = new List<string>();
                string? category = null;

                for (int t = 0; t < tokens.Count; t++)
                {
                    string tok = tokens[t].Trim();
                    if (TrySplitNamedArgument(tok, out string propName, out string propVal))
                    {
                        if (string.Equals(propName, "Category", StringComparison.Ordinal))
                        {
                            category = UnquoteStringLiteral(propVal);
                        }
                    }
                    else
                    {
                        positional.Add(tok);
                    }
                }

                if (positional.Count == 1)
                {
                    string? typeofTarget = ResolveTypeofExpression(positional[0]);
                    if (!string.IsNullOrEmpty(typeofTarget))
                    {
                        list.Add(new ParsedTestCaseSourceAttribute(typeofTarget, null, null, category));
                    }
                    else
                    {
                        string member = ResolveStringOrNameofExpression(positional[0]);
                        list.Add(new ParsedTestCaseSourceAttribute(null, member, null, category));
                    }
                }
                else if (positional.Count == 2)
                {
                    string? typeofTarget = ResolveTypeofExpression(positional[0]);
                    if (!string.IsNullOrEmpty(typeofTarget))
                    {
                        string member = ResolveStringOrNameofExpression(positional[1]);
                        list.Add(new ParsedTestCaseSourceAttribute(typeofTarget, member, null, category));
                    }
                    else
                    {
                        string member = ResolveStringOrNameofExpression(positional[0]);
                        list.Add(new ParsedTestCaseSourceAttribute(null, member, positional[1], category));
                    }
                }
                else if (positional.Count >= 3)
                {
                    string? typeofTarget = ResolveTypeofExpression(positional[0]);
                    string member = ResolveStringOrNameofExpression(positional[1]);
                    list.Add(new ParsedTestCaseSourceAttribute(typeofTarget, member, positional[2], category));
                }
            }

            return list;
        }

        private static string? ResolveTypeofExpression(string expr)
        {
            string trimmed = expr.Trim();
            if (!trimmed.StartsWith("typeof", StringComparison.Ordinal))
            {
                return null;
            }

            int open = trimmed.IndexOf('(', StringComparison.Ordinal);
            int close = trimmed.LastIndexOf(')');
            if (open >= 0 && close > open)
            {
                return trimmed.Substring(open + 1, close - open - 1).Trim();
            }

            return null;
        }

        private static string ResolveStringOrNameofExpression(string expr)
        {
            string trimmed = expr.Trim();
            if (trimmed.StartsWith("nameof", StringComparison.Ordinal))
            {
                int open = trimmed.IndexOf('(', StringComparison.Ordinal);
                int close = trimmed.LastIndexOf(')');
                if (open >= 0 && close > open)
                {
                    string inner = trimmed.Substring(open + 1, close - open - 1).Trim();
                    int lastDot = inner.LastIndexOf('.');
                    return lastDot >= 0 ? inner.Substring(lastDot + 1).Trim() : inner;
                }
            }

            return UnquoteStringLiteral(trimmed);
        }

        private static string UnquoteStringLiteral(string expr)
        {
            string trimmed = expr.Trim();
            if (trimmed.Length >= 2 && trimmed[0] == '"' && trimmed[trimmed.Length - 1] == '"')
            {
                return trimmed.Substring(1, trimmed.Length - 2);
            }

            return trimmed;
        }

        private static List<string> ExtractArrayInitializerElements(string expr)
        {
            int openBrace = expr.IndexOf('{', StringComparison.Ordinal);
            int closeBrace = expr.LastIndexOf('}');
            if (openBrace >= 0 && closeBrace > openBrace)
            {
                string inner = expr.Substring(openBrace + 1, closeBrace - openBrace - 1).Trim();
                return SplitTopLevelCommaTokens(inner);
            }

            return new List<string> { expr };
        }

        private static void ParseTestCaseArguments(
            string rawArgs,
            out List<string> positionalArgs,
            out string? expectedResultExpression,
            out string? customTestName,
            out string? category,
            out string? ignoreReason)
        {
            positionalArgs = new List<string>();
            expectedResultExpression = null;
            customTestName = null;
            category = null;
            ignoreReason = null;

            List<string> tokens = SplitTopLevelCommaTokens(rawArgs);
            for (int i = 0; i < tokens.Count; i++)
            {
                string token = tokens[i].Trim();
                if (string.IsNullOrEmpty(token))
                {
                    continue;
                }

                if (TrySplitNamedArgument(token, out string propName, out string propVal))
                {
                    if (string.Equals(propName, "ExpectedResult", StringComparison.Ordinal))
                    {
                        expectedResultExpression = propVal;
                    }
                    else if (string.Equals(propName, "TestName", StringComparison.Ordinal))
                    {
                        customTestName = UnquoteStringLiteral(propVal);
                    }
                    else if (string.Equals(propName, "Category", StringComparison.Ordinal))
                    {
                        category = UnquoteStringLiteral(propVal);
                    }
                    else if (string.Equals(propName, "Ignore", StringComparison.Ordinal)
                        || string.Equals(propName, "IgnoreReason", StringComparison.Ordinal))
                    {
                        ignoreReason = UnquoteStringLiteral(propVal);
                    }
                }
                else
                {
                    positionalArgs.Add(token);
                }
            }
        }

        private static bool TrySplitNamedArgument(string token, out string name, out string value)
        {
            name = string.Empty;
            value = string.Empty;
            int depth = 0;
            bool inString = false;
            bool inChar = false;

            for (int i = 0; i < token.Length; i++)
            {
                char c = token[i];
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
                else if (c == '=' && depth == 0)
                {
                    char prev = i > 0 ? token[i - 1] : '\0';
                    char next = i + 1 < token.Length ? token[i + 1] : '\0';
                    if (prev != '=' && prev != '!' && prev != '<' && prev != '>' && next != '=')
                    {
                        name = token.Substring(0, i).Trim();
                        value = token.Substring(i + 1).Trim();
                        return !string.IsNullOrEmpty(name);
                    }
                }
            }

            return false;
        }

        private static List<string> SplitTopLevelCommaTokens(string rawArgs)
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
                    if (!string.IsNullOrEmpty(token))
                    {
                        args.Add(token);
                    }

                    start = i + 1;
                }
            }

            string lastToken = rawArgs.Substring(start).Trim();
            if (!string.IsNullOrEmpty(lastToken))
            {
                args.Add(lastToken);
            }

            return args;
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
            IReadOnlyList<string> usingNamespaces,
            HashSet<string> staticMemberNames,
            IReadOnlyList<string> setUpMethods,
            IReadOnlyList<string> tearDownMethods,
            IReadOnlyList<MethodGenerationPlan> methods,
            IReadOnlyList<DynamicSourceMethodPlan> dynamicMethods)
        {
            NamespaceName = namespaceName;
            ClassName = className;
            UsingNamespaces = usingNamespaces;
            StaticMemberNames = staticMemberNames;
            SetUpMethods = setUpMethods;
            TearDownMethods = tearDownMethods;
            Methods = methods;
            DynamicMethods = dynamicMethods;
        }

        public string NamespaceName { get; }

        public string ClassName { get; }

        public IReadOnlyList<string> UsingNamespaces { get; }

        public HashSet<string> StaticMemberNames { get; }

        public IReadOnlyList<string> SetUpMethods { get; }

        public IReadOnlyList<string> TearDownMethods { get; }

        public IReadOnlyList<MethodGenerationPlan> Methods { get; }

        public IReadOnlyList<DynamicSourceMethodPlan> DynamicMethods { get; }
    }

    public sealed class MethodGenerationPlan
    {
        public MethodGenerationPlan(
            DiscoveredTestCaseDescriptor descriptor,
            IReadOnlyList<string> argumentExpressions,
            string? expectedResultExpression = null)
        {
            Descriptor = descriptor;
            ArgumentExpressions = argumentExpressions;
            ExpectedResultExpression = expectedResultExpression;
        }

        public DiscoveredTestCaseDescriptor Descriptor { get; }

        public IReadOnlyList<string> ArgumentExpressions { get; }

        public string? ExpectedResultExpression { get; }
    }
}
