using System;
using System.Collections;
using System.Collections.Generic;
using System.Threading;
using Unitas.Scripting;

namespace Unitas.Testing
{
    public enum TestModeFilter
    {
        All = 0,
        EditMode = 1,
        PlayMode = 2
    }

    public enum TestExecutionMode
    {
        EditMode = 0,
        PlayMode = 1
    }

    public enum TestOutcome
    {
        Passed = 0,
        Failed = 1,
        Skipped = 2,
        Inconclusive = 3,
        NotRun = 4
    }

    public sealed class TestRunOptions
    {
        public TestModeFilter Mode { get; set; } = TestModeFilter.All;

        public string? CategoryFilter { get; set; }

        public string? TestNameFilter { get; set; }

        public string? AssemblyFilter { get; set; }

        public IReadOnlyList<string> AssemblyFilters { get; set; } = Array.Empty<string>();

        public string? ResultsXmlPath { get; set; }

        public string? OutputDirectory { get; set; }

        public bool SkipCompilation { get; set; }

        public CancellationToken CancellationToken { get; set; } = CancellationToken.None;

        public int MaxTestsToRun { get; set; }

        public Action<TestCaseResult>? OnTestCompleted { get; set; }

        public int MaxUnityTestFrames { get; set; } = 1000;

        public double FrameDeltaSeconds { get; set; } = 0.02;
    }

    public sealed class DiscoveredTestAssembly
    {
        public DiscoveredTestAssembly(
            string name,
            string asmdefPath,
            TestExecutionMode mode,
            IReadOnlyList<string> sourceFiles,
            bool canRun,
            string? unrunnableReason)
        {
            Name = name ?? string.Empty;
            AsmdefPath = asmdefPath ?? string.Empty;
            Mode = mode;
            SourceFiles = sourceFiles ?? Array.Empty<string>();
            CanRun = canRun;
            UnrunnableReason = unrunnableReason;
        }

        public string Name { get; }

        public string AsmdefPath { get; }

        public TestExecutionMode Mode { get; }

        public IReadOnlyList<string> SourceFiles { get; }

        public bool CanRun { get; }

        public string? UnrunnableReason { get; }
    }

    public sealed class DiscoveredTestCaseDescriptor
    {
        public DiscoveredTestCaseDescriptor(
            string id,
            string assemblyName,
            TestExecutionMode mode,
            string namespaceName,
            string className,
            string methodName,
            string displayName,
            string fullName,
            bool isUnityTest,
            bool isIgnored,
            string? ignoreReason,
            IReadOnlyList<string> categories,
            IReadOnlyList<string> parameterExpressions)
        {
            Id = id ?? string.Empty;
            AssemblyName = assemblyName ?? string.Empty;
            Mode = mode;
            NamespaceName = namespaceName ?? string.Empty;
            ClassName = className ?? string.Empty;
            MethodName = methodName ?? string.Empty;
            DisplayName = displayName ?? string.Empty;
            FullName = fullName ?? string.Empty;
            IsUnityTest = isUnityTest;
            IsIgnored = isIgnored;
            IgnoreReason = ignoreReason;
            Categories = categories ?? Array.Empty<string>();
            ParameterExpressions = parameterExpressions ?? Array.Empty<string>();
        }

        public string Id { get; }

        public string AssemblyName { get; }

        public TestExecutionMode Mode { get; }

        public string NamespaceName { get; }

        public string ClassName { get; }

        public string MethodName { get; }

        public string DisplayName { get; }

        public string FullName { get; }

        public bool IsUnityTest { get; }

        public bool IsIgnored { get; }

        public string? IgnoreReason { get; }

        public IReadOnlyList<string> Categories { get; }

        public IReadOnlyList<string> ParameterExpressions { get; }

        public string FullClassName => string.IsNullOrEmpty(NamespaceName)
            ? ClassName
            : string.Concat(NamespaceName, ".", ClassName);
    }

    public sealed class RegisteredTestExecutionBinding
    {
        public RegisteredTestExecutionBinding(
            string id,
            Action? syncExecute,
            Func<IEnumerator>? asyncExecute,
            Action? tearDownExecute)
        {
            Id = id ?? string.Empty;
            SyncExecute = syncExecute;
            AsyncExecute = asyncExecute;
            TearDownExecute = tearDownExecute;
        }

        public string Id { get; }

        public Action? SyncExecute { get; }

        public Func<IEnumerator>? AsyncExecute { get; }

        public Action? TearDownExecute { get; }
    }

    public static class TestExecutionRegistry
    {
        private static readonly Dictionary<string, RegisteredTestExecutionBinding> Bindings =
            new Dictionary<string, RegisteredTestExecutionBinding>(StringComparer.Ordinal);

        private static readonly List<DiscoveredTestCaseDescriptor> DynamicDescriptors =
            new List<DiscoveredTestCaseDescriptor>();

        public static IReadOnlyList<DiscoveredTestCaseDescriptor> RegisteredDynamicDescriptors => DynamicDescriptors;

        public static void Clear()
        {
            Bindings.Clear();
            DynamicDescriptors.Clear();
        }

