using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    /// <summary>
    /// Resolves a requested scene name or build index to a loadable scene path and exposes the build index list.
    /// This is the bounded substitute for reading the editor build settings asset: parsing that asset is out of
    /// scope, so the default provider builds its list from the scene files that sit next to the loaded scene.
    /// </summary>
    public interface ISceneProvider
    {
        /// <summary>Scene paths in build index order.</summary>
        IReadOnlyList<string> ScenePaths { get; }

        bool TryResolveScenePath(string sceneName, out string scenePath);

        bool TryResolveScenePathByBuildIndex(int buildIndex, out string scenePath);
    }

    /// <summary>Performs the scene operations on the native runtime.</summary>
    public interface ISceneBackend
    {
        void LoadScene(string scenePath, LoadSceneMode mode);

        void UnloadScene(string sceneName);

        bool SetActiveScene(int index);

        int ActiveSceneIndex { get; }

        void DontDestroyOnLoad(ulong entity);
    }

    public sealed class NativeSceneBackend : ISceneBackend
    {
        private readonly ulong _runtimeHandle;

        public NativeSceneBackend(ulong runtimeHandle)
        {
            if (runtimeHandle == 0)
            {
                throw new ArgumentException("Runtime handle must be non-zero.", nameof(runtimeHandle));
            }

            _runtimeHandle = runtimeHandle;
        }

        public void LoadScene(string scenePath, LoadSceneMode mode)
        {
            if (!TryWriteToPathBuffer(scenePath, out IntPtr buffer, out ulong length))
            {
                throw new SceneManagementException("Failed to allocate the native path buffer.");
            }

            int result = RuntimeInterop.unitas_runtime_load_scene_mode(_runtimeHandle, buffer, length, (uint)mode);
            if (result != 0)
            {
                throw new SceneManagementException(
                    "Failed to load scene '" + scenePath + "' (error code " + result + "): " + ReadErrorMessage());
            }
        }

        public void UnloadScene(string sceneName)
        {
            if (!TryWriteToPathBuffer(sceneName, out IntPtr buffer, out ulong length))
            {
                throw new SceneManagementException("Failed to allocate the native path buffer.");
            }

            int result = RuntimeInterop.unitas_runtime_unload_scene(_runtimeHandle, buffer, length);
            if (result != 0)
            {
                throw new SceneManagementException(
                    "Failed to unload scene '" + sceneName + "' (error code " + result + "): " + ReadErrorMessage());
            }
        }

        public bool SetActiveScene(int index)
        {
            if (index < 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_set_active_scene(_runtimeHandle, (uint)index) >= 0;
        }

        public int ActiveSceneIndex => RuntimeInterop.unitas_runtime_active_scene_index(_runtimeHandle);

        public void DontDestroyOnLoad(ulong entity)
        {
            if (entity == 0)
            {
                return;
            }

            RuntimeInterop.unitas_runtime_dont_destroy_on_load(_runtimeHandle, entity);
        }

        private bool TryWriteToPathBuffer(string text, out IntPtr buffer, out ulong length)
        {
            buffer = IntPtr.Zero;
            length = 0;
            byte[] utf8Bytes = Encoding.UTF8.GetBytes(text);
            buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)utf8Bytes.Length);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            Marshal.Copy(utf8Bytes, 0, buffer, utf8Bytes.Length);
            length = (ulong)utf8Bytes.Length;
            return true;
        }

        private string ReadErrorMessage()
        {
            IntPtr errorPtr = RuntimeInterop.unitas_runtime_error_message(_runtimeHandle);
            if (errorPtr == IntPtr.Zero)
            {
                return "no error detail";
            }

            return Marshal.PtrToStringUTF8(errorPtr) ?? "no error detail";
        }
    }

    /// <summary>
    /// Default provider: the build index list is the set of scene files that sit next to the loaded scene.
    /// Scene names are the file stems, matching the native scene naming rule.
    /// </summary>
    public sealed class LoadedSceneFileProvider : ISceneProvider
    {
        private readonly List<string> _scenePaths = new List<string>();
        private string? _rootDirectory;
        private bool _scanned;

        public IReadOnlyList<string> ScenePaths
        {
            get
            {
                EnsureScenes();
                return _scenePaths;
            }
        }

        public void SetSceneRoot(string loadedScenePath)
        {
            _rootDirectory = null;
            if (!string.IsNullOrEmpty(loadedScenePath))
            {
                string fullPath = Path.GetFullPath(loadedScenePath);
                _rootDirectory = Path.GetDirectoryName(fullPath);
            }

            Refresh();
        }

        public bool TryResolveScenePath(string sceneName, out string scenePath)
        {
            EnsureScenes();
            for (int index = 0; index < _scenePaths.Count; index++)
            {
                string candidate = _scenePaths[index];
                if (string.Equals(Path.GetFileNameWithoutExtension(candidate), sceneName, StringComparison.Ordinal))
                {
                    scenePath = candidate;
                    return true;
                }
            }

            scenePath = string.Empty;
            return false;
        }

        public bool TryResolveScenePathByBuildIndex(int buildIndex, out string scenePath)
        {
            EnsureScenes();
            if (buildIndex < 0 || buildIndex >= _scenePaths.Count)
            {
                scenePath = string.Empty;
                return false;
            }

            scenePath = _scenePaths[buildIndex];
            return true;
        }

        private void EnsureScenes()
        {
            if (!_scanned)
            {
                Refresh();
            }
        }

        private void Refresh()
        {
            _scanned = true;
            _scenePaths.Clear();
            string directory = _rootDirectory ?? Directory.GetCurrentDirectory();
            if (!Directory.Exists(directory))
            {
                return;
            }

            string[] files = Directory.GetFiles(directory, "*.unity", SearchOption.AllDirectories);
            Array.Sort(files, StringComparer.Ordinal);
            for (int index = 0; index < files.Length; index++)
            {
                _scenePaths.Add(files[index]);
            }
        }
    }
}
