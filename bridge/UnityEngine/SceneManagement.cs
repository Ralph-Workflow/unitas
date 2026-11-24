using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;

[assembly: InternalsVisibleTo("Unitas.Bridge")]
[assembly: InternalsVisibleTo("Unitas.Bridge.Tests")]

namespace UnityEngine.SceneManagement
{
    public enum LoadSceneMode
    {
        Single = 0,
        Additive = 1,
    }

    /// <summary>Recorded for Unity compatibility: the runtime keeps one global physics world, so the mode is not applied.</summary>
    public enum LocalPhysicsMode
    {
        PhysicsMode2D = 0,
        PhysicsMode3D = 1,
        PhysicsMode2D3D = 2,
    }

    /// <summary>Load parameters; a non-empty <c>loadScene</c> overrides the requested scene name.</summary>
    public struct LoadSceneParameters
    {
        public string loadScene;

        public LoadSceneMode loadSceneMode;

        public LocalPhysicsMode localPhysicsMode;
    }

    public class SceneManagementException : InvalidOperationException
    {
        public SceneManagementException(string message) : base(message)
        {
        }

        public SceneManagementException(string message, Exception innerException) : base(message, innerException)
        {
        }
    }

    public struct Scene : IEquatable<Scene>
    {
        public Scene(string name, string path, int buildIndex, bool isLoaded, int rootCount, int handle)
        {
            this.name = name ?? string.Empty;
            this.path = path ?? string.Empty;
            this.buildIndex = buildIndex;
            this.isLoaded = isLoaded;
            this.rootCount = rootCount;
            this.handle = handle;
        }

        public string name { get; }

        public string path { get; }

        public int buildIndex { get; }

        public bool isLoaded { get; }

        /// <summary>Root object count; the native scene ABI does not report root counts yet, so this stays 0.</summary>
        public int rootCount { get; }

        /// <summary>Managed scene identity assigned by the bridge; the native scene ABI does not report handles yet.</summary>
        public int handle { get; }

        public bool IsValid() => handle != 0;

        public bool Equals(Scene other) => handle == other.handle;

        public override bool Equals(object? other) => other is Scene scene && Equals(scene);

        public override int GetHashCode() => handle;

        public override string ToString() => name;

        public static bool operator ==(Scene left, Scene right) => left.Equals(right);

        public static bool operator !=(Scene left, Scene right) => !left.Equals(right);
    }

    public sealed class AsyncOperation
    {
        private bool _isDone;

        /// <summary>1 once the operation completed, 0 before that.</summary>
        public float progress => _isDone ? 1f : 0f;

        public bool isDone => _isDone;

        /// <summary>
        /// Accepted for Unity compatibility. Scene activation is boundary-synchronous here: the scene is
        /// already active when the load call returns, so this flag does not delay activation.
        /// </summary>
        public bool allowSceneActivation { get; set; } = true;

        public event Action<AsyncOperation>? completed;

        internal void MarkComplete()
        {
            _isDone = true;
            Action<AsyncOperation>? handlers = completed;
            if (handlers == null)
            {
                return;
            }

            try
            {
                handlers.Invoke(this);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }
    }

    /// <summary>
    /// Scene operations the managed facade needs from the runtime host. Implemented by the bridge host;
    /// the managed facade never calls the native runtime itself.
    /// </summary>
    public interface ISceneHost
    {
        bool TryResolveScenePath(string sceneName, out string scenePath);

        int SceneCount { get; }

        Scene GetSceneAt(int index);

        Scene GetSceneByName(string sceneName);

        Scene GetSceneByBuildIndex(int buildIndex);

        Scene GetActiveScene();

        bool SetActiveScene(Scene scene);

        Scene LoadScene(string scenePath, LoadSceneMode mode);

        void UnloadScene(string sceneName);

        void DontDestroyOnLoad(ulong entity);
    }

    public static class SceneManager
    {
        private static readonly List<AsyncOperation> PendingOperations = new List<AsyncOperation>();

        public static ISceneHost? Host { get; set; }

        /// <summary>Raised after the load call returned; the load mode is the mode the scene was loaded with.</summary>
        public static event Action<Scene, LoadSceneMode>? sceneLoaded;

        /// <summary>Raised after the unload call returned; native teardown callbacks run inside that call.</summary>
        public static event Action<Scene>? sceneUnloaded;

