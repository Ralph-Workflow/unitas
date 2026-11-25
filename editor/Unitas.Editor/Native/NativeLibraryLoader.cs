using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;

namespace Unitas.Editor.Mcp
{
    public interface INativeLibrarySource
    {
        string LibraryPath { get; }

        string DescribeJson();

        string InvokeJson(string name, string argsJson);
    }

    public enum NativeLoadStatus
    {
        Loaded,
        LibraryNotFound,
        LoadFailed,
    }

    public sealed record NativeLoadOutcome(
        NativeLoadStatus Kind,
        string Status,
        string LibraryPath,
        int ToolCount,
        string Reason)
    {
        public bool IsLoaded => Kind == NativeLoadStatus.Loaded;

        public bool IsLibraryNotFound => Kind == NativeLoadStatus.LibraryNotFound;

        public bool IsLoadFailed => Kind == NativeLoadStatus.LoadFailed;

        public static NativeLoadOutcome Loaded(string libraryPath, int toolCount)
        {
            return new NativeLoadOutcome(
                NativeLoadStatus.Loaded,
                "loaded",
                libraryPath,
                toolCount,
                string.Empty);
        }

        public static NativeLoadOutcome LibraryNotFound(string reason)
        {
            return new NativeLoadOutcome(
                NativeLoadStatus.LibraryNotFound,
                "native-library-not-found",
                string.Empty,
                0,
                reason);
        }

        public static NativeLoadOutcome LoadFailed(string reason)
        {
            return new NativeLoadOutcome(
                NativeLoadStatus.LoadFailed,
                "native-library-load-failed",
                string.Empty,
                0,
                reason);
        }
    }

    public sealed class NativeLibraryLoader : INativeLibrarySource
    {
        public const string LibraryEnvironmentVariable = "UNITAS_EDITOR_ABI_LIBRARY";

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int InstallBuiltinToolsDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate IntPtr DescribeToolsDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate IntPtr InvokeToolDelegate(IntPtr name, IntPtr argsJson);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void FreeStringDelegate(IntPtr ptr);

        private readonly DescribeToolsDelegate describeTools;
        private readonly InvokeToolDelegate invokeTool;
        private readonly FreeStringDelegate freeString;

        private NativeLibraryLoader(
            string libraryPath,
            DescribeToolsDelegate describeTools,
            InvokeToolDelegate invokeTool,
            FreeStringDelegate freeString)
        {
            LibraryPath = libraryPath;
            this.describeTools = describeTools;
            this.invokeTool = invokeTool;
            this.freeString = freeString;
        }

        public string LibraryPath { get; }

