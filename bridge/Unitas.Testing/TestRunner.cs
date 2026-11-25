using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text;
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
        public static TestRunResult RunProject(string projectRoot, TestRunOptions? options = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            TestRunOptions effectiveOptions = options ?? new TestRunOptions();
            Stopwatch runStopwatch = Stopwatch.StartNew();

            List<DiscoveredTestAssembly> testAssemblies = TestDiscoveryAndGlueGenerator.DiscoverTestAssemblies(
                fullProjectRoot,
                out Dictionary<string, List<string>> _,
                out List<ScriptDiagnostic> discoveryDiagnostics);

            string outputDir = !string.IsNullOrEmpty(effectiveOptions.OutputDirectory)
                ? Path.GetFullPath(effectiveOptions.OutputDirectory)
                : Path.Combine(fullProjectRoot, "Library", ScriptCompilation.DefaultOutputFolderName);

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
            }

            List<DiscoveredTestCaseDescriptor> selectedDescriptors = FilterDescriptors(allDescriptors, effectiveOptions);

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
                if (!CompileAndLoadTestGlue(outputDir, compileResult, compileOptions, allFixturePlans, allDiagnostics, testLoadContext))
                {
                    return BuildCompileFailureResult(
                        fullProjectRoot,
                        effectiveOptions,
                        runStopwatch.Elapsed.TotalSeconds,
                        testAssemblies,
                        allDiagnostics);
                }
            }

            List<TestCaseResult> results = new List<TestCaseResult>();
            bool wasCancelled = false;
            int completedCount = 0;

            for (int i = 0; i < selectedDescriptors.Count; i++)
            {
                if (effectiveOptions.CancellationToken.IsCancellationRequested
                    || (effectiveOptions.MaxTestsToRun > 0 && completedCount >= effectiveOptions.MaxTestsToRun))
                {
                    wasCancelled = true;
                    break;
                }

                DiscoveredTestCaseDescriptor descriptor = selectedDescriptors[i];
                TestCaseResult caseResult = ExecuteSingleTestCase(descriptor, effectiveOptions);
                results.Add(caseResult);
                completedCount++;
                effectiveOptions.OnTestCompleted?.Invoke(caseResult);
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

        private static TestCaseResult ExecuteSingleTestCase(
            DiscoveredTestCaseDescriptor descriptor,
            TestRunOptions options)
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
            UnityEngine.Object[] preExistingObjects = UnityEngine.Object.FindObjectsOfType<UnityEngine.Object>(true);
            HashSet<int> preExistingIds = new HashSet<int>();
            for (int i = 0; i < preExistingObjects.Length; i++)
            {
                if (preExistingObjects[i] is not null)
                {
                    preExistingIds.Add(preExistingObjects[i].GetInstanceID());
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
                    catch (Exception) when (true)
                    {
                        playHost = null;
                    }

                    SceneManager.CreateScene("UnitasPlayModeTestScene_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
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
                CoroutineScheduler.StopAll();
                InvokeScheduler.StopAll();
                if (playHost != null)
                {
                    try
                    {
                        playHost.Dispose();
                    }
                    catch (Exception) when (true)
                    {
                    }
                }
            }

            IReadOnlyList<string> logProblems = LogAssert.Verify();
            if (string.IsNullOrEmpty(failureMessage) && logProblems.Count > 0)
            {
                failureMessage = string.Join("; ", logProblems);
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
                        StepSingleFrame(playHost, delta, ref consumedLogCount, capturedLogs);
                        frames++;
                        if (frames >= maxFrames)
                        {
                            throw new AssertionException("UnityTest CustomYieldInstruction exceeded frame budget.");
                        }
                    }

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
                LogAssert.Record(logType, entry.Message);
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

                if (!string.IsNullOrEmpty(options.AssemblyFilter)
                    && !string.Equals(d.AssemblyName, options.AssemblyFilter, StringComparison.OrdinalIgnoreCase))
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
                    if (!d.FullName.Contains(options.TestNameFilter, StringComparison.OrdinalIgnoreCase)
                        && !d.DisplayName.Contains(options.TestNameFilter, StringComparison.OrdinalIgnoreCase))
                    {
                        continue;
                    }
                }

                selected.Add(d);
            }

            return selected;
        }

        private static bool CompileAndLoadTestGlue(
            string outputDir,
            ScriptCompilationResult compileResult,
            ScriptCompilationOptions baseOptions,
            List<FixtureGenerationPlan> fixturePlans,
            List<ScriptDiagnostic> diagnostics,
            AssemblyLoadContext loadContext)
        {
            string glueSource = TestDiscoveryAndGlueGenerator.GenerateTestRunnerGlueSource(fixturePlans);
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

                using FileStream testGlueStream = File.OpenRead(targetGlueDll);
                var loaded = loadContext.LoadFromStream(testGlueStream);
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
