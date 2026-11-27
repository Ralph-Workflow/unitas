using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;

namespace Unitas.Bridge.Tests
{
    public static class DebugCompileTests
    {
        private const string ScriptSource = @"using UnityEngine;

namespace Unitas.Samples.DebugCompile
{
    public class ProbeBehaviour : MonoBehaviour
    {
        private int _counter;

        private void Start()
        {
            _counter = 0;
            _counter = _counter + 1;
        }
    }
}
";

        public static void Run()
        {
            TestDebugCompileProducesInstrumentedAssemblies();
            TestDefaultCompileKeepsTheOutputFreeOfTheDebuggingAssembly();
        }

        private static void TestDebugCompileProducesInstrumentedAssemblies()
        {
            string tempProject = CreateTempProject("debug");
            try
            {
                string outputDirectory = Path.Combine(tempProject, "out");
                ScriptCompilationResult result = ScriptCompilation.CompileProject(
                    tempProject,
                    CreateOptions(outputDirectory),
                    true);

                Check.Equal("debug compile: the instrumented compile succeeds", true, result.Succeeded);
                CompiledAssemblyRecord? record = result.FindAssembly("Assembly-CSharp");
                Check.That(
                    "debug compile: Assembly-CSharp is produced",
                    record != null && File.Exists(record.OutputPath));
                Check.That(
                    "debug compile: Assembly-CSharp references Unitas.Debugging.dll",
                    record != null && ContainsFileName(record.References, "Unitas.Debugging.dll"));
                Check.That(
                    "debug compile: Unitas.Debugging.dll is copied into the output directory",
                    File.Exists(Path.Combine(outputDirectory, "Unitas.Debugging.dll")));
                Check.That(
                    "debug compile: the produced assembly carries the debug probes",
                    record != null && ContainsMetadataText(record.OutputPath, "DebugAgent"));
                Check.That(
                    "debug compile: the produced assembly references Unitas.Debugging",
                    record != null && ContainsMetadataText(record.OutputPath, "Unitas.Debugging"));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestDefaultCompileKeepsTheOutputFreeOfTheDebuggingAssembly()
        {
            string tempProject = CreateTempProject("default");
            try
            {
                string outputDirectory = Path.Combine(tempProject, "out");

                // A default compile that follows an instrumented compile into the same
                // output directory must drop the debugging assembly again.
                ScriptCompilation.CompileProject(tempProject, CreateOptions(outputDirectory), true);

                ScriptCompilationResult result = ScriptCompilation.CompileProject(
                    tempProject,
                    CreateOptions(outputDirectory));

                Check.Equal("default compile: the compile succeeds", true, result.Succeeded);
                CompiledAssemblyRecord? record = result.FindAssembly("Assembly-CSharp");
                Check.That(
                    "default compile: Assembly-CSharp is produced",
                    record != null && File.Exists(record.OutputPath));
                Check.That(
                    "default compile: Assembly-CSharp does not reference Unitas.Debugging.dll",
                    record != null && !ContainsFileName(record.References, "Unitas.Debugging.dll"));
                Check.That(
                    "default compile: no Unitas.Debugging.dll in the output directory",
                    !File.Exists(Path.Combine(outputDirectory, "Unitas.Debugging.dll")));
                Check.That(
                    "default compile: the produced assembly carries no debug probes",
                    record != null && !ContainsMetadataText(record.OutputPath, "DebugAgent"));
                Check.That(
                    "default compile: the produced assembly does not reference Unitas.Debugging",
                    record != null && !ContainsMetadataText(record.OutputPath, "Unitas.Debugging"));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static ScriptCompilationOptions CreateOptions(string outputDirectory)
        {
            return new ScriptCompilationOptions
            {
                IsEditor = false,
                TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                TargetArchitecture = "x86_64",
                OutputDirectory = outputDirectory
            };
        }

        private static string CreateTempProject(string label)
        {
            string tempDirectory = Path.Combine(
                Path.GetTempPath(),
                "unitas_debug_compile_test_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            string scriptsDirectory = Path.Combine(tempDirectory, "Assets", "Scripts");
            Directory.CreateDirectory(scriptsDirectory);
            File.WriteAllText(Path.Combine(scriptsDirectory, "ProbeBehaviour.cs"), ScriptSource);
            return tempDirectory;
        }

        private static bool ContainsFileName(IReadOnlyList<string> paths, string fileName)
        {
            for (int i = 0; i < paths.Count; i++)
            {
                if (string.Equals(Path.GetFileName(paths[i]), fileName, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        /// <summary>
        /// Looks for a name in the assembly's metadata; the debug probes and the
        /// debugging assembly reference show up there as plain metadata strings.
        /// </summary>
        private static bool ContainsMetadataText(string assemblyPath, string text)
        {
            if (!File.Exists(assemblyPath))
            {
                return false;
            }

            byte[] bytes = File.ReadAllBytes(assemblyPath);
            byte[] needle = Encoding.ASCII.GetBytes(text);
            for (int i = 0; i + needle.Length <= bytes.Length; i++)
            {
                bool matches = true;
                for (int j = 0; j < needle.Length; j++)
                {
                    if (bytes[i + j] != needle[j])
                    {
                        matches = false;
                        break;
                    }
                }

                if (matches)
                {
                    return true;
                }
            }

            return false;
        }

        private static void DeleteDirectoryIfExists(string directory)
        {
            if (Directory.Exists(directory))
            {
                try
                {
                    Directory.Delete(directory, true);
                }
                catch (IOException)
                {
                }
            }
        }
    }
}
