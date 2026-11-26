using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text;
using System.Text.RegularExpressions;
using NUnit.Framework;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;

namespace Unitas.Testing
{
    public static class TestRunner
    {
        private static readonly Regex FileAssemblyPrebuildRegex = new Regex(
            @"\[\s*assembly\s*:\s*(?:UnityEngine\.TestTools\.)?PrebuildSetup(?:Attribute)?\s*\(([^]]+)\)\s*\]",
            RegexOptions.Compiled);

        private static readonly Regex FileAssemblyPostBuildRegex = new Regex(
            @"\[\s*assembly\s*:\s*(?:UnityEngine\.TestTools\.)?PostBuildCleanup(?:Attribute)?\s*\(([^]]+)\)\s*\]",
            RegexOptions.Compiled);

        private static readonly Regex NamespaceSourceRegex = new Regex(
            @"\bnamespace\s+([A-Za-z_][A-Za-z0-9_.]*)",
            RegexOptions.Compiled);

        private const string AttributePattern = @"\[(?:[^\[\]]|\[[^\[\]]*\])*\]";

        private static readonly Regex ClassSourceRegex = new Regex(
            @"((?:\s*" + AttributePattern + @"\s*)*)(?:(public|internal|private|protected)\s+)?(?:(abstract|sealed|static|partial)\s+)*class\s+([A-Za-z_][A-Za-z0-9_]*)",
            RegexOptions.Compiled);

        private static readonly Regex MethodSourceRegex = new Regex(
            @"((?:\s*" + AttributePattern + @"\s*)+)(?:(public|private|protected|internal)\s+)?(?:(virtual|override|new|sealed|static)\s+)*(void|IEnumerator|System\.Collections\.IEnumerator)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
            RegexOptions.Compiled);

        private static readonly Regex RetryAttrRegex = new Regex(
            @"\bRetry(?:Attribute)?\s*\(\s*(\d+)\s*\)",
            RegexOptions.Compiled);

        private static readonly Regex RepeatAttrRegex = new Regex(
            @"\bRepeat(?:Attribute)?\s*\(\s*(\d+)\s*\)",
            RegexOptions.Compiled);

        private static readonly Regex UnityPlatformAttrRegex = new Regex(
            @"\bUnityPlatform(?:Attribute)?\s*(?:\(((?:[^()]|\([^()]*\))*)\))?",
            RegexOptions.Compiled);

        private static readonly Regex PrebuildSetupAttrRegex = new Regex(
            @"(?<!assembly\s*:\s*)\bPrebuildSetup(?:Attribute)?\s*\(([^]]+)\)",
            RegexOptions.Compiled);

        private static readonly Regex PostBuildCleanupAttrRegex = new Regex(
            @"(?<!assembly\s*:\s*)\bPostBuildCleanup(?:Attribute)?\s*\(([^]]+)\)",
            RegexOptions.Compiled);

        private static readonly Regex RuntimePlatformTokenRegex = new Regex(
            @"(?:RuntimePlatform\.)?([A-Za-z_][A-Za-z0-9_]*)",
            RegexOptions.Compiled);

        public static TestRunResult RunProject(string projectRoot, TestRunOptions? options = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            TestRunOptions effectiveOptions = options ?? new TestRunOptions();
            Stopwatch runStopwatch = Stopwatch.StartNew();

            // A run starts from a clean log-assertion state: leftovers from an
            // earlier host test would otherwise be attributed to the first case.
            LogAssert.Reset();
            LogRecorder.Clear();

            string outputDir = !string.IsNullOrEmpty(effectiveOptions.OutputDirectory)
                ? Path.GetFullPath(effectiveOptions.OutputDirectory)
                : Path.Combine(fullProjectRoot, "Library", ScriptCompilation.DefaultOutputFolderName);

            List<DiscoveredTestAssembly> testAssemblies = TestDiscoveryAndGlueGenerator.DiscoverTestAssemblies(
                fullProjectRoot,
                out Dictionary<string, List<string>> _,
                out List<ScriptDiagnostic> discoveryDiagnostics);

            if (effectiveOptions.SkipCompilation
                && LastCompilationCache != null
                && string.Equals(LastCompilationCache.ProjectRoot, fullProjectRoot, StringComparison.Ordinal)
                && string.Equals(LastCompilationCache.OutputDir, outputDir, StringComparison.Ordinal))
            {
                return ExecuteCompiledTestRun(
                    fullProjectRoot,
                    outputDir,
                    effectiveOptions,
                    runStopwatch,
                    testAssemblies,
                    LastCompilationCache.AllDescriptors,
                    LastCompilationCache.MetadataByMethodKey,
                    LastCompilationCache.AllDiagnostics);
            }

            ScriptCompilationOptions compileOptions = new ScriptCompilationOptions
            {
                IsEditor = true,
                OutputDirectory = outputDir,
                EmitGeneratedGlue = true,
                AutoLoadGeneratedGlue = false
            };
            compileOptions.AdditionalDefines.Add("UNITY_INCLUDE_TESTS");

            string nunitShimDll = typeof(Assert).Assembly.Location;
            if (!string.IsNullOrEmpty(nunitShimDll) && File.Exists(nunitShimDll))
            {
                compileOptions.AdditionalReferencePaths.Add(nunitShimDll);
            }

            string testingRunnerDll = typeof(TestExecutionRegistry).Assembly.Location;
            if (!string.IsNullOrEmpty(testingRunnerDll) && File.Exists(testingRunnerDll))
            {
                compileOptions.AdditionalReferencePaths.Add(testingRunnerDll);
            }

            ScriptCompilationResult compileResult = ScriptCompilation.CompileProject(fullProjectRoot, compileOptions);
            List<ScriptDiagnostic> allDiagnostics = new List<ScriptDiagnostic>(discoveryDiagnostics);
            for (int i = 0; i < compileResult.Diagnostics.Count; i++)
            {
                allDiagnostics.Add(compileResult.Diagnostics[i]);
            }

            if (!compileResult.Succeeded)
            {
                return BuildCompileFailureResult(
                    fullProjectRoot,
                    effectiveOptions,
                    runStopwatch.Elapsed.TotalSeconds,
                    testAssemblies,
                    allDiagnostics);
            }

            HashSet<string> baseDefines = DefinesAndResponseParser.BuildBaseDefines(compileOptions);
            List<DiscoveredTestCaseDescriptor> allDescriptors = new List<DiscoveredTestCaseDescriptor>();
            List<FixtureGenerationPlan> allFixturePlans = new List<FixtureGenerationPlan>();
            Dictionary<string, SupplementalTestMetadata> metadataByMethodKey =
                new Dictionary<string, SupplementalTestMetadata>(StringComparer.Ordinal);
            Dictionary<string, LifecycleHooksMetadata> lifecycleByClassKey =
                new Dictionary<string, LifecycleHooksMetadata>(StringComparer.Ordinal);
            List<string> assemblyPrebuildTypes = new List<string>();
            List<string> assemblyPostBuildTypes = new List<string>();

            for (int i = 0; i < testAssemblies.Count; i++)
            {
                DiscoveredTestAssembly testAsm = testAssemblies[i];
                CompiledAssemblyRecord? compiledRecord = compileResult.FindAssembly(testAsm.Name);
                HashSet<string> asmDefines = compiledRecord != null
                    ? new HashSet<string>(compiledRecord.Defines, StringComparer.Ordinal)
                    : new HashSet<string>(baseDefines, StringComparer.Ordinal);

                List<DiscoveredTestCaseDescriptor> asmDescriptors = TestDiscoveryAndGlueGenerator.DiscoverTestsInAssembly(
                    testAsm,
                    asmDefines,
                    out List<FixtureGenerationPlan> asmPlans);

                allDescriptors.AddRange(asmDescriptors);
                allFixturePlans.AddRange(asmPlans);

                ScanSupplementalMetadataInAssembly(
                    testAsm,
                    asmDefines,
                    metadataByMethodKey,
                    lifecycleByClassKey,
                    assemblyPrebuildTypes,
                    assemblyPostBuildTypes);
            }

            AssemblyLoadContext testLoadContext = new AssemblyLoadContext(
                "UnitasTestRun_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture),
                isCollectible: true);
            testLoadContext.Resolving += (alc, assemblyName) =>
            {
                string simpleName = assemblyName.Name ?? string.Empty;
                foreach (var existing in AssemblyLoadContext.Default.Assemblies)
                {
                    if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                    {
                        return existing;
                    }
                }

                string candidatePath = Path.Combine(outputDir, simpleName + ".dll");
                if (File.Exists(candidatePath))
                {
                    using FileStream candidateStream = File.OpenRead(candidatePath);
                    return alc.LoadFromStream(candidateStream);
                }

                return null;
            };

            for (int i = 0; i < compileResult.Assemblies.Count; i++)
            {
                CompiledAssemblyRecord asm = compileResult.Assemblies[i];
                if (!asm.IsExcluded && File.Exists(asm.OutputPath))
                {
                    using FileStream asmStream = File.OpenRead(asm.OutputPath);
                    testLoadContext.LoadFromStream(asmStream);
                }
            }

            if (!string.IsNullOrEmpty(compileResult.GeneratedGlueAssemblyPath)
                && File.Exists(compileResult.GeneratedGlueAssemblyPath))
            {
                using FileStream glueStream = File.OpenRead(compileResult.GeneratedGlueAssemblyPath);
                var glueAsm = testLoadContext.LoadFromStream(glueStream);
                RuntimeHelpers.RunModuleConstructor(glueAsm.ManifestModule.ModuleHandle);
                glueAsm.CreateInstance("UnitasGenerated.GlueInitializer");
                glueAsm.CreateInstance("UnitasGeneratedGlue");
            }

            if (allFixturePlans.Count > 0)
            {
                TestExecutionRegistry.Clear();
                if (!CompileAndLoadTestGlue(
                    outputDir,
                    compileResult,
                    compileOptions,
                    allFixturePlans,
                    metadataByMethodKey,
                    lifecycleByClassKey,
                    assemblyPrebuildTypes,
                    assemblyPostBuildTypes,
                    allDiagnostics,
                    testLoadContext))
                {
                    return BuildCompileFailureResult(
                        fullProjectRoot,
                        effectiveOptions,
                        runStopwatch.Elapsed.TotalSeconds,
                        testAssemblies,
                        allDiagnostics);
                }
            }

            LastCompilationCache = new CachedTestCompilation(
                fullProjectRoot,
                outputDir,
                allDescriptors,
                metadataByMethodKey,
                allDiagnostics);

            return ExecuteCompiledTestRun(
                fullProjectRoot,
                outputDir,
                effectiveOptions,
                runStopwatch,
                testAssemblies,
                allDescriptors,
                metadataByMethodKey,
                allDiagnostics);
        }