        public static void RegisterDynamicDescriptor(DiscoveredTestCaseDescriptor descriptor)
        {
            ArgumentNullException.ThrowIfNull(descriptor);
            for (int i = 0; i < DynamicDescriptors.Count; i++)
            {
                if (string.Equals(DynamicDescriptors[i].Id, descriptor.Id, StringComparison.Ordinal))
                {
                    DynamicDescriptors[i] = descriptor;
                    return;
                }
            }

            DynamicDescriptors.Add(descriptor);
        }

        public static void RegisterSync(string id, Action execute, Action? tearDown = null)
        {
            if (string.IsNullOrEmpty(id))
            {
                throw new ArgumentException("Test id must be non-empty.", nameof(id));
            }

            ArgumentNullException.ThrowIfNull(execute);

            Bindings[id] = new RegisteredTestExecutionBinding(id, execute, null, tearDown);
        }

        public static void RegisterCoroutine(string id, Func<IEnumerator> execute, Action? tearDown = null)
        {
            if (string.IsNullOrEmpty(id))
            {
                throw new ArgumentException("Test id must be non-empty.", nameof(id));
            }

            ArgumentNullException.ThrowIfNull(execute);

            Bindings[id] = new RegisteredTestExecutionBinding(id, null, execute, tearDown);
        }

        public static bool TryGet(string id, out RegisteredTestExecutionBinding? binding)
        {
            return Bindings.TryGetValue(id, out binding);
        }
    }

    public sealed class TestCaseResult
    {
        public TestCaseResult(
            DiscoveredTestCaseDescriptor descriptor,
            TestOutcome outcome,
            double durationSeconds,
            string? message,
            string? stackTrace,
            IReadOnlyList<string> capturedLogs)
        {
            Descriptor = descriptor ?? throw new ArgumentNullException(nameof(descriptor));
            Outcome = outcome;
            DurationSeconds = durationSeconds;
            Message = message;
            StackTrace = stackTrace;
            CapturedLogs = capturedLogs ?? Array.Empty<string>();
        }

        public DiscoveredTestCaseDescriptor Descriptor { get; }

        public TestOutcome Outcome { get; }

        public double DurationSeconds { get; }

        public string? Message { get; }

        public string? StackTrace { get; }

        public IReadOnlyList<string> CapturedLogs { get; }
    }

    public sealed class TestRunResult
    {
        public TestRunResult(
            bool compileSucceeded,
            bool wasCancelled,
            int selectedTestCaseCount,
            int completedCount,
            int passedCount,
            int failedCount,
            int skippedCount,
            int inconclusiveCount,
            int notRunCount,
            double durationSeconds,
            string nunitXml,
            string? resultsXmlPath,
            IReadOnlyList<DiscoveredTestAssembly> assemblies,
            IReadOnlyList<TestCaseResult> results,
            IReadOnlyList<ScriptDiagnostic> diagnostics)
        {
            CompileSucceeded = compileSucceeded;
            WasCancelled = wasCancelled;
            SelectedTestCaseCount = selectedTestCaseCount;
            CompletedCount = completedCount;
            PassedCount = passedCount;
            FailedCount = failedCount;
            SkippedCount = skippedCount;
            InconclusiveCount = inconclusiveCount;
            NotRunCount = notRunCount;
            DurationSeconds = durationSeconds;
            NunitXml = nunitXml ?? string.Empty;
            ResultsXmlPath = resultsXmlPath;
            Assemblies = assemblies ?? Array.Empty<DiscoveredTestAssembly>();
            Results = results ?? Array.Empty<TestCaseResult>();
            Diagnostics = diagnostics ?? Array.Empty<ScriptDiagnostic>();
        }

        public bool CompileSucceeded { get; }

        public bool WasCancelled { get; }

        public int SelectedTestCaseCount { get; }

        public int CompletedCount { get; }

        public int PassedCount { get; }

        public int FailedCount { get; }

        public int SkippedCount { get; }

        public int InconclusiveCount { get; }

        public int NotRunCount { get; }

        public double DurationSeconds { get; }

        public string NunitXml { get; }

        public string? ResultsXmlPath { get; }

        public IReadOnlyList<DiscoveredTestAssembly> Assemblies { get; }

        public IReadOnlyList<TestCaseResult> Results { get; }

        public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; }

        public bool Succeeded => CompileSucceeded && !WasCancelled && FailedCount == 0 && SelectedTestCaseCount == CompletedCount;

        public TestCaseResult? FindTest(string fullOrDisplayName)
        {
            for (int index = 0; index < Results.Count; index++)
            {
                TestCaseResult r = Results[index];
                if (string.Equals(r.Descriptor.FullName, fullOrDisplayName, StringComparison.Ordinal)
                    || string.Equals(r.Descriptor.DisplayName, fullOrDisplayName, StringComparison.Ordinal)
                    || string.Equals(r.Descriptor.MethodName, fullOrDisplayName, StringComparison.Ordinal))
                {
                    return r;
                }
            }

            return null;
        }
    }
}
