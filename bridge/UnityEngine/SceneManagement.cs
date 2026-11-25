using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;

[assembly: InternalsVisibleTo("Unitas.Bridge")]
[assembly: InternalsVisibleTo("Unitas.Bridge.Tests")]

namespace UnityEngine
{
    public class AsyncOperation : YieldInstruction
    {
        private bool _isDone;
        private float _progress;
        private bool _allowSceneActivation = true;
        private Action? _activationAction;
        private bool _activationPending;
        private bool _activationExecuted;
        private Action? _deferredCompletionCallback;
        private Action<AsyncOperation>? _completed;

        public float progress
        {
            get => _isDone ? 1f : _progress;
            internal set => _progress = Math.Clamp(value, 0f, 1f);
        }

        public bool isDone => _isDone;

        public int priority { get; set; }

        public bool allowSceneActivation
        {
            get => _allowSceneActivation;
            set
            {
                _allowSceneActivation = value;
                if (!value && !_isDone)
                {
                    _progress = 0.9f;
                }
            }
        }

        internal bool HasPendingActivation => _activationPending && !_activationExecuted;

        public event Action<AsyncOperation>? completed
        {
            add
            {
                if (value == null)
                {
                    return;
                }

                if (_isDone)
                {
                    try
                    {
                        value(this);
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex);
                    }
                    return;
                }

                _completed += value;
            }
            remove
            {
                if (value != null)
                {
                    _completed -= value;
                }
            }
        }

        internal void SetDeferredActivation(Action activationAction)
        {
            _activationAction = activationAction;
            _activationPending = true;
            _activationExecuted = false;
            _progress = 0.9f;
        }

        internal void SetDeferredSceneLoadedEvent(Action callback)
        {
            _deferredCompletionCallback = callback;
        }

        internal void ClearDeferredSceneLoadedEvent()
        {
            _deferredCompletionCallback = null;
        }

        internal bool TryStep()
        {
            if (_isDone)
            {
                return true;
            }

            if (_activationPending && !_activationExecuted)
            {
                if (!_allowSceneActivation)
                {
                    _progress = 0.9f;
                    return false;
                }

                _activationExecuted = true;
                Action? action = _activationAction;
                _activationAction = null;
                action?.Invoke();
            }

            if (!_allowSceneActivation)
            {
                _progress = 0.9f;
                return false;
            }

            Action? deferredCallback = _deferredCompletionCallback;
            _deferredCompletionCallback = null;
            deferredCallback?.Invoke();

            MarkComplete();
            return true;
        }

        internal void MarkComplete()
        {
            if (_isDone)
            {
                return;
            }

            _progress = 1f;
            _isDone = true;
            Action<AsyncOperation>? handlers = _completed;
            _completed = null;
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
}

namespace UnityEngine.SceneManagement
{
    public enum LoadSceneMode
    {
        Single = 0,
        Additive = 1,
    }

    public enum LocalPhysicsMode
    {
        None = 0,
        PhysicsMode2D = 0,
        Physics2D = 1,
        PhysicsMode3D = 1,
        PhysicsMode2D3D = 2,
        Physics3D = 2,
    }

    public struct LoadSceneParameters
    {
        public string loadScene;

        public LoadSceneMode loadSceneMode;

        public LocalPhysicsMode localPhysicsMode;

        public LoadSceneParameters(LoadSceneMode mode)
        {
            loadScene = string.Empty;
            loadSceneMode = mode;
            localPhysicsMode = LocalPhysicsMode.None;
        }

        public LoadSceneParameters(LoadSceneMode mode, LocalPhysicsMode physicsMode)
        {
            loadScene = string.Empty;
            loadSceneMode = mode;
            localPhysicsMode = physicsMode;
        }
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

        public int rootCount { get; }

        public int handle { get; }

        public bool IsValid() => handle != 0;

        public GameObject[] GetRootGameObjects()
        {
            if (!IsValid())
            {
                return Array.Empty<GameObject>();
            }

            List<UnityEngine.Object> snapshot = UnityEngine.Object.SnapshotLiveObjects();
            List<GameObject> roots = new List<GameObject>();
            for (int i = 0; i < snapshot.Count; i++)
            {
                if (snapshot[i] is GameObject go
                    && !go.IsDestroyed
                    && go.scene.handle == handle
                    && go.transform.parent == null)
                {
                    roots.Add(go);
                }
            }

            return roots.ToArray();
        }