        private sealed class CachedTestCompilation
        {
            public CachedTestCompilation(
                string projectRoot,
                string outputDir,
                List<DiscoveredTestCaseDescriptor> allDescriptors,
                Dictionary<string, SupplementalTestMetadata> metadataByMethodKey,
                List<ScriptDiagnostic> allDiagnostics)
            {
                ProjectRoot = projectRoot;
                OutputDir = outputDir;
                AllDescriptors = allDescriptors;
                MetadataByMethodKey = metadataByMethodKey;
                AllDiagnostics = allDiagnostics;
            }

            public string ProjectRoot { get; }

            public string OutputDir { get; }

            public List<DiscoveredTestCaseDescriptor> AllDescriptors { get; }

            public Dictionary<string, SupplementalTestMetadata> MetadataByMethodKey { get; }

            public List<ScriptDiagnostic> AllDiagnostics { get; }
        }

        private static CachedTestCompilation? LastCompilationCache;

        private static TestRunResult ExecuteCompiledTestRun(
            string fullProjectRoot,
            string outputDir,
            TestRunOptions effectiveOptions,
            Stopwatch runStopwatch,
            List<DiscoveredTestAssembly> testAssemblies,
            List<DiscoveredTestCaseDescriptor> allDescriptors,
            Dictionary<string, SupplementalTestMetadata> metadataByMethodKey,
            List<ScriptDiagnostic> allDiagnostics)
        {
            _ = outputDir;
            List<DiscoveredTestCaseDescriptor> selectedDescriptors = FilterDescriptors(allDescriptors, effectiveOptions);

            List<TestCaseResult> results = new List<TestCaseResult>();
            bool wasCancelled = false;
            int completedCount = 0;

            string previousDataPath = Application.Host?.DataPath ?? string.Empty;
            EditTimeApplicationHost editTimeHost = new EditTimeApplicationHost(
                Path.Combine(fullProjectRoot, "Assets"),
                previousDataPath);
            IApplicationHost? savedAppHost = Application.Host;
            if (savedAppHost == null)
            {
                Application.Host = editTimeHost;
            }

            HashSet<string> executedPrebuildScopes = new HashSet<string>(StringComparer.Ordinal);
            List<string> pendingPostBuildScopes = new List<string>();

            try
            {
                ExecuteRegisteredHookIfPresent("__lifecycle_prebuild_assembly__");
                pendingPostBuildScopes.Add("__lifecycle_postbuild_assembly__");

                for (int i = 0; i < selectedDescriptors.Count; i++)
                {
                    if (effectiveOptions.CancellationToken.IsCancellationRequested
                        || (effectiveOptions.MaxTestsToRun > 0 && completedCount >= effectiveOptions.MaxTestsToRun))
                    {
                        wasCancelled = true;
                        break;
                    }

                    DiscoveredTestCaseDescriptor descriptor = selectedDescriptors[i];
                    string classLifecycleScope = descriptor.AssemblyName + "::" + descriptor.FullClassName;
                    if (executedPrebuildScopes.Add(classLifecycleScope))
                    {
                        ExecuteRegisteredHookIfPresent("__lifecycle_prebuild_class__::" + classLifecycleScope);
                        pendingPostBuildScopes.Add("__lifecycle_postbuild_class__::" + classLifecycleScope);
                    }

                    string methodKey = BuildMethodMetadataKey(descriptor.AssemblyName, descriptor.FullClassName, descriptor.MethodName);
                    metadataByMethodKey.TryGetValue(methodKey, out SupplementalTestMetadata? methodMeta);

                    ExecuteRegisteredHookIfPresent("__lifecycle_prebuild_method__::" + methodKey);
                    TestCaseResult caseResult;
                    try
                    {
                        caseResult = ExecuteSingleTestCase(descriptor, effectiveOptions, methodMeta, fullProjectRoot);
                    }
                    finally
                    {
                        ExecuteRegisteredHookIfPresent("__lifecycle_postbuild_method__::" + methodKey);
                    }

                    results.Add(caseResult);
                    completedCount++;
                    effectiveOptions.OnTestCompleted?.Invoke(caseResult);
                }
            }
            finally
            {
                for (int p = pendingPostBuildScopes.Count - 1; p >= 0; p--)
                {
                    ExecuteRegisteredHookIfPresent(pendingPostBuildScopes[p]);
                }

                if (ReferenceEquals(Application.Host, editTimeHost))
                {
                    Application.Host = savedAppHost;
                }
            }

            if (!wasCancelled && effectiveOptions.CancellationToken.IsCancellationRequested)
            {
                wasCancelled = true;
            }

            int passed = 0;
            int failed = 0;
            int skipped = 0;
            int inconclusive = 0;
            for (int i = 0; i < results.Count; i++)
            {
                switch (results[i].Outcome)
                {
                    case TestOutcome.Passed:
                        passed++;
                        break;
                    case TestOutcome.Failed:
                        failed++;
                        break;
                    case TestOutcome.Skipped:
                        skipped++;
                        break;
                    case TestOutcome.Inconclusive:
                        inconclusive++;
                        break;
                }
            }

            int notRun = Math.Max(0, selectedDescriptors.Count - completedCount);
            double totalDuration = runStopwatch.Elapsed.TotalSeconds;

            List<string> suiteFailures = new List<string>();
            if (wasCancelled)
            {
                suiteFailures.Add("Test run was cancelled after " + completedCount.ToString(CultureInfo.InvariantCulture) + " of " + selectedDescriptors.Count.ToString(CultureInfo.InvariantCulture) + " test(s).");
            }

            string xml = NunitXmlWriter.GenerateXml(
                fullProjectRoot,
                selectedDescriptors.Count,
                wasCancelled,
                true,
                totalDuration,
                results,
                suiteFailures);

            if (!string.IsNullOrEmpty(effectiveOptions.ResultsXmlPath))
            {
                string fullXmlPath = Path.GetFullPath(effectiveOptions.ResultsXmlPath);
                string? dir = Path.GetDirectoryName(fullXmlPath);
                if (!string.IsNullOrEmpty(dir))
                {
                    Directory.CreateDirectory(dir);
                }

                File.WriteAllText(fullXmlPath, xml, Encoding.UTF8);
            }

            return new TestRunResult(
                true,
                wasCancelled,
                selectedDescriptors.Count,
                completedCount,
                passed,
                failed,
                skipped,
                inconclusive,
                notRun,
                totalDuration,
                xml,
                effectiveOptions.ResultsXmlPath,
                testAssemblies,
                results,
                allDiagnostics);
        }