        public static bool TryOpen(
            string? explicitPath,
            out NativeLibraryLoader? loader,
            out NativeLoadOutcome failureOutcome)
        {
            loader = null;
            IReadOnlyList<string> candidates = CandidatePaths(explicitPath);
            string? resolvedPath = null;
            foreach (string candidate in candidates)
            {
                if (!string.IsNullOrWhiteSpace(candidate) && File.Exists(candidate))
                {
                    resolvedPath = Path.GetFullPath(candidate);
                    break;
                }
            }

            if (resolvedPath == null)
            {
                string detail = string.IsNullOrWhiteSpace(explicitPath)
                    ? "no libunitas_editor_abi binary found in default search paths"
                    : "library file does not exist: " + explicitPath;
                failureOutcome = NativeLoadOutcome.LibraryNotFound(detail);
                return false;
            }

            if (!NativeLibrary.TryLoad(resolvedPath, out IntPtr loadedHandle) || loadedHandle == IntPtr.Zero)
            {
                failureOutcome = NativeLoadOutcome.LoadFailed("NativeLibrary.TryLoad failed for " + resolvedPath);
                return false;
            }

            if (!NativeLibrary.TryGetExport(loadedHandle, "unitas_editor_tools_describe", out IntPtr describeAddr)
                || !NativeLibrary.TryGetExport(loadedHandle, "unitas_editor_tool_invoke", out IntPtr invokeAddr)
                || !NativeLibrary.TryGetExport(loadedHandle, "unitas_editor_string_free", out IntPtr freeAddr))
            {
                NativeLibrary.Free(loadedHandle);
                failureOutcome = NativeLoadOutcome.LoadFailed(
                    "missing required unitas_editor_* FFI exports in " + resolvedPath);
                return false;
            }

            if (NativeLibrary.TryGetExport(loadedHandle, "unitas_editor_install_builtin_tools", out IntPtr installAddr))
            {
                InstallBuiltinToolsDelegate installBuiltin =
                    Marshal.GetDelegateForFunctionPointer<InstallBuiltinToolsDelegate>(installAddr);
                int rc = installBuiltin();
                if (rc != 0)
                {
                    NativeLibrary.Free(loadedHandle);
                    failureOutcome = NativeLoadOutcome.LoadFailed(
                        "unitas_editor_install_builtin_tools returned " + rc);
                    return false;
                }
            }

            DescribeToolsDelegate describe =
                Marshal.GetDelegateForFunctionPointer<DescribeToolsDelegate>(describeAddr);
            InvokeToolDelegate invoke =
                Marshal.GetDelegateForFunctionPointer<InvokeToolDelegate>(invokeAddr);
            FreeStringDelegate free =
                Marshal.GetDelegateForFunctionPointer<FreeStringDelegate>(freeAddr);

            loader = new NativeLibraryLoader(resolvedPath, describe, invoke, free);
            failureOutcome = NativeLoadOutcome.Loaded(resolvedPath, 0);
            return true;
        }

        public static IReadOnlyList<string> CandidatePaths(string? explicitPath = null)
        {
            if (!string.IsNullOrWhiteSpace(explicitPath))
            {
                return new[] { explicitPath };
            }

            List<string> candidates = new List<string>();
            string? envPath = Environment.GetEnvironmentVariable(LibraryEnvironmentVariable);
            if (!string.IsNullOrWhiteSpace(envPath))
            {
                candidates.Add(envPath);
            }

            string[] fileNames =
            {
                "libunitas_editor_abi.dylib",
                "libunitas_editor_abi.so",
                "unitas_editor_abi.dll",
            };

            string[] baseDirs =
            {
                Directory.GetCurrentDirectory(),
                AppContext.BaseDirectory,
            };

            foreach (string startDir in baseDirs)
            {
                if (string.IsNullOrWhiteSpace(startDir))
                {
                    continue;
                }

                DirectoryInfo? current = new DirectoryInfo(startDir);
                for (int depth = 0; depth < 7 && current != null; depth++)
                {
                    foreach (string fileName in fileNames)
                    {
                        candidates.Add(Path.Combine(current.FullName, "target", "debug", fileName));
                        candidates.Add(Path.Combine(current.FullName, "target", "release", fileName));
                        candidates.Add(Path.Combine(current.FullName, fileName));
                    }

                    current = current.Parent;
                }
            }

            return candidates;
        }

        public string DescribeJson()
        {
            IntPtr ptr = describeTools();
            return ReadAndFreeOwnedString(ptr, "unitas_editor_tools_describe");
        }

        public string InvokeJson(string name, string argsJson)
        {
            ArgumentNullException.ThrowIfNull(name);
            ArgumentNullException.ThrowIfNull(argsJson);

            IntPtr namePtr = Marshal.StringToCoTaskMemUTF8(name);
            IntPtr argsPtr = Marshal.StringToCoTaskMemUTF8(argsJson);
            try
            {
                IntPtr resultPtr = invokeTool(namePtr, argsPtr);
                return ReadAndFreeOwnedString(resultPtr, "unitas_editor_tool_invoke");
            }
            finally
            {
                Marshal.FreeCoTaskMem(argsPtr);
                Marshal.FreeCoTaskMem(namePtr);
            }
        }

        private string ReadAndFreeOwnedString(IntPtr ptr, string exportName)
        {
            if (ptr == IntPtr.Zero)
            {
                throw new McpError.InvalidRequest(exportName + " returned a null pointer");
            }

            try
            {
                return Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
            }
            finally
            {
                freeString(ptr);
            }
        }
    }
}