        public bool Equals(Scene other) => handle == other.handle;

        public override bool Equals(object? other) => other is Scene scene && Equals(scene);

        public override int GetHashCode() => handle;

        public override string ToString() => name;

        public static bool operator ==(Scene left, Scene right) => left.Equals(right);

        public static bool operator !=(Scene left, Scene right) => !left.Equals(right);
    }

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
        private static readonly List<Action> DeferredCallbackQueue = new List<Action>();
        private static readonly List<string> BuildScenePaths = new List<string>();
        private static int _nextRuntimeSceneHandle = 10000;
        private static int _callbackDepth;
        private static AsyncOperation? _capturingAsyncLoad;

        public static ISceneHost? Host { get; set; }

        public static bool EnforceBuildList { get; set; }

        public static bool IsInCallback => _callbackDepth > 0;

        public static int PendingOperationCount => PendingOperations.Count + DeferredCallbackQueue.Count;

        public static event Action<Scene, LoadSceneMode>? sceneLoaded;

        public static event Action<Scene>? sceneUnloaded;

        public static event Action<Scene, Scene>? activeSceneChanged;

        public static int sceneCount => Host?.SceneCount ?? 0;

        public static int sceneCountInBuildSettings => BuildScenePaths.Count;

        public static void SetBuildScenes(params string[] scenePaths)
        {
            BuildScenePaths.Clear();
            if (scenePaths == null)
            {
                return;
            }

            for (int i = 0; i < scenePaths.Length; i++)
            {
                if (!string.IsNullOrEmpty(scenePaths[i]))
                {
                    BuildScenePaths.Add(scenePaths[i]);
                }
            }
        }

        public static void ClearBuildScenes()
        {
            BuildScenePaths.Clear();
            EnforceBuildList = false;
            DeferredCallbackQueue.Clear();
            PendingOperations.Clear();
            _callbackDepth = 0;
        }

        public static void EnterCallbackScope()
        {
            _callbackDepth++;
        }

        public static void ExitCallbackScope()
        {
            if (_callbackDepth > 0)
            {
                _callbackDepth--;
            }
        }

        public static Scene GetActiveScene() => Host?.GetActiveScene() ?? default;

        public static bool SetActiveScene(Scene scene)
        {
            if (!scene.IsValid() || !scene.isLoaded)
            {
                Debug.LogError("SceneManager.SetActiveScene failed: scene '" + scene.name + "' is not loaded.");
                return false;
            }

            Scene previous = GetActiveScene();
            bool changed = Host?.SetActiveScene(scene) ?? false;
            if (changed && previous != scene)
            {
                RaiseActiveSceneChanged(previous, scene);
            }
            return changed;
        }

        public static Scene CreateScene(string sceneName)
        {
            if (string.IsNullOrEmpty(sceneName))
            {
                throw new ArgumentException("Scene name cannot be null or empty.", nameof(sceneName));
            }

            Scene existing = GetSceneByName(sceneName);
            if (existing.IsValid() && existing.isLoaded)
            {
                throw new ArgumentException("Cannot create a new scene with the name of an already loaded scene: " + sceneName, nameof(sceneName));
            }

            int handle = _nextRuntimeSceneHandle++;
            return new Scene(sceneName, string.Empty, -1, true, 0, handle);
        }

        public static void MoveGameObjectToScene(GameObject go, Scene scene)
        {
            if (go is null)
            {
                throw new ArgumentNullException(nameof(go));
            }

            if (go.transform.parent != null)
            {
                throw new ArgumentException("GameObject must be a root object to move to another scene.", nameof(go));
            }

            if (!scene.IsValid() || !scene.isLoaded)
            {
                throw new ArgumentException("Target scene is not loaded.", nameof(scene));
            }

            AssignSceneRecursive(go, scene);
        }

        public static Scene GetSceneByName(string sceneName)
        {
            if (string.IsNullOrEmpty(sceneName))
            {
                return default;
            }

            ISceneHost? host = Host;
            return host == null ? default : host.GetSceneByName(sceneName);
        }