        private static void ExecuteRegisteredHookIfPresent(string hookId)
        {
            if (TestExecutionRegistry.TryGet(hookId, out RegisteredTestExecutionBinding? hookBinding)
                && hookBinding?.SyncExecute != null)
            {
                hookBinding.SyncExecute();
            }
        }

        private static TestCaseResult ExecuteSingleTestCase(
            DiscoveredTestCaseDescriptor descriptor,
            TestRunOptions options,
            SupplementalTestMetadata? methodMeta,
            string fullProjectRoot)
        {
            if (descriptor.IsIgnored)
            {
                return new TestCaseResult(
                    descriptor,
                    TestOutcome.Skipped,
                    0.0,
                    descriptor.IgnoreReason ?? "Ignored",
                    null,
                    Array.Empty<string>());
            }

            if (methodMeta != null
                && !methodMeta.PlatformFilter.IsSupported(Application.platform, out string? platformSkipReason))
            {
                return new TestCaseResult(
                    descriptor,
                    TestOutcome.Skipped,
                    0.0,
                    platformSkipReason ?? "Skipped by UnityPlatform.",
                    null,
                    Array.Empty<string>());
            }

            int maxTries = methodMeta != null && methodMeta.RetryCount > 1 ? methodMeta.RetryCount : 1;
            int repeatCount = methodMeta != null && methodMeta.RepeatCount > 1 ? methodMeta.RepeatCount : 1;

            TestCaseResult? lastAttemptResult = null;
            for (int attempt = 1; attempt <= maxTries; attempt++)
            {
                for (int rep = 1; rep <= repeatCount; rep++)
                {
                    lastAttemptResult = ExecuteSingleTestAttempt(descriptor, options, fullProjectRoot);
                    if (lastAttemptResult.Outcome != TestOutcome.Passed)
                    {
                        break;
                    }
                }

                if (lastAttemptResult != null && lastAttemptResult.Outcome == TestOutcome.Passed)
                {
                    return lastAttemptResult;
                }
            }

            return lastAttemptResult ?? new TestCaseResult(
                descriptor,
                TestOutcome.Failed,
                0.0,
                "Test did not execute.",
                null,
                Array.Empty<string>());
        }

