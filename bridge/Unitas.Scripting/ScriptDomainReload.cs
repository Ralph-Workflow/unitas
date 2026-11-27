using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.Loader;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Scripting
{
    public static class ScriptDomainReload
    {
        public sealed class Request
        {
            public string ProjectRoot { get; set; } = string.Empty;

            public ScriptCompilationOptions? Options { get; set; }

            public IReadOnlyList<MonoBehaviour> LiveInstances { get; set; } = Array.Empty<MonoBehaviour>();
        }

        public sealed class Result
        {
            public Result(
                bool succeeded,
                bool compileSucceeded,
                ActiveAssemblyState activeState,
                IReadOnlyList<ScriptDiagnostic> diagnostics,
                AssemblyLoadContext? context,
                WeakReference? replacedContext,
                IReadOnlyList<MonoBehaviour> recreatedInstances,
                string? error)
            {
                Succeeded = succeeded;
                CompileSucceeded = compileSucceeded;
                ActiveState = activeState;
                Diagnostics = diagnostics;
                Context = context;
                ReplacedContext = replacedContext;
                RecreatedInstances = recreatedInstances;
                Error = error;
            }

            public bool Succeeded { get; }

            public bool CompileSucceeded { get; }

            public ActiveAssemblyState ActiveState { get; }

            public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; }

            public AssemblyLoadContext? Context { get; }

            public WeakReference? ReplacedContext { get; }

            public IReadOnlyList<MonoBehaviour> RecreatedInstances { get; }

            public string? Error { get; }
        }

        private sealed class InstanceState
        {
            public InstanceState(ulong guidHi, ulong guidLo, string? json)
            {
                GuidHi = guidHi;
                GuidLo = guidLo;
                Json = json;
            }

            public ulong GuidHi { get; }

            public ulong GuidLo { get; }

            public string? Json { get; }
        }

        private static AssemblyLoadContext? s_currentContext;

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        public static Result Reload(Request request)
        {
            ArgumentNullException.ThrowIfNull(request);
            if (string.IsNullOrEmpty(request.ProjectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(request));
            }

            ScriptCompilationResult compilation = ScriptCompilation.CompileProject(
                request.ProjectRoot,
                CopyOptions(request.Options));

            if (!compilation.Succeeded)
            {
                return new Result(
                    false,
                    false,
                    compilation.ActiveState,
                    compilation.Diagnostics,
                    s_currentContext,
                    null,
                    Array.Empty<MonoBehaviour>(),
                    "The script compile failed; the last valid assemblies remain in effect.");
            }

            string gluePath = compilation.GeneratedGlueAssemblyPath ?? string.Empty;
            if (gluePath.Length == 0 || !File.Exists(gluePath))
            {
                return new Result(
                    false,
                    true,
                    compilation.ActiveState,
                    compilation.Diagnostics,
                    s_currentContext,
                    null,
                    Array.Empty<MonoBehaviour>(),
                    "The compile produced no generated glue assembly.");
            }

            IReadOnlyList<MonoBehaviour> liveInstances = request.LiveInstances ?? Array.Empty<MonoBehaviour>();
            List<InstanceState> states = CaptureStates(liveInstances);
            WeakReference? replacedReference = UnloadReplacedContext(liveInstances);

            AssemblyLoadContext context = ScriptGlueRegistry.CreateCollectibleContext("UnitasGame", compilation.OutputDirectory);

            try
            {
                for (int i = 0; i < compilation.Assemblies.Count; i++)
                {
                    CompiledAssemblyRecord record = compilation.Assemblies[i];
                    if (record.IsExcluded || !File.Exists(record.OutputPath))
                    {
                        continue;
                    }

                    ScriptGlueRegistry.LoadAssemblyIntoContext(record.OutputPath, context);
                }

                ScriptGlueRegistry.LoadGlueAssembly(gluePath, context);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                context.Unload();
                return new Result(
                    false,
                    true,
                    compilation.ActiveState,
                    compilation.Diagnostics,
                    s_currentContext,
                    null,
                    Array.Empty<MonoBehaviour>(),
                    "Loading the recompiled assemblies failed: " + ex.Message);
            }

            List<MonoBehaviour> recreated = CreateReplacements(states);
            s_currentContext = context;

            return new Result(
                true,
                true,
                compilation.ActiveState,
                compilation.Diagnostics,
                context,
                replacedReference,
                recreated,
                null);
        }

        public static AssemblyLoadContext? CurrentContext => s_currentContext;

        private static ScriptCompilationOptions CopyOptions(ScriptCompilationOptions? source)
        {
            ScriptCompilationOptions copy = new ScriptCompilationOptions();
            if (source != null)
            {
                copy.IsEditor = source.IsEditor;
                copy.IsDevelopmentBuild = source.IsDevelopmentBuild;
                copy.TargetPlatform = source.TargetPlatform;
                copy.TargetArchitecture = source.TargetArchitecture;
                copy.IsProjectTrusted = source.IsProjectTrusted;
                copy.OutputDirectory = source.OutputDirectory;
                copy.UnityVersion = source.UnityVersion;
                copy.EmitGeneratedGlue = source.EmitGeneratedGlue;
                copy.AdditionalDefines.AddRange(source.AdditionalDefines);
                copy.AdditionalReferencePaths.AddRange(source.AdditionalReferencePaths);
                foreach (KeyValuePair<string, string> package in source.PackageVersions)
                {
                    copy.PackageVersions[package.Key] = package.Value;
                }
            }

            copy.AutoLoadGeneratedGlue = false;
            return copy;
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static WeakReference? UnloadReplacedContext(IReadOnlyList<MonoBehaviour> liveInstances)
        {
            AssemblyLoadContext? replacedContext = ResolveReplacedContext(liveInstances);
            if (replacedContext == null)
            {
                return null;
            }

            s_currentContext = null;
            ScriptGlueRegistry.ReleaseLoadContext(replacedContext);
            EditorMethodRegistry.ReleaseLoadContext(replacedContext);
            ScriptRuntimeInitializeRegistry.ReleaseLoadContext(replacedContext);
            ReleaseInstances(liveInstances);
            WeakReference replacedReference = new WeakReference(replacedContext, trackResurrection: false);
            replacedContext.Unload();
            return replacedReference;
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static AssemblyLoadContext? ResolveReplacedContext(IReadOnlyList<MonoBehaviour> liveInstances)
        {
            for (int i = 0; i < liveInstances.Count; i++)
            {
                AssemblyLoadContext? candidate = ScriptGlueRegistry.GetInstanceLoadContext(liveInstances[i]);
                if (candidate != null && candidate != AssemblyLoadContext.Default)
                {
                    return candidate;
                }
            }

            return s_currentContext;
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static List<InstanceState> CaptureStates(IReadOnlyList<MonoBehaviour> liveInstances)
        {
            List<InstanceState> states = new List<InstanceState>();
            for (int i = 0; i < liveInstances.Count; i++)
            {
                MonoBehaviour instance = liveInstances[i];
                if (!ScriptGlueRegistry.TryGetRegisteredGuid(instance, out ulong guidHi, out ulong guidLo))
                {
                    continue;
                }

                ScriptGlueRegistry.TryCaptureState(instance, out string? json);
                states.Add(new InstanceState(guidHi, guidLo, json));
            }

            return states;
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static List<MonoBehaviour> CreateReplacements(IReadOnlyList<InstanceState> states)
        {
            List<MonoBehaviour> recreated = new List<MonoBehaviour>();
            for (int i = 0; i < states.Count; i++)
            {
                InstanceState state = states[i];
                if (!ScriptGlueRegistry.TryCreate(state.GuidHi, state.GuidLo, out MonoBehaviour? replacement, out _))
                {
                    continue;
                }

                if (replacement == null)
                {
                    continue;
                }

                if (state.Json != null)
                {
                    ScriptGlueRegistry.TryRestoreState(replacement, state.Json);
                }

                recreated.Add(replacement);
            }

            return recreated;
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static void ReleaseInstances(IReadOnlyList<MonoBehaviour> liveInstances)
        {
            for (int i = 0; i < liveInstances.Count; i++)
            {
                ScriptGlueRegistry.ReleaseInstance(liveInstances[i]);
            }
        }
    }
}