        public static Scene GetSceneByPath(string scenePath)
        {
            if (string.IsNullOrEmpty(scenePath))
            {
                return default;
            }

            int count = sceneCount;
            for (int i = 0; i < count; i++)
            {
                Scene loaded = GetSceneAt(i);
                if (string.Equals(NormalizePath(loaded.path), NormalizePath(scenePath), StringComparison.Ordinal))
                {
                    return loaded;
                }
            }

            return GetSceneByName(ExtractShortName(scenePath));
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

        public static Scene LoadScene(int sceneBuildIndex) => LoadScene(sceneBuildIndex, LoadSceneMode.Single);

        public static Scene LoadScene(int sceneBuildIndex, LoadSceneMode mode)
        {
            ISceneHost host = RequireHost();
            Scene byIndex = host.GetSceneByBuildIndex(sceneBuildIndex);
            if (string.IsNullOrEmpty(byIndex.path))
            {
                Debug.LogError("Scene with build index " + sceneBuildIndex + " could not be loaded because it is not in the build settings.");
                throw new SceneManagementException("Scene with build index " + sceneBuildIndex + " is not in the build settings.");
            }

            return LoadScene(byIndex.path, mode);
        }

        public static Scene LoadScene(string sceneName) => LoadScene(sceneName, LoadSceneMode.Single);

        public static Scene LoadScene(string sceneName, LoadSceneMode mode)
        {
            if (sceneName == null)
            {
                throw new ArgumentNullException(nameof(sceneName));
            }

            ISceneHost host = RequireHost();
            string scenePath = ResolveAndCheckBuildList(host, sceneName);
            if (_callbackDepth > 0)
            {
                DeferredCallbackQueue.Add(() => ExecuteHostLoad(host, scenePath, mode));
                return new Scene(ExtractShortName(scenePath), scenePath, -1, false, 0, 0);
            }

            return ExecuteHostLoad(host, scenePath, mode);
        }

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
            if (_callbackDepth > 0)
            {
                DeferredCallbackQueue.Add(() => ExecuteHostUnload(host, sceneName));
                return true;
            }

            ExecuteHostUnload(host, sceneName);
            return true;
        }

        public static AsyncOperation LoadSceneAsync(int sceneBuildIndex) =>
            LoadSceneAsync(sceneBuildIndex, LoadSceneMode.Single);

        public static AsyncOperation LoadSceneAsync(int sceneBuildIndex, LoadSceneMode mode)
        {
            ISceneHost host = RequireHost();
            Scene byIndex = host.GetSceneByBuildIndex(sceneBuildIndex);
            if (string.IsNullOrEmpty(byIndex.path))
            {
                Debug.LogError("Scene with build index " + sceneBuildIndex + " could not be loaded because it is not in the build settings.");
                throw new SceneManagementException("Scene with build index " + sceneBuildIndex + " is not in the build settings.");
            }

            return LoadSceneAsync(byIndex.path, mode);
        }

        public static AsyncOperation LoadSceneAsync(string sceneName) => LoadSceneAsync(sceneName, LoadSceneMode.Single);

        public static AsyncOperation LoadSceneAsync(string sceneName, LoadSceneMode mode) =>
            LoadSceneAsync(sceneName, mode, true);

        public static AsyncOperation LoadSceneAsync(string sceneName, LoadSceneParameters parameters)
        {
            string requested = string.IsNullOrEmpty(parameters.loadScene) ? sceneName : parameters.loadScene;
            return LoadSceneAsync(requested, parameters.loadSceneMode, true);
        }

        public static AsyncOperation LoadSceneAsync(string sceneName, LoadSceneMode mode, bool allowSceneActivation)
        {
            if (sceneName == null)
            {
                throw new ArgumentNullException(nameof(sceneName));
            }

            ISceneHost host = RequireHost();
            string scenePath = ResolveAndCheckBuildList(host, sceneName);
            AsyncOperation operation = new AsyncOperation
            {
                allowSceneActivation = allowSceneActivation,
            };

            bool deferExecution = !allowSceneActivation || _callbackDepth > 0 || HasHeldOperationAhead();
            if (deferExecution)
            {
                operation.SetDeferredActivation(() => ExecuteHostLoad(host, scenePath, mode));
            }
            else
            {
                AsyncOperation? previousCapturing = _capturingAsyncLoad;
                _capturingAsyncLoad = operation;
                try
                {
                    ExecuteHostLoad(host, scenePath, mode);
                }
                catch (SceneManagementException ex) when (IsCallbackReentrancyRejection(ex))
                {
                    operation.ClearDeferredSceneLoadedEvent();
                    operation.SetDeferredActivation(() => ExecuteHostLoad(host, scenePath, mode));
                }
                finally
                {
                    _capturingAsyncLoad = previousCapturing;
                }
            }

            PendingOperations.Add(operation);
            return operation;
        }