        private static TestCaseResult ExecuteSingleTestAttempt(
            DiscoveredTestCaseDescriptor descriptor,
            TestRunOptions options,
            string fullProjectRoot)
        {
            if (!TestExecutionRegistry.TryGet(descriptor.Id, out RegisteredTestExecutionBinding? binding)
                || binding == null)
            {
                return new TestCaseResult(
                    descriptor,
                    TestOutcome.Failed,
                    0.0,
                    "Test binding was not registered for '" + descriptor.FullName + "'.",
                    null,
                    Array.Empty<string>());
            }

            Stopwatch caseTimer = Stopwatch.StartNew();
            LogRecorder.Clear();
            LogAssert.Reset();
            int consumedLogCount = 0;
            List<string> capturedLogs = new List<string>();

            BridgeHost? playHost = null;
            ISceneHost? previousSceneHost = SceneManager.Host;
            IApplicationHost? previousAppHost = Application.Host;
            PlayModeIsolatedSceneHost? isolatedSceneHost = null;
            float savedTimeScale = Time.timeScale;

            UnityEngine.Object[] preExistingObjects = UnityEngine.Object.FindObjectsOfType<UnityEngine.Object>(true);
            HashSet<int> preExistingIds = new HashSet<int>();
            Dictionary<int, TransformSnapshot> preExistingTransforms = new Dictionary<int, TransformSnapshot>();
            Dictionary<int, GameObjectSnapshot> preExistingGameObjects = new Dictionary<int, GameObjectSnapshot>();
            for (int i = 0; i < preExistingObjects.Length; i++)
            {
                UnityEngine.Object obj = preExistingObjects[i];
                if (obj is not null)
                {
                    int id = obj.GetInstanceID();
                    preExistingIds.Add(id);
                    if (obj is Transform tr)
                    {
                        preExistingTransforms[id] = new TransformSnapshot(tr, tr.localPosition, tr.localRotation, tr.localScale);
                    }
                    else if (obj is GameObject go)
                    {
                        preExistingGameObjects[id] = new GameObjectSnapshot(go, go.name, go.activeSelf, go.tag, go.layer);
                    }
                }
            }

            string? failureMessage = null;
            string? failureStack = null;

            try
            {
                if (descriptor.Mode == TestExecutionMode.PlayMode)
                {
                    try
                    {
                        playHost = new BridgeHost();
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        playHost = null;
                    }

                    string testSceneName = "UnitasPlayModeTestScene_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture);
                    Scene freshScene = SceneManager.CreateScene(testSceneName);
                    isolatedSceneHost = new PlayModeIsolatedSceneHost(SceneManager.Host, freshScene);
                    SceneManager.Host = isolatedSceneHost;
                    SceneManager.SetActiveScene(freshScene);
                }

                if (binding.SyncExecute != null)
                {
                    binding.SyncExecute();
                    DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);
                }
                else if (binding.AsyncExecute != null)
                {
                    IEnumerator routine = binding.AsyncExecute();
                    DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);
                    DriveUnityTestEnumerator(
                        routine,
                        playHost,
                        options,
                        ref consumedLogCount,
                        capturedLogs);
                }
            }
            catch (AssertionException aex)
            {
                DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);
                failureMessage = aex.Message;
                failureStack = aex.StackTrace;
            }
            catch (Exception ex)
            {
                DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);
                failureMessage = ex.Message;
                failureStack = ex.StackTrace;
            }
            finally
            {
                if (binding.TearDownExecute != null)
                {
                    try
                    {
                        binding.TearDownExecute();
                        DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);
                    }
                    catch (Exception tdEx)
                    {
                        if (string.IsNullOrEmpty(failureMessage))
                        {
                            failureMessage = "TearDown failed: " + tdEx.Message;
                            failureStack = tdEx.StackTrace;
                        }
                    }
                }

                CleanupTestCreatedObjects(preExistingIds);
                RestoreEditTimeSnapshots(preExistingTransforms, preExistingGameObjects);
                CoroutineScheduler.StopAll();
                InvokeScheduler.StopAll();
                Time.timeScale = savedTimeScale;
                playHost?.Dispose();

                SceneManager.Host = previousSceneHost;
                Application.Host = previousAppHost ?? new EditTimeApplicationHost(Path.Combine(fullProjectRoot, "Assets"), string.Empty);
            }

            IReadOnlyList<string> logProblems = LogAssert.Verify();
            string logProblemText = logProblems.Count > 0 ? string.Join("; ", logProblems) : string.Empty;
            if (string.IsNullOrEmpty(failureMessage))
            {
                failureMessage = logProblemText;
                if (string.IsNullOrEmpty(failureStack))
                {
                    failureStack = "at " + descriptor.FullName + "()";
                }
            }
            else if (logProblemText.Length > 0)
            {
                // Keep the log-assertion verdict even when the body already failed.
                failureMessage = failureMessage + "; " + logProblemText;
            }

            LogAssert.Reset();
            double duration = caseTimer.Elapsed.TotalSeconds;

            if (!string.IsNullOrEmpty(failureMessage))
            {
                return new TestCaseResult(
                    descriptor,
                    TestOutcome.Failed,
                    duration,
                    failureMessage,
                    failureStack,
                    capturedLogs);
            }

            return new TestCaseResult(
                descriptor,
                TestOutcome.Passed,
                duration,
                null,
                null,
                capturedLogs);
        }

        private static void DriveUnityTestEnumerator(
            IEnumerator rootRoutine,
            BridgeHost? playHost,
            TestRunOptions options,
            ref int consumedLogCount,
            List<string> capturedLogs)
        {
            if (rootRoutine == null)
            {
                return;
            }

            Stack<IEnumerator> stack = new Stack<IEnumerator>();
            stack.Push(rootRoutine);

            int maxFrames = options.MaxUnityTestFrames > 0 ? options.MaxUnityTestFrames : 1000;
            double delta = options.FrameDeltaSeconds > 0.0 ? options.FrameDeltaSeconds : 0.02;
            int frames = 0;

            while (stack.Count > 0)
            {
                if (options.CancellationToken.IsCancellationRequested)
                {
                    break;
                }

                if (frames >= maxFrames)
                {
                    throw new AssertionException(
                        "UnityTest exceeded maximum frame budget (" + maxFrames.ToString(CultureInfo.InvariantCulture) + " frames).");
                }

                IEnumerator current = stack.Peek();
                bool hasNext = current.MoveNext();
                DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);

                if (!hasNext)
                {
                    stack.Pop();
                    continue;
                }

                object? yielded = current.Current;
                if (yielded is IEnumerator nested && !(yielded is CustomYieldInstruction))
                {
                    stack.Push(nested);
                    continue;
                }

                if (yielded is WaitForSeconds waitSeconds)
                {
                    float remaining = waitSeconds.Seconds;
                    while (remaining > 1e-6f)
                    {
                        if (options.CancellationToken.IsCancellationRequested)
                        {
                            break;
                        }

                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        remaining -= (float)delta * Math.Max(0.0001f, Time.timeScale);
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest WaitForSeconds exceeded frame budget.");
                        }
                    }

                    continue;
                }

                if (yielded is WaitForSecondsRealtime waitRealtime)
                {
                    float remaining = waitRealtime.Seconds;
                    while (remaining > 1e-6f)
                    {
                        if (options.CancellationToken.IsCancellationRequested)
                        {
                            break;
                        }

                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        remaining -= (float)delta;
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest WaitForSecondsRealtime exceeded frame budget.");
                        }
                    }

                    continue;
                }

                if (yielded is WaitUntil waitUntil)
                {
                    while (!waitUntil.Predicate())
                    {
                        if (options.CancellationToken.IsCancellationRequested)
                        {
                            break;
                        }

                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest WaitUntil exceeded frame budget.");
                        }
                    }

                    continue;
                }

                if (yielded is AsyncOperation asyncOp)
                {
                    while (!asyncOp.isDone)
                    {
                        if (options.CancellationToken.IsCancellationRequested)
                        {
                            break;
                        }

                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest AsyncOperation exceeded frame budget.");
                        }
                    }

                    continue;
                }

                if (yielded is Coroutine)
                {
                    while (CoroutineScheduler.ActiveCount > 0)
                    {
                        if (options.CancellationToken.IsCancellationRequested)
                        {
                            break;
                        }

                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest Coroutine exceeded frame budget.");
                        }
                    }

                    continue;
                }

                if (yielded is CustomYieldInstruction customYield)
                {
                    while (customYield.keepWaiting)
                    {
                        if (options.CancellationToken.IsCancellationRequested)
                        {
                            break;
                        }

                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest CustomYieldInstruction exceeded frame budget.");
                        }
                    }

                    continue;
                }

                if (yielded is WaitForFixedUpdate || yielded is WaitForEndOfFrame || yielded == null)
                {
                    StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                    frames++;
                    continue;
                }

                StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                frames++;
            }
        }

        private static void StepSingleFrame(
            BridgeHost? playHost,
            double deltaSeconds,
            ref int consumedLogCount,
            List<string> capturedLogs)
        {
            if (playHost != null)
            {
                playHost.Tick(deltaSeconds);
            }
            else
            {
                Time.Advance(deltaSeconds);
            }

            InvokeScheduler.PumpFrame();
            CoroutineScheduler.PumpFixed();
            CoroutineScheduler.PumpFrame();
            SceneManager.FlushDeferredOperations();
            DrainNewLogsToLogAssert(ref consumedLogCount, capturedLogs);
        }

        private static void DrainNewLogsToLogAssert(ref int consumedLogCount, List<string> capturedLogs)
        {
            LogAssert.DrainFromLogRecorder();
            IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
            while (consumedLogCount < entries.Count)
            {
                LogEntry entry = entries[consumedLogCount];
                consumedLogCount++;

                LogType logType = entry.Severity switch
                {
                    LogSeverity.Error => LogType.Error,
                    LogSeverity.Warning => LogType.Warning,
                    _ => LogType.Log
                };

                capturedLogs.Add("[" + logType + "] " + entry.Message);
            }
        }

        private static void CleanupTestCreatedObjects(HashSet<int> preExistingIds)
        {
            UnityEngine.Object[] currentObjects = UnityEngine.Object.FindObjectsOfType<UnityEngine.Object>(true);
            for (int i = 0; i < currentObjects.Length; i++)
            {
                UnityEngine.Object obj = currentObjects[i];
                if (obj is not null && !preExistingIds.Contains(obj.GetInstanceID()))
                {
                    try
                    {
                        UnityEngine.Object.Destroy(obj);
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                    }
                }
            }
        }

        private static void RestoreEditTimeSnapshots(
            Dictionary<int, TransformSnapshot> transforms,
            Dictionary<int, GameObjectSnapshot> gameObjects)
        {
            foreach (KeyValuePair<int, GameObjectSnapshot> kv in gameObjects)
            {
                GameObjectSnapshot snap = kv.Value;
                if (snap.Instance != null)
                {
                    snap.Instance.name = snap.Name;
                    snap.Instance.tag = snap.Tag;
                    snap.Instance.layer = snap.Layer;
                    if (snap.Instance.activeSelf != snap.ActiveSelf)
                    {
                        snap.Instance.SetActive(snap.ActiveSelf);
                    }
                }
            }

            foreach (KeyValuePair<int, TransformSnapshot> kv in transforms)
            {
                TransformSnapshot snap = kv.Value;
                if (snap.Instance != null)
                {
                    snap.Instance.localPosition = snap.LocalPosition;
                    snap.Instance.localRotation = snap.LocalRotation;
                    snap.Instance.localScale = snap.LocalScale;
                }
            }
        }

        private static List<DiscoveredTestCaseDescriptor> FilterDescriptors(
            List<DiscoveredTestCaseDescriptor> allDescriptors,
            TestRunOptions options)
        {
            List<DiscoveredTestCaseDescriptor> selected = new List<DiscoveredTestCaseDescriptor>();
            for (int i = 0; i < allDescriptors.Count; i++)
            {
                DiscoveredTestCaseDescriptor d = allDescriptors[i];
                if (options.Mode == TestModeFilter.EditMode && d.Mode != TestExecutionMode.EditMode)
                {
                    continue;
                }

                if (options.Mode == TestModeFilter.PlayMode && d.Mode != TestExecutionMode.PlayMode)
                {
                    continue;
                }

                if (!MatchesAssemblyFilters(d.AssemblyName, options))
                {
                    continue;
                }

                if (!string.IsNullOrEmpty(options.CategoryFilter))
                {
                    string[] wanted = options.CategoryFilter.Split(',', StringSplitOptions.RemoveEmptyEntries);
                    bool matchedCategory = false;
                    for (int w = 0; w < wanted.Length; w++)
                    {
                        string targetCat = wanted[w].Trim();
                        for (int c = 0; c < d.Categories.Count; c++)
                        {
                            if (string.Equals(d.Categories[c], targetCat, StringComparison.OrdinalIgnoreCase))
                            {
                                matchedCategory = true;
                                break;
                            }
                        }

                        if (matchedCategory)
                        {
                            break;
                        }
                    }

                    if (!matchedCategory)
                    {
                        continue;
                    }
                }

                if (!string.IsNullOrEmpty(options.TestNameFilter))
                {
                    if (!MatchesTestNameFilter(d, options.TestNameFilter))
                    {
                        continue;
                    }
                }

                selected.Add(d);
            }

            return selected;
        }

        private static bool MatchesAssemblyFilters(string assemblyName, TestRunOptions options)
        {
            List<string> filters = new List<string>();
            if (!string.IsNullOrEmpty(options.AssemblyFilter))
            {
                string[] parts = options.AssemblyFilter.Split(new[] { ',', ';', '|' }, StringSplitOptions.RemoveEmptyEntries);
                for (int i = 0; i < parts.Length; i++)
                {
                    string trimmed = parts[i].Trim();
                    if (!string.IsNullOrEmpty(trimmed))
                    {
                        filters.Add(trimmed);
                    }
                }
            }

            if (options.AssemblyFilters != null)
            {
                for (int i = 0; i < options.AssemblyFilters.Count; i++)
                {
                    string item = options.AssemblyFilters[i];
                    if (!string.IsNullOrEmpty(item))
                    {
                        string[] parts = item.Split(new[] { ',', ';', '|' }, StringSplitOptions.RemoveEmptyEntries);
                        for (int p = 0; p < parts.Length; p++)
                        {
                            string trimmed = parts[p].Trim();
                            if (!string.IsNullOrEmpty(trimmed))
                            {
                                filters.Add(trimmed);
                            }
                        }
                    }
                }
            }

            if (filters.Count == 0)
            {
                return true;
            }

            for (int i = 0; i < filters.Count; i++)
            {
                string candidate = filters[i];
                if (candidate.EndsWith(".dll", StringComparison.OrdinalIgnoreCase))
                {
                    candidate = candidate.Substring(0, candidate.Length - 4);
                }

                if (string.Equals(assemblyName, candidate, StringComparison.OrdinalIgnoreCase)
                    || MatchesSingleNamePattern(assemblyName, candidate))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool MatchesTestNameFilter(DiscoveredTestCaseDescriptor descriptor, string rawFilter)
        {
            string[] patterns = rawFilter.Split(new[] { ';', '|' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < patterns.Length; i++)
            {
                string pattern = patterns[i].Trim();
                if (string.IsNullOrEmpty(pattern))
                {
                    continue;
                }

                if (MatchesSingleNamePattern(descriptor.FullName, pattern)
                    || MatchesSingleNamePattern(descriptor.DisplayName, pattern)
                    || MatchesSingleNamePattern(descriptor.MethodName, pattern)
                    || MatchesSingleNamePattern(descriptor.FullClassName, pattern)
                    || MatchesSingleNamePattern(descriptor.ClassName, pattern))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool MatchesSingleNamePattern(string candidate, string pattern)
        {
            if (string.IsNullOrEmpty(candidate) || string.IsNullOrEmpty(pattern))
            {
                return false;
            }

            if (pattern.Contains('*', StringComparison.Ordinal) || pattern.Contains('?', StringComparison.Ordinal))
            {
                StringBuilder rxBuilder = new StringBuilder();
                rxBuilder.Append('^');
                for (int i = 0; i < pattern.Length; i++)
                {
                    char c = pattern[i];
                    if (c == '*')
                    {
                        rxBuilder.Append(".*");
                    }
                    else if (c == '?')
                    {
                        rxBuilder.Append('.');
                    }
                    else
                    {
                        rxBuilder.Append(Regex.Escape(c.ToString()));
                    }
                }

                rxBuilder.Append('$');
                return Regex.IsMatch(candidate, rxBuilder.ToString(), RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);
            }

            if (string.Equals(candidate, pattern, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            if (candidate.StartsWith(pattern + ".", StringComparison.OrdinalIgnoreCase)
                || candidate.StartsWith(pattern + "(", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            return candidate.Contains(pattern, StringComparison.OrdinalIgnoreCase);
        }

        private static bool CompileAndLoadTestGlue(
            string outputDir,
            ScriptCompilationResult compileResult,
            ScriptCompilationOptions baseOptions,
            List<FixtureGenerationPlan> fixturePlans,
            Dictionary<string, SupplementalTestMetadata> metadataByMethodKey,
            Dictionary<string, LifecycleHooksMetadata> lifecycleByClassKey,
            List<string> assemblyPrebuildTypes,
            List<string> assemblyPostBuildTypes,
            List<ScriptDiagnostic> diagnostics,
            AssemblyLoadContext loadContext)
        {
            string baseGlueSource = TestDiscoveryAndGlueGenerator.GenerateTestRunnerGlueSource(fixturePlans);
            string glueSource = InjectLifecycleHooksIntoGlueSource(
                baseGlueSource,
                metadataByMethodKey,
                lifecycleByClassKey,
                assemblyPrebuildTypes,
                assemblyPostBuildTypes);
            string tempGlueDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_test_glue_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(tempGlueDir);

            try
            {
                string assetsDir = Path.Combine(tempGlueDir, "Assets");
                Directory.CreateDirectory(assetsDir);
                string glueFile = Path.Combine(assetsDir, "UnitasGeneratedTestGlue.cs");
                File.WriteAllText(glueFile, glueSource, Encoding.UTF8);

                ScriptCompilationOptions glueCompileOptions = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    OutputDirectory = Path.Combine(tempGlueDir, "Out"),
                    EmitGeneratedGlue = false,
                    AutoLoadGeneratedGlue = false
                };

                for (int i = 0; i < baseOptions.AdditionalReferencePaths.Count; i++)
                {
                    glueCompileOptions.AdditionalReferencePaths.Add(baseOptions.AdditionalReferencePaths[i]);
                }

                for (int i = 0; i < compileResult.Assemblies.Count; i++)
                {
                    CompiledAssemblyRecord asm = compileResult.Assemblies[i];
                    if (!asm.IsExcluded && File.Exists(asm.OutputPath))
                    {
                        glueCompileOptions.AdditionalReferencePaths.Add(asm.OutputPath);
                    }
                }

                ScriptCompilationResult glueRes = ScriptCompilation.CompileProject(tempGlueDir, glueCompileOptions);
                if (!glueRes.Succeeded)
                {
                    for (int i = 0; i < glueRes.Diagnostics.Count; i++)
                    {
                        diagnostics.Add(glueRes.Diagnostics[i]);
                    }

                    return false;
                }

                CompiledAssemblyRecord? compiledGlue = glueRes.FindAssembly(AssemblyDiscovery.AssemblyCSharp);
                if (compiledGlue == null || !File.Exists(compiledGlue.OutputPath))
                {
                    return false;
                }

                string targetGlueDll = Path.Combine(
                    outputDir,
                    "Unitas.TestGlue." + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture) + ".dll");
                File.Copy(compiledGlue.OutputPath, targetGlueDll, true);

                AssemblyLoadContext testGlueContext = new AssemblyLoadContext(
                    "UnitasTestGlue_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture),
                    isCollectible: true);
                testGlueContext.Resolving += (alc, assemblyName) =>
                {
                    string simpleName = assemblyName.Name ?? string.Empty;
                    foreach (var existing in loadContext.Assemblies)
                    {
                        if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                        {
                            return existing;
                        }
                    }

                    foreach (var existing in AssemblyLoadContext.Default.Assemblies)
                    {
                        if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                        {
                            return existing;
                        }
                    }

                    return null;
                };
                using FileStream testGlueStream = File.OpenRead(targetGlueDll);
                var loaded = testGlueContext.LoadFromStream(testGlueStream);
                RuntimeHelpers.RunModuleConstructor(loaded.ManifestModule.ModuleHandle);
                loaded.CreateInstance("UnitasGeneratedTests.TestGlueInitializer");
                return true;
            }
            finally
            {
                if (Directory.Exists(tempGlueDir))
                {
                    try
                    {
                        Directory.Delete(tempGlueDir, true);
                    }
                    catch (IOException)
                    {
                    }
                }
            }
        }

        private static string InjectLifecycleHooksIntoGlueSource(
            string baseGlueSource,
            Dictionary<string, SupplementalTestMetadata> metadataByMethodKey,
            Dictionary<string, LifecycleHooksMetadata> lifecycleByClassKey,
            List<string> assemblyPrebuildTypes,
            List<string> assemblyPostBuildTypes)
        {
            StringBuilder hookLines = new StringBuilder();
            EmitHookRegistration(hookLines, "__lifecycle_prebuild_assembly__", assemblyPrebuildTypes, isPrebuild: true);
            EmitHookRegistration(hookLines, "__lifecycle_postbuild_assembly__", assemblyPostBuildTypes, isPrebuild: false);

            foreach (KeyValuePair<string, LifecycleHooksMetadata> kv in lifecycleByClassKey)
            {
                EmitHookRegistration(hookLines, "__lifecycle_prebuild_class__::" + kv.Key, kv.Value.PrebuildTypes, isPrebuild: true);
                EmitHookRegistration(hookLines, "__lifecycle_postbuild_class__::" + kv.Key, kv.Value.PostBuildTypes, isPrebuild: false);
            }

            foreach (KeyValuePair<string, SupplementalTestMetadata> kv in metadataByMethodKey)
            {
                EmitHookRegistration(hookLines, "__lifecycle_prebuild_method__::" + kv.Key, kv.Value.PrebuildTypes, isPrebuild: true);
                EmitHookRegistration(hookLines, "__lifecycle_postbuild_method__::" + kv.Key, kv.Value.PostBuildTypes, isPrebuild: false);
            }

            if (hookLines.Length == 0)
            {
                return baseGlueSource;
            }

            const string anchor = "        public static void Initialize()\n        {\n";
            const string anchorCrLf = "        public static void Initialize()\r\n        {\r\n";
            if (baseGlueSource.Contains(anchor, StringComparison.Ordinal))
            {
                return baseGlueSource.Replace(anchor, anchor + hookLines, StringComparison.Ordinal);
            }

            if (baseGlueSource.Contains(anchorCrLf, StringComparison.Ordinal))
            {
                return baseGlueSource.Replace(anchorCrLf, anchorCrLf + hookLines, StringComparison.Ordinal);
            }

            return baseGlueSource;
        }

        private static void EmitHookRegistration(
            StringBuilder sb,
            string hookId,
            IReadOnlyList<string> targetTypes,
            bool isPrebuild)
        {
            if (targetTypes.Count == 0)
            {
                return;
            }

            string escapedHookId = hookId
                .Replace("\\", "\\\\", StringComparison.Ordinal)
                .Replace("\"", "\\\"", StringComparison.Ordinal);

            sb.AppendLine("            TestExecutionRegistry.RegisterSync(");
            sb.AppendLine("                \"" + escapedHookId + "\",");
            sb.AppendLine("                () =>");
            sb.AppendLine("                {");
            for (int i = 0; i < targetTypes.Count; i++)
            {
                string typeRef = targetTypes[i].StartsWith("global::", StringComparison.Ordinal)
                    ? targetTypes[i]
                    : "global::" + targetTypes[i];
                if (isPrebuild)
                {
                    sb.AppendLine("                    { " + typeRef + " h = new " + typeRef + "(); if (h is global::UnityEngine.TestTools.IPrebuildSetup s) s.Setup(); }");
                }
                else
                {
                    sb.AppendLine("                    { " + typeRef + " h = new " + typeRef + "(); if (h is global::UnityEngine.TestTools.IPostBuildCleanup c) c.Cleanup(); }");
                }
            }

            sb.AppendLine("                },");
            sb.AppendLine("                null);");
        }

        private static string BuildMethodMetadataKey(string assemblyName, string fullClassName, string methodName)
        {
            return assemblyName + "::" + fullClassName + "." + methodName;
        }

        private static void ScanSupplementalMetadataInAssembly(
            DiscoveredTestAssembly assembly,
            HashSet<string> activeDefines,
            Dictionary<string, SupplementalTestMetadata> metadataByMethodKey,
            Dictionary<string, LifecycleHooksMetadata> lifecycleByClassKey,
            List<string> assemblyPrebuildTypes,
            List<string> assemblyPostBuildTypes)
        {
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

                string cleanSource = StripCommentsFromSource(
                    ScriptAnalyzerAndGlueGenerator.FilterConditionalCompilation(rawSource, activeDefines));
                string ns = ExtractSourceNamespace(cleanSource);

                ExtractLifecycleTargetTypes(FileAssemblyPrebuildRegex.Matches(cleanSource), ns, assemblyPrebuildTypes);
                ExtractLifecycleTargetTypes(FileAssemblyPostBuildRegex.Matches(cleanSource), ns, assemblyPostBuildTypes);

                MatchCollection classMatches = ClassSourceRegex.Matches(cleanSource);
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
                    string classBody = ExtractSourceBraceBody(cleanSource, cm.Index + cm.Length);
                    if (string.IsNullOrEmpty(classBody))
                    {
                        continue;
                    }

                    string fullClassName = string.IsNullOrEmpty(ns) ? className : ns + "." + className;
                    string classScopeKey = assembly.Name + "::" + fullClassName;

                    List<string> classPrebuilds = new List<string>();
                    List<string> classPostBuilds = new List<string>();
                    ExtractLifecycleTargetTypes(PrebuildSetupAttrRegex.Matches(classAttrs), ns, classPrebuilds);
                    ExtractLifecycleTargetTypes(PostBuildCleanupAttrRegex.Matches(classAttrs), ns, classPostBuilds);
                    if (classPrebuilds.Count > 0 || classPostBuilds.Count > 0)
                    {
                        lifecycleByClassKey[classScopeKey] = new LifecycleHooksMetadata(classPrebuilds, classPostBuilds);
                    }

                    PlatformFilterSpec classPlatform = ParseUnityPlatformAttributes(classAttrs);

                    MatchCollection methodMatches = MethodSourceRegex.Matches(classBody);
                    for (int mIdx = 0; mIdx < methodMatches.Count; mIdx++)
                    {
                        Match mm = methodMatches[mIdx];
                        string methodAttrs = mm.Groups[1].Value;
                        string methodName = mm.Groups[5].Value.Trim();

                        int retryCount = ParseIntAttribute(RetryAttrRegex, methodAttrs, 1);
                        int repeatCount = ParseIntAttribute(RepeatAttrRegex, methodAttrs, 1);
                        PlatformFilterSpec methodPlatform = ParseUnityPlatformAttributes(methodAttrs);
                        PlatformFilterSpec combinedPlatform = PlatformFilterSpec.Combine(classPlatform, methodPlatform);

                        List<string> methodPrebuilds = new List<string>();
                        List<string> methodPostBuilds = new List<string>();
                        ExtractLifecycleTargetTypes(PrebuildSetupAttrRegex.Matches(methodAttrs), ns, methodPrebuilds);
                        ExtractLifecycleTargetTypes(PostBuildCleanupAttrRegex.Matches(methodAttrs), ns, methodPostBuilds);

                        string methodKey = BuildMethodMetadataKey(assembly.Name, fullClassName, methodName);
                        metadataByMethodKey[methodKey] = new SupplementalTestMetadata(
                            retryCount,
                            repeatCount,
                            combinedPlatform,
                            methodPrebuilds,
                            methodPostBuilds);
                    }
                }
            }
        }

        private static int ParseIntAttribute(Regex regex, string attributeBlock, int defaultValue)
        {
            if (string.IsNullOrEmpty(attributeBlock))
            {
                return defaultValue;
            }

            Match m = regex.Match(attributeBlock);
            if (m.Success && int.TryParse(m.Groups[1].Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int val))
            {
                return Math.Max(1, val);
            }

            return defaultValue;
        }

        private static void ExtractLifecycleTargetTypes(MatchCollection matches, string currentNamespace, List<string> targetList)
        {
            for (int i = 0; i < matches.Count; i++)
            {
                string rawArg = matches[i].Groups[1].Value.Trim();
                string resolved = ResolveLifecycleTargetTypeName(rawArg, currentNamespace);
                if (!string.IsNullOrEmpty(resolved) && !targetList.Contains(resolved))
                {
                    targetList.Add(resolved);
                }
            }
        }

        private static string ResolveLifecycleTargetTypeName(string rawArg, string currentNamespace)
        {
            if (string.IsNullOrEmpty(rawArg))
            {
                return string.Empty;
            }

            string candidate = rawArg.Trim();
            if (candidate.StartsWith("typeof", StringComparison.Ordinal))
            {
                int open = candidate.IndexOf('(', StringComparison.Ordinal);
                int close = candidate.LastIndexOf(')');
                if (open >= 0 && close > open)
                {
                    candidate = candidate.Substring(open + 1, close - open - 1).Trim();
                }
            }
            else if (candidate.StartsWith('\"') && candidate.EndsWith('\"') && candidate.Length >= 2)
            {
                candidate = candidate.Substring(1, candidate.Length - 2).Trim();
            }

            int comma = candidate.IndexOf(',', StringComparison.Ordinal);
            if (comma > 0)
            {
                candidate = candidate.Substring(0, comma).Trim();
            }

            if (candidate.StartsWith("global::", StringComparison.Ordinal))
            {
                candidate = candidate.Substring("global::".Length);
            }

            if (!candidate.Contains('.', StringComparison.Ordinal) && !string.IsNullOrEmpty(currentNamespace))
            {
                return currentNamespace + "." + candidate;
            }

            return candidate;
        }

        private static PlatformFilterSpec ParseUnityPlatformAttributes(string attributeBlock)
        {
            List<RuntimePlatform> includes = new List<RuntimePlatform>();
            List<RuntimePlatform> excludes = new List<RuntimePlatform>();

            if (string.IsNullOrEmpty(attributeBlock))
            {
                return new PlatformFilterSpec(includes, excludes);
            }

            MatchCollection matches = UnityPlatformAttrRegex.Matches(attributeBlock);
            for (int i = 0; i < matches.Count; i++)
            {
                string args = matches[i].Groups[1].Success ? matches[i].Groups[1].Value.Trim() : string.Empty;
                if (string.IsNullOrEmpty(args))
                {
                    continue;
                }

                ParsePlatformArguments(args, includes, excludes);
            }

            return new PlatformFilterSpec(includes, excludes);
        }

        private static void ParsePlatformArguments(string args, List<RuntimePlatform> includes, List<RuntimePlatform> excludes)
        {
            int excludeAssign = args.IndexOf("exclude", StringComparison.Ordinal);
            int includeAssign = args.IndexOf("include", StringComparison.Ordinal);

            if (excludeAssign >= 0 || includeAssign >= 0)
            {
                if (includeAssign >= 0)
                {
                    string incSegment = excludeAssign > includeAssign
                        ? args.Substring(includeAssign, excludeAssign - includeAssign)
                        : args.Substring(includeAssign);
                    ExtractPlatformsFromText(incSegment, includes);
                }

                if (excludeAssign >= 0)
                {
                    string excSegment = includeAssign > excludeAssign
                        ? args.Substring(excludeAssign, includeAssign - excludeAssign)
                        : args.Substring(excludeAssign);
                    ExtractPlatformsFromText(excSegment, excludes);
                }

                return;
            }

            ExtractPlatformsFromText(args, includes);
        }

        private static void ExtractPlatformsFromText(string text, List<RuntimePlatform> destination)
        {
            MatchCollection matches = RuntimePlatformTokenRegex.Matches(text);
            for (int i = 0; i < matches.Count; i++)
            {
                string token = matches[i].Groups[1].Value;
                if (Enum.TryParse<RuntimePlatform>(token, ignoreCase: false, out RuntimePlatform plat))
                {
                    if (!destination.Contains(plat))
                    {
                        destination.Add(plat);
                    }
                }
            }
        }

        private static string ExtractSourceNamespace(string source)
        {
            Match m = NamespaceSourceRegex.Match(source);
            return m.Success ? m.Groups[1].Value.Trim() : string.Empty;
        }

        private static string ExtractSourceBraceBody(string source, int startIndex)
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

        private static string StripCommentsFromSource(string source)
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

        private sealed class PlatformFilterSpec
        {
            public PlatformFilterSpec(IReadOnlyList<RuntimePlatform> include, IReadOnlyList<RuntimePlatform> exclude)
            {
                Include = include;
                Exclude = exclude;
            }

            public IReadOnlyList<RuntimePlatform> Include { get; }

            public IReadOnlyList<RuntimePlatform> Exclude { get; }

            public static PlatformFilterSpec Combine(PlatformFilterSpec parent, PlatformFilterSpec child)
            {
                List<RuntimePlatform> inc = new List<RuntimePlatform>(parent.Include);
                for (int i = 0; i < child.Include.Count; i++)
                {
                    if (!inc.Contains(child.Include[i]))
                    {
                        inc.Add(child.Include[i]);
                    }
                }

                List<RuntimePlatform> exc = new List<RuntimePlatform>(parent.Exclude);
                for (int i = 0; i < child.Exclude.Count; i++)
                {
                    if (!exc.Contains(child.Exclude[i]))
                    {
                        exc.Add(child.Exclude[i]);
                    }
                }

                return new PlatformFilterSpec(inc, exc);
            }

            public bool IsSupported(RuntimePlatform current, out string? skipReason)
            {
                for (int i = 0; i < Exclude.Count; i++)
                {
                    if (Exclude[i] == current)
                    {
                        skipReason = "Only supported on platforms other than " + current + " (excluded: " + string.Join(", ", Exclude) + ").";
                        return false;
                    }
                }

                if (Include.Count > 0)
                {
                    bool matched = false;
                    for (int i = 0; i < Include.Count; i++)
                    {
                        if (Include[i] == current)
                        {
                            matched = true;
                            break;
                        }
                    }

                    if (!matched)
                    {
                        skipReason = "Only supported on " + string.Join(", ", Include) + " (current platform: " + current + ").";
                        return false;
                    }
                }

                skipReason = null;
                return true;
            }
        }

        private sealed class SupplementalTestMetadata
        {
            public SupplementalTestMetadata(
                int retryCount,
                int repeatCount,
                PlatformFilterSpec platformFilter,
                IReadOnlyList<string> prebuildTypes,
                IReadOnlyList<string> postBuildTypes)
            {
                RetryCount = retryCount;
                RepeatCount = repeatCount;
                PlatformFilter = platformFilter;
                PrebuildTypes = prebuildTypes;
                PostBuildTypes = postBuildTypes;
            }

            public int RetryCount { get; }

            public int RepeatCount { get; }

            public PlatformFilterSpec PlatformFilter { get; }

            public IReadOnlyList<string> PrebuildTypes { get; }

            public IReadOnlyList<string> PostBuildTypes { get; }
        }

        private sealed class LifecycleHooksMetadata
        {
            public LifecycleHooksMetadata(IReadOnlyList<string> prebuildTypes, IReadOnlyList<string> postBuildTypes)
            {
                PrebuildTypes = prebuildTypes;
                PostBuildTypes = postBuildTypes;
            }

            public IReadOnlyList<string> PrebuildTypes { get; }

            public IReadOnlyList<string> PostBuildTypes { get; }
        }

        private readonly struct TransformSnapshot
        {
            public TransformSnapshot(Transform instance, Vector3 localPosition, Quaternion localRotation, Vector3 localScale)
            {
                Instance = instance;
                LocalPosition = localPosition;
                LocalRotation = localRotation;
                LocalScale = localScale;
            }

            public Transform Instance { get; }

            public Vector3 LocalPosition { get; }

            public Quaternion LocalRotation { get; }

            public Vector3 LocalScale { get; }
        }

        private readonly struct GameObjectSnapshot
        {
            public GameObjectSnapshot(GameObject instance, string name, bool activeSelf, string tag, int layer)
            {
                Instance = instance;
                Name = name;
                ActiveSelf = activeSelf;
                Tag = tag;
                Layer = layer;
            }

            public GameObject Instance { get; }

            public string Name { get; }

            public bool ActiveSelf { get; }

            public string Tag { get; }

            public int Layer { get; }
        }

        private sealed class EditTimeApplicationHost : IApplicationHost
        {
            public EditTimeApplicationHost(string dataPath, string fallbackDataPath)
            {
                DataPath = !string.IsNullOrEmpty(dataPath) ? dataPath : fallbackDataPath;
            }

            public bool IsPlaying => false;

            public bool IsFocused => true;

            public bool IsPaused => false;

            public ApplicationBuildKind BuildKind => ApplicationBuildKind.Editor;

            public string DataPath { get; }

            public string StreamingAssetsPath => Path.Combine(DataPath, "StreamingAssets");

            public string CompanyName => "DefaultCompany";

            public string ProductName => "UnitasProduct";

            public string Version => "0.1";

            public void EndSession()
            {
            }
        }

        private sealed class PlayModeIsolatedSceneHost : ISceneHost
        {
            private readonly ISceneHost? _inner;
            private Scene _activeScene;
            private readonly List<Scene> _scenes = new List<Scene>();

            public PlayModeIsolatedSceneHost(ISceneHost? inner, Scene initialScene)
            {
                _inner = inner;
                _activeScene = initialScene;
                _scenes.Add(initialScene);
            }

            public int SceneCount => _scenes.Count;

            public bool TryResolveScenePath(string sceneName, out string scenePath)
            {
                if (_inner != null && _inner.TryResolveScenePath(sceneName, out scenePath))
                {
                    return true;
                }

                scenePath = sceneName;
                return false;
            }

            public Scene GetSceneAt(int index)
            {
                if (index >= 0 && index < _scenes.Count)
                {
                    return _scenes[index];
                }

                return default;
            }

            public Scene GetSceneByName(string sceneName)
            {
                for (int i = 0; i < _scenes.Count; i++)
                {
                    if (string.Equals(_scenes[i].name, sceneName, StringComparison.Ordinal))
                    {
                        return _scenes[i];
                    }
                }

                return _inner != null ? _inner.GetSceneByName(sceneName) : default;
            }

            public Scene GetSceneByBuildIndex(int buildIndex)
            {
                return _inner != null ? _inner.GetSceneByBuildIndex(buildIndex) : default;
            }

            public Scene GetActiveScene() => _activeScene;

            public bool SetActiveScene(Scene scene)
            {
                if (!scene.IsValid())
                {
                    return false;
                }

                _activeScene = scene;
                bool exists = false;
                for (int i = 0; i < _scenes.Count; i++)
                {
                    if (_scenes[i].handle == scene.handle)
                    {
                        _scenes[i] = scene;
                        exists = true;
                        break;
                    }
                }

                if (!exists)
                {
                    _scenes.Add(scene);
                }

                return true;
            }

            public Scene LoadScene(string scenePath, LoadSceneMode mode)
            {
                if (_inner != null)
                {
                    Scene loaded = _inner.LoadScene(scenePath, mode);
                    if (mode == LoadSceneMode.Single)
                    {
                        _scenes.Clear();
                    }

                    _scenes.Add(loaded);
                    _activeScene = loaded;
                    return loaded;
                }

                return _activeScene;
            }

            public void UnloadScene(string sceneName)
            {
                for (int i = _scenes.Count - 1; i >= 0; i--)
                {
                    if (string.Equals(_scenes[i].name, sceneName, StringComparison.Ordinal))
                    {
                        _scenes.RemoveAt(i);
                    }
                }

                _inner?.UnloadScene(sceneName);
            }

            public void DontDestroyOnLoad(ulong entity)
            {
                _inner?.DontDestroyOnLoad(entity);
            }
        }

        private static TestRunResult BuildCompileFailureResult(
            string projectRoot,
            TestRunOptions options,
            double durationSeconds,
            List<DiscoveredTestAssembly> testAssemblies,
            List<ScriptDiagnostic> diagnostics)
        {
            List<string> errorLines = new List<string>();
            for (int i = 0; i < diagnostics.Count; i++)
            {
                if (diagnostics[i].Severity == ScriptDiagnosticSeverity.Error)
                {
                    errorLines.Add(diagnostics[i].FormatCli());
                }
            }

            List<DiscoveredTestAssembly> markedAssemblies = new List<DiscoveredTestAssembly>();
            for (int i = 0; i < testAssemblies.Count; i++)
            {
                DiscoveredTestAssembly asm = testAssemblies[i];
                string reason = errorLines.Count > 0 ? errorLines[0] : "Compilation failed.";
                markedAssemblies.Add(new DiscoveredTestAssembly(
                    asm.Name,
                    asm.AsmdefPath,
                    asm.Mode,
                    asm.SourceFiles,
                    false,
                    reason));
            }

            string xml = NunitXmlWriter.GenerateXml(
                projectRoot,
                0,
                false,
                false,
                durationSeconds,
                Array.Empty<TestCaseResult>(),
                errorLines);

            if (!string.IsNullOrEmpty(options.ResultsXmlPath))
            {
                string fullXmlPath = Path.GetFullPath(options.ResultsXmlPath);
                string? dir = Path.GetDirectoryName(fullXmlPath);
                if (!string.IsNullOrEmpty(dir))
                {
                    Directory.CreateDirectory(dir);
                }

                File.WriteAllText(fullXmlPath, xml, Encoding.UTF8);
            }

            return new TestRunResult(
                false,
                false,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                durationSeconds,
                xml,
                options.ResultsXmlPath,
                markedAssemblies,
                Array.Empty<TestCaseResult>(),
                diagnostics);
        }
    }
}
