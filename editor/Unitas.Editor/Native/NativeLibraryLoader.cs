using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

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

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate IntPtr StringBufferDelegate(nuint capacity);

        private readonly DescribeToolsDelegate describeTools;
        private readonly InvokeToolDelegate invokeTool;
        private readonly FreeStringDelegate freeString;
        private readonly StringBufferDelegate stringBuffer;

        private NativeLibraryLoader(
            string libraryPath,
            DescribeToolsDelegate describeTools,
            InvokeToolDelegate invokeTool,
            FreeStringDelegate freeString,
            StringBufferDelegate stringBuffer)
        {
            LibraryPath = libraryPath;
            this.describeTools = describeTools;
            this.invokeTool = invokeTool;
            this.freeString = freeString;
            this.stringBuffer = stringBuffer;
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
                || !NativeLibrary.TryGetExport(loadedHandle, "unitas_editor_string_free", out IntPtr freeAddr)
                || !NativeLibrary.TryGetExport(loadedHandle, "unitas_editor_string_buffer", out IntPtr bufferAddr))
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
            StringBufferDelegate buffer =
                Marshal.GetDelegateForFunctionPointer<StringBufferDelegate>(bufferAddr);

            loader = new NativeLibraryLoader(resolvedPath, describe, invoke, free, buffer);
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

            IntPtr nameBuffer = AllocateStringBuffer(name);
            IntPtr argsBuffer = AllocateStringBuffer(argsJson);
            try
            {
                IntPtr resultPtr = invokeTool(nameBuffer, argsBuffer);
                return ReadAndFreeOwnedString(resultPtr, "unitas_editor_tool_invoke");
            }
            finally
            {
                freeString(nameBuffer);
                freeString(argsBuffer);
            }
        }

        private IntPtr AllocateStringBuffer(string value)
        {
            byte[] payload = Encoding.UTF8.GetBytes(value);
            IntPtr buffer = stringBuffer((nuint)(payload.Length + 1));
            if (buffer == IntPtr.Zero)
            {
                throw new McpError.InvalidRequest("native string buffer allocation failed");
            }

            Marshal.Copy(payload, 0, buffer, payload.Length);
            return buffer;
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