        public static AsyncOperation UnloadSceneAsync(Scene scene) => UnloadSceneAsync(scene.name);

        public static AsyncOperation UnloadSceneAsync(string sceneName)
        {
            if (sceneName == null)
            {
                throw new ArgumentNullException(nameof(sceneName));
            }

            ISceneHost host = RequireHost();
            if (EnforceBuildList && host.SceneCount == 1)
            {
                Scene only = host.GetSceneAt(0);
                if (string.Equals(only.name, sceneName, StringComparison.Ordinal)
                    || string.Equals(NormalizePath(only.path), NormalizePath(sceneName), StringComparison.Ordinal))
                {
                    Debug.LogError("Cannot unload the only loaded scene '" + sceneName + "'.");
                    throw new SceneManagementException("Cannot unload the only loaded scene '" + sceneName + "'.");
                }
            }

            AsyncOperation operation = new AsyncOperation();
            if (_callbackDepth > 0 || HasHeldOperationAhead())
            {
                operation.SetDeferredActivation(() => ExecuteHostUnload(host, sceneName));
            }
            else
            {
                try
                {
                    ExecuteHostUnload(host, sceneName);
                }
                catch (SceneManagementException ex) when (IsCallbackReentrancyRejection(ex))
                {
                    operation.SetDeferredActivation(() => ExecuteHostUnload(host, sceneName));
                }
            }

            PendingOperations.Add(operation);
            return operation;
        }

        public static void FlushDeferredOperations()
        {
            PumpAsyncOperations();
        }