        public static int sceneCount => Host?.SceneCount ?? 0;

        public static Scene GetActiveScene() => Host?.GetActiveScene() ?? default;

        public static bool SetActiveScene(Scene scene) => Host?.SetActiveScene(scene) ?? false;

        public static Scene GetSceneByName(string sceneName)
        {
            if (string.IsNullOrEmpty(sceneName))
            {
                return default;
            }

            ISceneHost? host = Host;
            return host == null ? default : host.GetSceneByName(sceneName);
        }

        public static Scene GetSceneByBuildIndex(int buildIndex)
        {
            ISceneHost? host = Host;
            return host == null ? default : host.GetSceneByBuildIndex(buildIndex);
        }

        public static Scene GetSceneAt(int index)
        {
            ISceneHost? host = Host;
            return host == null ? default : host.GetSceneAt(index);
        }

        public static Scene LoadScene(string sceneName) => LoadScene(sceneName, LoadSceneMode.Single);

        public static Scene LoadScene(string sceneName, LoadSceneMode mode)
        {
            if (sceneName == null)
            {
                throw new ArgumentNullException(nameof(sceneName));
            }

            ISceneHost host = RequireHost();
            if (!host.TryResolveScenePath(sceneName, out string scenePath))
            {
                scenePath = sceneName;
            }

            return host.LoadScene(scenePath, mode);
        }

        /// <summary>
        /// Loads the scene named by <paramref name="parameters"/>; a non-empty
        /// <c>parameters.loadScene</c> overrides <paramref name="sceneName"/>.
        /// </summary>
        public static Scene LoadScene(string sceneName, LoadSceneParameters parameters)
        {
            string requested = string.IsNullOrEmpty(parameters.loadScene) ? sceneName : parameters.loadScene;
            return LoadScene(requested, parameters.loadSceneMode);
        }

        public static bool UnloadScene(string sceneName)
        {
            if (sceneName == null)
            {
                throw new ArgumentNullException(nameof(sceneName));
            }

            ISceneHost host = RequireHost();
            host.UnloadScene(sceneName);
            return true;
        }

        /// <summary>
        /// Loads the scene inside the call; the returned operation completes on the next tick boundary.
        /// The load itself is synchronous, so a failed load surfaces as a <see cref="SceneManagementException"/>
        /// from this call instead of through the operation.
        /// </summary>
        public static AsyncOperation LoadSceneAsync(string sceneName) => LoadSceneAsync(sceneName, LoadSceneMode.Single);

        /// <summary>
        /// Loads the scene inside the call; the returned operation completes on the next tick boundary.
        /// The load itself is synchronous, so a failed load surfaces as a <see cref="SceneManagementException"/>
        /// from this call instead of through the operation.
        /// </summary>
        public static AsyncOperation LoadSceneAsync(string sceneName, LoadSceneMode mode)
        {
            AsyncOperation operation = new AsyncOperation();
            LoadScene(sceneName, mode);
            PendingOperations.Add(operation);
            return operation;
        }

        /// <summary>
        /// Unloads the scene inside the call; the returned operation completes on the next tick boundary.
        /// </summary>
        public static AsyncOperation UnloadSceneAsync(string sceneName)
        {
            AsyncOperation operation = new AsyncOperation();
            UnloadScene(sceneName);
            PendingOperations.Add(operation);
            return operation;
        }

        internal static void RaiseSceneLoaded(Scene scene, LoadSceneMode mode)
        {
            Action<Scene, LoadSceneMode>? handlers = sceneLoaded;
            if (handlers == null)
            {
                return;
            }

            try
            {
                handlers.Invoke(scene, mode);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        internal static void RaiseSceneUnloaded(Scene scene)
        {
            Action<Scene>? handlers = sceneUnloaded;
            if (handlers == null)
            {
                return;
            }

            try
            {
                handlers.Invoke(scene);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        internal static void PumpAsyncOperations()
        {
            if (PendingOperations.Count == 0)
            {
                return;
            }

            List<AsyncOperation> pending = new List<AsyncOperation>(PendingOperations);
            PendingOperations.Clear();
            for (int index = 0; index < pending.Count; index++)
            {
                pending[index].MarkComplete();
            }
        }

        private static ISceneHost RequireHost()
        {
            ISceneHost? host = Host;
            if (host == null)
            {
                throw new SceneManagementException("No scene host is configured.");
            }

            return host;
        }
    }
}