        internal static void RaiseSceneLoaded(Scene scene, LoadSceneMode mode)
        {
            if (_capturingAsyncLoad != null)
            {
                _capturingAsyncLoad.SetDeferredSceneLoadedEvent(() => RaiseSceneLoaded(scene, mode));
                return;
            }

            Action<Scene, LoadSceneMode>? handlers = sceneLoaded;
            if (handlers == null)
            {
                return;
            }

            _callbackDepth++;
            try
            {
                handlers.Invoke(scene, mode);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
            finally
            {
                _callbackDepth--;
            }
        }

        internal static void RaiseSceneUnloaded(Scene scene)
        {
            Action<Scene>? handlers = sceneUnloaded;
            if (handlers == null)
            {
                return;
            }

            _callbackDepth++;
            try
            {
                handlers.Invoke(scene);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
            finally
            {
                _callbackDepth--;
            }
        }

        internal static void RaiseActiveSceneChanged(Scene previousScene, Scene newScene)
        {
            Action<Scene, Scene>? handlers = activeSceneChanged;
            if (handlers == null)
            {
                return;
            }

            _callbackDepth++;
            try
            {
                handlers.Invoke(previousScene, newScene);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
            finally
            {
                _callbackDepth--;
            }
        }

        internal static void PumpAsyncOperations()
        {
            if (DeferredCallbackQueue.Count > 0)
            {
                List<Action> deferred = new List<Action>(DeferredCallbackQueue);
                DeferredCallbackQueue.Clear();
                for (int i = 0; i < deferred.Count; i++)
                {
                    deferred[i]();
                }
            }

            if (PendingOperations.Count == 0)
            {
                return;
            }

            int index = 0;
            while (index < PendingOperations.Count)
            {
                AsyncOperation current = PendingOperations[index];
                if (!current.TryStep())
                {
                    break;
                }

                PendingOperations.RemoveAt(index);
            }

            if (DeferredCallbackQueue.Count > 0)
            {
                List<Action> postDeferred = new List<Action>(DeferredCallbackQueue);
                DeferredCallbackQueue.Clear();
                for (int i = 0; i < postDeferred.Count; i++)
                {
                    postDeferred[i]();
                }
            }
        }

        private static bool HasHeldOperationAhead()
        {
            for (int i = 0; i < PendingOperations.Count; i++)
            {
                if (!PendingOperations[i].isDone && (!PendingOperations[i].allowSceneActivation || PendingOperations[i].HasPendingActivation))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool IsCallbackReentrancyRejection(SceneManagementException ex) =>
            ex.Message.IndexOf("inside a runtime callback", StringComparison.OrdinalIgnoreCase) >= 0;

        private static Scene ExecuteHostLoad(ISceneHost host, string scenePath, LoadSceneMode mode)
        {
            Scene previousActive = host.GetActiveScene();
            Scene loaded = host.LoadScene(scenePath, mode);
            Scene newActive = host.GetActiveScene();
            if (previousActive != newActive && newActive.IsValid())
            {
                RaiseActiveSceneChanged(previousActive, newActive);
            }

            return loaded;
        }

        private static void ExecuteHostUnload(ISceneHost host, string sceneName)
        {
            host.UnloadScene(sceneName);
        }

        private static string ResolveAndCheckBuildList(ISceneHost host, string sceneName)
        {
            if (BuildScenePaths.Count > 0)
            {
                bool isPathQuery = sceneName.IndexOf('/') >= 0
                    || sceneName.IndexOf('\\') >= 0
                    || sceneName.EndsWith(".unity", StringComparison.OrdinalIgnoreCase);

                if (isPathQuery)
                {
                    string normalizedQuery = NormalizePath(sceneName);
                    for (int i = 0; i < BuildScenePaths.Count; i++)
                    {
                        if (string.Equals(NormalizePath(BuildScenePaths[i]), normalizedQuery, StringComparison.Ordinal)
                            || NormalizePath(BuildScenePaths[i]).EndsWith("/" + normalizedQuery, StringComparison.Ordinal))
                        {
                            return BuildScenePaths[i];
                        }
                    }
                }
                else
                {
                    List<string> matches = new List<string>();
                    for (int i = 0; i < BuildScenePaths.Count; i++)
                    {
                        if (string.Equals(ExtractShortName(BuildScenePaths[i]), sceneName, StringComparison.Ordinal))
                        {
                            matches.Add(BuildScenePaths[i]);
                        }
                    }

                    if (matches.Count > 1)
                    {
                        Debug.LogWarning(
                            "Multiple scenes in build settings match short name '"
                            + sceneName
                            + "': "
                            + string.Join(", ", matches)
                            + ". Loading '"
                            + matches[0]
                            + "'. Specify the full scene path to disambiguate.");
                        return matches[0];
                    }

                    if (matches.Count == 1)
                    {
                        return matches[0];
                    }
                }

                if (EnforceBuildList)
                {
                    string message = "Scene '" + sceneName + "' couldn't be loaded because it has not been added to the build settings.";
                    Debug.LogError(message);
                    throw new SceneManagementException(message);
                }
            }

            if (host.TryResolveScenePath(sceneName, out string resolvedPath))
            {
                return resolvedPath;
            }

            if (EnforceBuildList)
            {
                string message = "Scene '" + sceneName + "' couldn't be loaded because it has not been added to the build settings.";
                Debug.LogError(message);
                throw new SceneManagementException(message);
            }

            return sceneName;
        }

        private static string NormalizePath(string path) =>
            (path ?? string.Empty).Replace('\\', '/').Trim('/');

        private static string ExtractShortName(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                return string.Empty;
            }

            string normalized = NormalizePath(path);
            int slash = normalized.LastIndexOf('/');
            string file = slash >= 0 ? normalized.Substring(slash + 1) : normalized;
            if (file.EndsWith(".unity", StringComparison.OrdinalIgnoreCase))
            {
                return file.Substring(0, file.Length - 6);
            }

            return file;
        }

        private static void AssignSceneRecursive(GameObject node, Scene targetScene)
        {
            node.scene = targetScene;
            IReadOnlyList<Component> components = node.Components;
            for (int i = 0; i < components.Count; i++)
            {
                components[i].scene = targetScene;
            }

            for (int i = 0; i < node.transform.childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child != null)
                {
                    AssignSceneRecursive(child, targetScene);
                }
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
