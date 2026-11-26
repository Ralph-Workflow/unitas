using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed class BridgeHost : IDisposable, IPhysicsProvider, ICharacterControllerProvider, IInputProvider, ISceneHost, IApplicationHost, IAudioProvider, IRuntimeObjectProvider
    {
        private const uint CollisionPhaseEnter = 0;
        private const int GAME_OBJECT_CLASS = 1;
        private const int TRANSFORM_CLASS = 4;
        private const int MONO_BEHAVIOUR_CLASS = 114;
        private const int MESH_COLLIDER_CLASS = 64;
        private const int PREFAB_INSTANCE_CLASS = 1001;
        private const int MaxPrefabInstanceDepth = 8;
        private const uint CollisionPhaseStay = 1;
        private const uint CollisionPhaseExit = 2;
        private const uint InputStateDown = 1;
        private const uint InputStatePressed = 2;
        private const uint InputStateReleased = 4;

        private readonly ulong _runtimeHandle;
        private readonly Dictionary<ulong, MonoBehaviour> _scripts = new Dictionary<ulong, MonoBehaviour>();
        private readonly Dictionary<ulong, ulong> _entityScripts = new Dictionary<ulong, ulong>();
        private readonly Dictionary<ulong, List<ulong>> _entityScriptHandles = new Dictionary<ulong, List<ulong>>();
        private readonly Dictionary<ulong, GameObject> _entityGameObjects = new Dictionary<ulong, GameObject>();
        private readonly Dictionary<ulong, RuntimeBodySnapshot> _runtimeBodySnapshots = new Dictionary<ulong, RuntimeBodySnapshot>();
        private readonly HashSet<ulong> _runtimeCreatedEntities = new HashSet<ulong>();
        private MonoBehaviour? _lastCreatedScript;
        private readonly List<ulong> _materializedSceneEntities = new List<ulong>();
        private readonly List<UnityEngine.Object> _destroyedObjects = new List<UnityEngine.Object>();
        private readonly Dictionary<ulong, string> _sceneObjectNames = new Dictionary<ulong, string>();
        private readonly Dictionary<ulong, ulong> _sceneParentEntities = new Dictionary<ulong, ulong>();
        private readonly Dictionary<ulong, SceneRigidbodyMetadata> _sceneRigidbodies = new Dictionary<ulong, SceneRigidbodyMetadata>();
        private readonly Dictionary<ulong, SceneSphereColliderMetadata> _sceneSphereColliders = new Dictionary<ulong, SceneSphereColliderMetadata>();
        private readonly Dictionary<ulong, SceneBoxColliderMetadata> _sceneBoxColliders = new Dictionary<ulong, SceneBoxColliderMetadata>();
        private readonly Dictionary<ulong, SceneCapsuleColliderMetadata> _sceneCapsuleColliders = new Dictionary<ulong, SceneCapsuleColliderMetadata>();
        private readonly Dictionary<ulong, SceneMeshColliderMetadata> _sceneMeshColliders = new Dictionary<ulong, SceneMeshColliderMetadata>();
        private readonly Dictionary<string, Mesh> _sceneMeshes = new Dictionary<string, Mesh>(StringComparer.Ordinal);
        private readonly Dictionary<ulong, SceneTransformMetadata> _sceneTransforms = new Dictionary<ulong, SceneTransformMetadata>();
        private readonly Dictionary<ulong, SceneCameraSettings> _sceneCameras = new Dictionary<ulong, SceneCameraSettings>();
        private readonly Dictionary<(ulong, ScriptGlueRegistry.BuiltInUiComponent), ParsedUiComponentRecord> _sceneUiComponents = new Dictionary<(ulong, ScriptGlueRegistry.BuiltInUiComponent), ParsedUiComponentRecord>();
        private readonly Dictionary<(ulong, ulong), List<ulong>> _sceneScriptOwners = new Dictionary<(ulong, ulong), List<ulong>>();
        private bool _alignSceneEntityBase;
        private ulong _sceneEntityKeyShift;
        private readonly List<GameObject> _materializedSceneCameras = new List<GameObject>();
        private ulong _nextSceneEntityBase = 1;
        private readonly List<Scene> _loadedScenes = new List<Scene>();
        private readonly Dictionary<string, int> _sceneHandles = new Dictionary<string, int>(StringComparer.Ordinal);
        private readonly LoadedSceneFileProvider _defaultSceneProvider;
        private readonly NativeSceneBackend _defaultSceneBackend;
        private readonly RuntimeAnimationProvider _animationProvider;
        private readonly RuntimeRenderProvider _renderProvider;
        private readonly RuntimeUiProvider _uiProvider;
        private ISceneProvider _sceneProvider;
        private ISceneBackend _sceneBackend;
        private static int _nextSceneHandle = 1;
        private ulong _nextScriptHandle = 1;
        private readonly List<ulong> _fixedUpdatePhaseHandles = new List<ulong>();
        private readonly List<ulong> _updatePhaseHandles = new List<ulong>();
        private readonly List<ulong> _lateUpdatePhaseHandles = new List<ulong>();
        private bool _isDisposed;
        private bool _isQuitting;
        private bool _isFocused = true;
        private bool _isPaused;
        private bool _pendingAfterSceneLoadRuntimeInit;
        private bool _deferStartDuringSceneLoad;
        private readonly List<ulong> _deferredStartHandles = new List<ulong>();
        private readonly bool _isDomainReloadDisabled;
        private readonly HashSet<RuntimeInitializeLoadType> _executedRuntimeInitTypes =
            new HashSet<RuntimeInitializeLoadType>();
        private ApplicationBuildKind _buildKind = ApplicationBuildKind.Editor;
        private string _dataPath = string.Empty;
        private string _companyName = string.Empty;
        private string _productName = string.Empty;
        private string _projectVersion = string.Empty;

        public static bool DomainReloadDisabled { get; set; }

        private readonly RuntimeInterop.CreateScriptCallback _createScriptCallback;
        private readonly RuntimeInterop.AwakeCallback _awakeCallback;
        private readonly RuntimeInterop.OnEnableCallback _onEnableCallback;
        private readonly RuntimeInterop.StartCallback _startCallback;
        private readonly RuntimeInterop.FixedUpdateCallback _fixedUpdateCallback;
        private readonly RuntimeInterop.UpdateCallback _updateCallback;
        private readonly RuntimeInterop.LateUpdateCallback _lateUpdateCallback;
        private readonly RuntimeInterop.OnDisableCallback _onDisableCallback;
        private readonly RuntimeInterop.DestroyCallback _destroyCallback;
        private readonly RuntimeInterop.CollisionCallback _collisionCallback;
        private readonly RuntimeInterop.TriggerCallback _triggerCallback;

        public BridgeHost() : this(CreateRuntimeHandle())
        {
        }

        public BridgeHost(ulong runtimeHandle)
        {
            if (runtimeHandle == 0)
            {
                throw new ArgumentException("Runtime handle must be non-zero.", nameof(runtimeHandle));
            }
            _runtimeHandle = runtimeHandle;

            _defaultSceneProvider = new LoadedSceneFileProvider();
            _defaultSceneBackend = new NativeSceneBackend(runtimeHandle);
            _animationProvider = new RuntimeAnimationProvider(runtimeHandle);
            _renderProvider = new RuntimeRenderProvider(runtimeHandle);
            _uiProvider = new RuntimeUiProvider(runtimeHandle);
            _sceneProvider = _defaultSceneProvider;
            _sceneBackend = _defaultSceneBackend;

            _createScriptCallback = OnCreateScript;
            _awakeCallback = OnAwake;
            _onEnableCallback = OnOnEnable;
            _startCallback = OnStart;
            _fixedUpdateCallback = OnFixedUpdate;
            _updateCallback = OnUpdate;
            _lateUpdateCallback = OnLateUpdate;
            _onDisableCallback = OnOnDisable;
            _destroyCallback = OnDestroy;
            _collisionCallback = OnCollision;
            _triggerCallback = OnTrigger;

            int result = RuntimeInterop.unitas_runtime_set_callbacks(
                _runtimeHandle,
                _createScriptCallback,
                _awakeCallback,
                _onEnableCallback,
                _startCallback,
                _fixedUpdateCallback,
                _updateCallback,
                _lateUpdateCallback,
                _onDisableCallback,
                _destroyCallback,
                _collisionCallback,
                _triggerCallback);

            if (result != 0)
            {
                string? error = GetErrorMessage();
                if (string.IsNullOrEmpty(error))
                {
                    throw new InvalidOperationException("Failed to set callbacks on native runtime (error code " + result.ToString(CultureInfo.InvariantCulture) + ").");
                }
                throw new InvalidOperationException("Failed to set callbacks on native runtime (error code " + result.ToString(CultureInfo.InvariantCulture) + "): " + error);
            }

            Physics.Provider = this;
            CharacterController.Provider = this;
            Input.Provider = this;
            SceneManager.Host = this;
            Application.Host = this;
            string? persistentDataPathOverride = Environment.GetEnvironmentVariable("UNITAS_PERSISTENT_DATA_PATH");
            if (!string.IsNullOrEmpty(persistentDataPathOverride))
            {
                Application.PersistentDataPathOverride = persistentDataPathOverride;
            }
            AudioSource.Provider = this;
            RuntimeObjectProvider.Current = this;
            RuntimeObjectProvider.ScriptFactoryRegistrar = ScriptGlueRegistry.RegisterTypeFactory;
            RuntimeObjectProvider.RuntimeObjectRegistrationHandler = RegisterRuntimeGameObject;
            Animator.Provider = _animationProvider;
            Camera.Provider = _renderProvider;
            Mesh.Provider = _renderProvider;
            Material.Provider = _renderProvider;
            MeshFilter.Provider = _renderProvider;
            MeshRenderer.Provider = _renderProvider;
            Light.Provider = _renderProvider;
            UnityEngine.UI.Ui.Provider = _uiProvider;
            ScriptGlueRegistry.RegisterBuiltInUiComponents();
            Unitas.Bridge.Messaging.MessageDispatcher.EnsureInstalled();
            UnitySynchronizationContext.Install();
            Time.PushTimeScaleToHost();
            ScanProjectResources(DataPath);

            string? glueAssemblyPath = Environment.GetEnvironmentVariable("UNITAS_GLUE_ASSEMBLY");
            if (!string.IsNullOrEmpty(glueAssemblyPath) && File.Exists(glueAssemblyPath))
            {
                ScriptGlueRegistry.LoadGlueAssembly(glueAssemblyPath);
            }

            _isDomainReloadDisabled = DomainReloadDisabled;
            RunPreSceneRuntimeInitialization();
        }

        public void LoadGlueAssembly(string assemblyPath)
        {
            ThrowIfDisposed();
            ScriptGlueRegistry.LoadGlueAssembly(assemblyPath);
            RunPreSceneRuntimeInitialization();
        }

        public bool IsDomainReloadDisabled => _isDomainReloadDisabled;

        public void CompleteRuntimeInitializationForSession()
        {
            ThrowIfDisposed();
            RunPreSceneRuntimeInitialization();
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.BeforeSceneLoad);
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
        }

        private void RunPreSceneRuntimeInitialization()
        {
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.SubsystemRegistration);
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterAssembliesLoaded);
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.BeforeSplashScreen);
        }

        private void RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType loadType)
        {
            if (!_executedRuntimeInitTypes.Add(loadType))
            {
                return;
            }

            List<Action> callbacks = RuntimeInitializeOnLoadMethodAttribute.GetCallbacksForLoadType(loadType);
            for (int index = 0; index < callbacks.Count; index++)
            {
                try
                {
                    callbacks[index]();
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex);
                }
            }
        }

        public ulong Handle => _runtimeHandle;

        public int ScriptCount => _scripts.Count;

        public bool IsPlaying => !_isDisposed;

        public bool IsFocused => !_isDisposed && _isFocused;

        public bool IsPaused => !_isDisposed && _isPaused;

        public ApplicationBuildKind BuildKind
        {
            get => _buildKind;
            set => _buildKind = value;
        }

        public string DataPath => string.IsNullOrEmpty(_dataPath)
            ? Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "Assets"))
            : _dataPath;

        public string StreamingAssetsPath => Path.Combine(DataPath, "StreamingAssets");

        public string CompanyName => _companyName;

        public string ProductName => _productName;

        public string Version => _projectVersion;

        public void EndSession()
        {
            Shutdown();
        }

        public void SetFocus(bool hasFocus)
        {
            ThrowIfDisposed();
            _isFocused = hasFocus;
            Application.RaiseFocusChanged(hasFocus);
            DispatchApplicationCallback(script => script.OnApplicationFocus(hasFocus));
        }

        public void SetPaused(bool isPaused)
        {
            ThrowIfDisposed();
            _isPaused = isPaused;
            if (_runtimeHandle != 0)
            {
                RuntimeInterop.unitas_runtime_audio_set_paused(_runtimeHandle, isPaused ? 1U : 0U);
            }
            Application.RaisePauseChanged(isPaused);
            DispatchApplicationCallback(script => script.OnApplicationPause(isPaused));
        }

        public ISceneProvider Provider
        {
            get => _sceneProvider;
            set => _sceneProvider = value ?? throw new ArgumentNullException(nameof(value));
        }

        public ISceneBackend Backend
        {
            get => _sceneBackend;
            set => _sceneBackend = value ?? throw new ArgumentNullException(nameof(value));
        }

        public MonoBehaviour? GetScript(ulong handle)
        {
            if (_scripts.TryGetValue(handle, out MonoBehaviour? script))
            {
                return script;
            }
            return null;
        }

        public void LoadScene(string path)
        {
            ThrowIfDisposed();
            ArgumentNullException.ThrowIfNull(path);
            RunPreSceneRuntimeInitialization();
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.BeforeSceneLoad);
            byte[] utf8Bytes = Encoding.UTF8.GetBytes(path);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)utf8Bytes.Length);
            if (buffer == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to allocate path buffer in native runtime.");
            }
            Marshal.Copy(utf8Bytes, 0, buffer, utf8Bytes.Length);
            int result = RuntimeInterop.unitas_runtime_load_scene(_runtimeHandle, buffer, (ulong)utf8Bytes.Length);
            if (result != 0)
            {
                string? error = GetErrorMessage();
                if (string.IsNullOrEmpty(error))
                {
                    throw new InvalidOperationException("Failed to load scene (error code " + result.ToString(CultureInfo.InvariantCulture) + ").");
                }
                throw new InvalidOperationException("Failed to load scene (error code " + result.ToString(CultureInfo.InvariantCulture) + "): " + error);
            }
            double fixedTimestep = RuntimeInterop.unitas_runtime_fixed_timestep(_runtimeHandle);
            if (fixedTimestep > 0.0)
            {
                Time.SetFixedDeltaTime((float)fixedTimestep);
            }

            Console.Error.WriteLine("DIAG LoadScene(1) path=" + path + " afterNativeLoad scripts=" + _scripts.Count);
            DropReplacedScripts();
            DestroyReplacedSceneObjects();
            Console.Error.WriteLine("DIAG LoadScene(1) afterTeardown scripts=" + _scripts.Count);
            ParseLoadedSceneRigidbodies(path, LoadSceneMode.Single);
            _defaultSceneProvider.SetSceneRoot(path);
            UpdateProjectMetadata(path);
            RegisterLoadedScene(path, LoadSceneMode.Single);
            Scene loaded = FindLoadedScene(path);
            DeregisterReplacedSceneObjects(loaded, LoadSceneMode.Single);
            SceneManager.RaiseSceneLoaded(loaded, LoadSceneMode.Single);
            Console.Error.WriteLine("DIAG LoadScene(1) hasAfterSceneLoad=" + RuntimeInitializeOnLoadMethodAttribute.HasRegisteredCallbacks(RuntimeInitializeLoadType.AfterSceneLoad));
            if (RuntimeInitializeOnLoadMethodAttribute.HasRegisteredCallbacks(RuntimeInitializeLoadType.AfterSceneLoad))
            {
                _pendingAfterSceneLoadRuntimeInit = true;
                _deferStartDuringSceneLoad = true;
                try
                {
                    int initResult = RuntimeInterop.unitas_runtime_tick(_runtimeHandle, 0.0);
                    if (initResult != 0)
                    {
                        string? error = GetErrorMessage();
                        if (string.IsNullOrEmpty(error))
                        {
                            throw new InvalidOperationException("Failed to initialize scene scripts (error code " + initResult.ToString(CultureInfo.InvariantCulture) + ").");
                        }
                        throw new InvalidOperationException("Failed to initialize scene scripts (error code " + initResult.ToString(CultureInfo.InvariantCulture) + "): " + error);
                    }
                }
                finally
                {
                    _deferStartDuringSceneLoad = false;
                    _alignSceneEntityBase = false;
                }
                if (_pendingAfterSceneLoadRuntimeInit)
                {
                    _pendingAfterSceneLoadRuntimeInit = false;
                    RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
                }
            }
            else
            {
                RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
            }
            Console.Error.WriteLine("DIAG LoadScene(1) afterTick scripts=" + _scripts.Count + " gameObjects=" + _entityGameObjects.Count);
            FlushDestroyedObjects();
        }

        public Scene LoadScene(string scenePath, LoadSceneMode mode)
        {
            ThrowIfDisposed();
            ArgumentNullException.ThrowIfNull(scenePath);
            RunPreSceneRuntimeInitialization();
            RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.BeforeSceneLoad);

            try
            {
                Backend.LoadScene(scenePath, mode);
            }
            catch (SceneManagementException)
            {
                throw;
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                throw new SceneManagementException("Failed to load scene '" + scenePath + "': " + ex.Message, ex);
            }

            Console.Error.WriteLine("DIAG LoadScene(2) path=" + scenePath + " mode=" + mode + " afterNativeLoad scripts=" + _scripts.Count);
            if (mode == LoadSceneMode.Single)
            {
                DropReplacedScripts();
                DestroyReplacedSceneObjects();
            }

            Console.Error.WriteLine("DIAG LoadScene(2) afterTeardown scripts=" + _scripts.Count);
            ParseLoadedSceneRigidbodies(scenePath, mode);
            _defaultSceneProvider.SetSceneRoot(scenePath);
            UpdateProjectMetadata(scenePath);
            RegisterLoadedScene(scenePath, mode);
            Scene loaded = FindLoadedScene(scenePath);
            DeregisterReplacedSceneObjects(loaded, mode);
            SceneManager.RaiseSceneLoaded(loaded, mode);
            Console.Error.WriteLine("DIAG LoadScene(2) hasAfterSceneLoad=" + RuntimeInitializeOnLoadMethodAttribute.HasRegisteredCallbacks(RuntimeInitializeLoadType.AfterSceneLoad));
            if (RuntimeInitializeOnLoadMethodAttribute.HasRegisteredCallbacks(RuntimeInitializeLoadType.AfterSceneLoad))
            {
                _pendingAfterSceneLoadRuntimeInit = true;
                _deferStartDuringSceneLoad = true;
                try
                {
                    int initResult = RuntimeInterop.unitas_runtime_tick(_runtimeHandle, 0.0);
                    if (initResult != 0)
                    {
                        string? error = GetErrorMessage();
                        if (string.IsNullOrEmpty(error))
                        {
                            throw new InvalidOperationException("Failed to initialize scene scripts (error code " + initResult.ToString(CultureInfo.InvariantCulture) + ").");
                        }
                        throw new InvalidOperationException("Failed to initialize scene scripts (error code " + initResult.ToString(CultureInfo.InvariantCulture) + "): " + error);
                    }
                }
                finally
                {
                    _deferStartDuringSceneLoad = false;
                    _alignSceneEntityBase = false;
                }
                if (_pendingAfterSceneLoadRuntimeInit)
                {
                    _pendingAfterSceneLoadRuntimeInit = false;
                    RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
                }
            }
            else
            {
                RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
            }
            Console.Error.WriteLine("DIAG LoadScene(2) afterTick scripts=" + _scripts.Count + " gameObjects=" + _entityGameObjects.Count);
            FlushDestroyedObjects();
            return loaded;
        }

        public void UnloadScene(string sceneName)
        {
            ThrowIfDisposed();
            ArgumentNullException.ThrowIfNull(sceneName);
            Scene scene = GetSceneByName(sceneName);
            try
            {
                Backend.UnloadScene(sceneName);
            }
            catch (SceneManagementException)
            {
                throw;
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                throw new SceneManagementException("Failed to unload scene '" + sceneName + "': " + ex.Message, ex);
            }

            RemoveLoadedScene(sceneName);
            DeregisterUnloadedSceneObjects(scene);
            SceneManager.RaiseSceneUnloaded(scene);
        }

        private static void DeregisterUnloadedSceneObjects(Scene unloadedScene)
        {
            if (unloadedScene.handle == 0)
            {
                return;
            }

            List<UnityEngine.Object> snapshot = UnityEngine.Object.SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                UnityEngine.Object candidate = snapshot[i];
                if (candidate is null || candidate.IsPersistent || candidate.scene.handle != unloadedScene.handle)
                {
                    continue;
                }

                if (candidate is GameObject go)
                {
                    if (!go.IsPersistent)
                    {
                        UnityEngine.Object.DeregisterLiveObject(go);
                        IReadOnlyList<Component> comps = go.Components;
                        for (int j = 0; j < comps.Count; j++)
                        {
                            UnityEngine.Object.DeregisterLiveObject(comps[j]);
                        }
                    }
                }
                else if (candidate is Component comp)
                {
                    GameObject? owner = comp.gameObject;
                    if (owner is null || !owner.IsPersistent)
                    {
                        UnityEngine.Object.DeregisterLiveObject(comp);
                    }
                }
                else
                {
                    UnityEngine.Object.DeregisterLiveObject(candidate);
                }
            }
        }

        private static void DeregisterReplacedSceneObjects(Scene loadedScene, LoadSceneMode mode)
        {
            if (mode != LoadSceneMode.Single || loadedScene.handle == 0)
            {
                return;
            }

            List<UnityEngine.Object> snapshot = UnityEngine.Object.SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                UnityEngine.Object candidate = snapshot[i];
                if (candidate is null || candidate.IsPersistent || candidate.scene.handle == 0 || candidate.scene.handle == loadedScene.handle)
                {
                    continue;
                }

                if (candidate is GameObject go)
                {
                    if (!go.IsPersistent)
                    {
                        UnityEngine.Object.DeregisterLiveObject(go);
                        IReadOnlyList<Component> comps = go.Components;
                        for (int j = 0; j < comps.Count; j++)
                        {
                            UnityEngine.Object.DeregisterLiveObject(comps[j]);
                        }
                    }
                }
                else if (candidate is Component comp)
                {
                    GameObject? owner = comp.gameObject;
                    if (owner is null || !owner.IsPersistent)
                    {
                        UnityEngine.Object.DeregisterLiveObject(comp);
                    }
                }
                else
                {
                    UnityEngine.Object.DeregisterLiveObject(candidate);
                }
            }
        }

        /// <summary>
        /// Destroys the objects of the scenes a single-mode load replaces. The scene handle
        /// is keyed by path, so reloading the same path leaves the objects of the previous
        /// load looking like members of the new scene; Unity destroys the replaced scene's
        /// objects, and the bridge has to do the same.
        /// </summary>
        private void DestroyReplacedSceneObjects()
        {
            List<UnityEngine.Object> snapshot = UnityEngine.Object.SnapshotLiveObjects();
            for (int index = 0; index < snapshot.Count; index++)
            {
                UnityEngine.Object candidate = snapshot[index];
                if (candidate is null || candidate.IsDestroyed || candidate.IsPersistent)
                {
                    continue;
                }

                if (candidate is GameObject gameObject && BelongsToLoadedScene(gameObject))
                {
                    UnityEngine.Object.DestroySceneObject(gameObject);
                }
            }
        }

        private bool BelongsToLoadedScene(UnityEngine.Object candidate)
        {
            int handle = candidate.scene.handle;
            if (handle == 0)
            {
                return false;
            }

            for (int index = 0; index < _loadedScenes.Count; index++)
            {
                if (_loadedScenes[index].handle == handle)
                {
                    return true;
                }
            }

            return false;
        }

        public bool TryResolveScenePath(string sceneName, out string scenePath)
        {
            return Provider.TryResolveScenePath(sceneName, out scenePath);
        }

        public int SceneCount => _loadedScenes.Count;

        public Scene GetSceneAt(int index)
        {
            if (index < 0 || index >= _loadedScenes.Count)
            {
                return default;
            }

            return _loadedScenes[index];
        }

        public Scene GetSceneByName(string sceneName)
        {
            if (string.IsNullOrEmpty(sceneName))
            {
                return default;
            }

            for (int index = 0; index < _loadedScenes.Count; index++)
            {
                if (string.Equals(_loadedScenes[index].name, sceneName, StringComparison.Ordinal))
                {
                    return _loadedScenes[index];
                }
            }

            if (!Provider.TryResolveScenePath(sceneName, out string scenePath))
            {
                return default;
            }

            return new Scene(sceneName, scenePath, BuildIndexOf(scenePath), false, 0, 0);
        }

        public Scene GetSceneByBuildIndex(int buildIndex)
        {
            if (!Provider.TryResolveScenePathByBuildIndex(buildIndex, out string scenePath))
            {
                return default;
            }

            string name = System.IO.Path.GetFileNameWithoutExtension(scenePath);
            return new Scene(name, scenePath, buildIndex, IsLoaded(scenePath), 0, HandleFor(scenePath));
        }

        public Scene GetActiveScene()
        {
            int index = Backend.ActiveSceneIndex;
            if (index < 0 || index >= _loadedScenes.Count)
            {
                return default;
            }

            return _loadedScenes[index];
        }

        public bool SetActiveScene(Scene scene)
        {
            int index = IndexOfLoadedScene(scene);
            if (index < 0)
            {
                return false;
            }

            return Backend.SetActiveScene(index);
        }

        public void DontDestroyOnLoad(ulong entity)
        {
            Backend.DontDestroyOnLoad(entity);
        }

        private void RegisterLoadedScene(string path, LoadSceneMode mode)
        {
            string name = System.IO.Path.GetFileNameWithoutExtension(path);
            if (mode == LoadSceneMode.Single)
            {
                _loadedScenes.Clear();
            }

            RemoveLoadedScene(name);
            _loadedScenes.Add(new Scene(name, path, BuildIndexOf(path), true, 0, HandleFor(path)));
        }

        private void RemoveLoadedScene(string sceneName)
        {
            for (int index = _loadedScenes.Count - 1; index >= 0; index--)
            {
                if (string.Equals(_loadedScenes[index].name, sceneName, StringComparison.Ordinal))
                {
                    _loadedScenes.RemoveAt(index);
                }
            }
        }

        private void DropReplacedScripts()
        {
            _fixedUpdatePhaseHandles.Clear();
            _updatePhaseHandles.Clear();
            _lateUpdatePhaseHandles.Clear();
            List<ulong> dropped = new List<ulong>();
            foreach (KeyValuePair<ulong, MonoBehaviour> entry in _scripts)
            {
                MonoBehaviour script = entry.Value;
                GameObject? owner = script == null ? null : script.gameObject;
                if (owner != null && owner.IsPersistent)
                {
                    continue;
                }

                if (script is not null)
                {
                    StopAllSafely(script);
                }

                dropped.Add(entry.Key);
            }

            for (int index = 0; index < dropped.Count; index++)
            {
                ulong handle = dropped[index];
                ulong entity = EntityForScript(handle);
                _scripts.Remove(handle);
                if (entity != 0)
                {
                    if (_entityScriptHandles.TryGetValue(entity, out List<ulong>? list))
                    {
                        list.Remove(handle);
                        if (list.Count == 0)
                        {
                            _entityScriptHandles.Remove(entity);
                        }
                    }
                    _entityScripts.Remove(entity);
                    _entityGameObjects.Remove(entity);
                }
            }
            _runtimeBodySnapshots.Clear();
            _runtimeCreatedEntities.Clear();
        }

        private ulong EntityForScript(ulong scriptHandle)
        {
            foreach (KeyValuePair<ulong, List<ulong>> entry in _entityScriptHandles)
            {
                for (int i = 0; i < entry.Value.Count; i++)
                {
                    if (entry.Value[i] == scriptHandle)
                    {
                        return entry.Key;
                    }
                }
            }

            foreach (KeyValuePair<ulong, ulong> entry in _entityScripts)
            {
                if (entry.Value == scriptHandle)
                {
                    return entry.Key;
                }
            }

            return 0;
        }

        private int BuildIndexOf(string scenePath)
        {
            IReadOnlyList<string> paths = Provider.ScenePaths;
            for (int index = 0; index < paths.Count; index++)
            {
                if (string.Equals(paths[index], scenePath, StringComparison.Ordinal))
                {
                    return index;
                }
            }

            return -1;
        }

        private bool IsLoaded(string scenePath)
        {
            for (int index = 0; index < _loadedScenes.Count; index++)
            {
                if (string.Equals(_loadedScenes[index].path, scenePath, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private int IndexOfLoadedScene(Scene scene)
        {
            for (int index = 0; index < _loadedScenes.Count; index++)
            {
                if (scene.handle != 0 && _loadedScenes[index].handle == scene.handle)
                {
                    return index;
                }
            }

            return -1;
        }

        private Scene FindLoadedScene(string scenePath)
        {
            for (int index = 0; index < _loadedScenes.Count; index++)
            {
                if (string.Equals(_loadedScenes[index].path, scenePath, StringComparison.Ordinal))
                {
                    return _loadedScenes[index];
                }
            }

            string name = System.IO.Path.GetFileNameWithoutExtension(scenePath);
            return new Scene(name, scenePath, BuildIndexOf(scenePath), true, 0, HandleFor(scenePath));
        }

        private int HandleFor(string scenePath)
        {
            if (_sceneHandles.TryGetValue(scenePath, out int handle))
            {
                return handle;
            }

            handle = _nextSceneHandle++;
            _sceneHandles[scenePath] = handle;
            return handle;
        }

        public void Tick(double deltaSeconds)
        {
            ThrowIfDisposed();
            if (_pendingAfterSceneLoadRuntimeInit)
            {
                _pendingAfterSceneLoadRuntimeInit = false;
                RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
            }
            if (_deferredStartHandles.Count > 0)
            {
                List<ulong> pendingStarts = new List<ulong>(_deferredStartHandles);
                _deferredStartHandles.Clear();
                SyncSimulatedTransforms();
                for (int i = 0; i < pendingStarts.Count; i++)
                {
                    if (_scripts.TryGetValue(pendingStarts[i], out MonoBehaviour? script) && IsEligible(script))
                    {
                        InvokeWithOrigin(script!, script!.Start);
                    }
                }
            }
            WarnOnLeftoverPhaseHandles();
            _fixedUpdatePhaseHandles.Clear();
            _updatePhaseHandles.Clear();
            _lateUpdatePhaseHandles.Clear();
            SyncRuntimeObjectsBeforeTick();
            SyncAudioBeforeTick();
            Time.Advance(deltaSeconds);
            double tickDeltaSeconds = Time.captureFramerate > 0 ? Time.unscaledDeltaTime : deltaSeconds;
            NativeDestroyInterop.unitas_runtime_bind_destroy_channel(_runtimeHandle);
            int result = RuntimeInterop.unitas_runtime_tick(_runtimeHandle, tickDeltaSeconds);
            if (result != 0)
            {
                string? error = GetErrorMessage();
                if (string.IsNullOrEmpty(error))
                {
                    throw new InvalidOperationException("Failed to advance runtime tick (error code " + result.ToString(CultureInfo.InvariantCulture) + ").");
                }
                throw new InvalidOperationException("Failed to advance runtime tick (error code " + result.ToString(CultureInfo.InvariantCulture) + "): " + error);
            }

            if (_fixedUpdatePhaseHandles.Count > 0)
            {
                DispatchPhaseInExecutionOrder(_fixedUpdatePhaseHandles, script => script.FixedUpdate());
                _fixedUpdatePhaseHandles.Clear();
                PumpFixedSafely();
            }
            if (_updatePhaseHandles.Count > 0)
            {
                DispatchPhaseInExecutionOrder(_updatePhaseHandles, script => script.Update());
                _updatePhaseHandles.Clear();
            }
            if (_lateUpdatePhaseHandles.Count > 0)
            {
                DispatchPhaseInExecutionOrder(_lateUpdatePhaseHandles, script => script.LateUpdate());
                _lateUpdatePhaseHandles.Clear();
                PumpFrameSafely();
            }

            FlushDestroyedObjects();
            SyncSimulatedTransforms();
            SyncAudioSourceTransforms();
            SceneManager.PumpAsyncOperations();
            if (_scripts.Count == 0)
            {
                UnitySynchronizationContext.PumpFrame();
            }

            // The clicks the runtime queued this frame reach the managed buttons
            // here, the point the packaged player pumps them at.
            UnityEngine.UI.Ui.PumpClicks();
        }

        private void SyncAudioSourceTransforms()
        {
            if (_isDisposed || _runtimeHandle == 0)
            {
                return;
            }

            AudioSource[] sources = UnityEngine.Object.FindObjectsOfType<AudioSource>();
            for (int index = 0; index < sources.Length; index++)
            {
                AudioSource source = sources[index];
                if (source != null && source.isActiveAndEnabled && source.TryGetEntityId(out ulong entityId) && entityId != 0UL)
                {
                    Vector3 pos = source.transform.position;
                    RuntimeInterop.unitas_runtime_audio_set_position(_runtimeHandle, entityId, pos.x, pos.y, pos.z);
                }
            }
        }

        public void Shutdown()
        {
            if (!_isDisposed && !_isQuitting)
            {
                _isQuitting = true;
                Application.RaiseQuitting();
                DispatchApplicationCallback(script => script.OnApplicationQuit());
                _isDisposed = true;
                int result = RuntimeInterop.unitas_runtime_destroy(_runtimeHandle);
                _fixedUpdatePhaseHandles.Clear();
                _updatePhaseHandles.Clear();
                _lateUpdatePhaseHandles.Clear();
                _scripts.Clear();
                _entityScripts.Clear();
                _entityScriptHandles.Clear();
                _entityGameObjects.Clear();
                _runtimeBodySnapshots.Clear();
                _runtimeCreatedEntities.Clear();
                _lastCreatedScript = null;
                _materializedSceneEntities.Clear();
                DropMaterializedSceneCameras();
                _sceneObjectNames.Clear();
                _sceneParentEntities.Clear();
                _sceneRigidbodies.Clear();
                _sceneSphereColliders.Clear();
                _sceneBoxColliders.Clear();
                _sceneCapsuleColliders.Clear();
                _sceneMeshColliders.Clear();
                _sceneTransforms.Clear();
                _sceneUiComponents.Clear();
                StopAllSafely();
                if (ReferenceEquals(Physics.Provider, this))
                {
                    Physics.Provider = null;
                }
                if (ReferenceEquals(CharacterController.Provider, this))
                {
                    CharacterController.Provider = null;
                }
                if (ReferenceEquals(Input.Provider, this))
                {
                    Input.Provider = null;
                }
                if (ReferenceEquals(SceneManager.Host, this))
                {
                    SceneManager.Host = null;
                }
                if (ReferenceEquals(Application.Host, this))
                {
                    Application.Host = null;
                }
                if (ReferenceEquals(AudioSource.Provider, this))
                {
                    AudioSource.Provider = null;
                }
                if (ReferenceEquals(RuntimeObjectProvider.Current, this))
                {
                    RuntimeObjectProvider.Current = null;
                    RuntimeObjectProvider.RuntimeObjectRegistrationHandler = null;
                }
                if (ReferenceEquals(Animator.Provider, _animationProvider))
                {
                    Animator.Provider = null;
                }
                if (ReferenceEquals(UnityEngine.UI.Ui.Provider, _uiProvider))
                {
                    UnityEngine.UI.Ui.Provider = null;
                }
                if (result != 0)
                {
                    string? error = GetErrorMessage();
                    if (string.IsNullOrEmpty(error))
                    {
                        throw new InvalidOperationException("Failed to destroy native runtime (error code " + result.ToString(CultureInfo.InvariantCulture) + ").");
                    }
                    throw new InvalidOperationException("Failed to destroy native runtime (error code " + result.ToString(CultureInfo.InvariantCulture) + "): " + error);
                }
            }
        }

        private void ThrowIfDisposed()
        {
            ObjectDisposedException.ThrowIf(_isDisposed, this);
        }

        private void DispatchApplicationCallback(Action<MonoBehaviour> action)
        {
            if (_scripts.Count == 0)
            {
                return;
            }

            List<ulong> handles = new List<ulong>(_scripts.Keys);
            handles.Sort();
            for (int index = 0; index < handles.Count; index++)
            {
                if (_scripts.TryGetValue(handles[index], out MonoBehaviour? script) && IsEligible(script))
                {
                    MonoBehaviour target = script!;
                    InvokeWithOrigin(target, () => action(target));
                }
            }
        }

        private void UpdateProjectMetadata(string scenePath)
        {
            string? projectRoot = null;
            if (File.Exists(scenePath))
            {
                projectRoot = ResolveProjectRoot(scenePath, out string resolvedDataPath);
                _dataPath = resolvedDataPath;
            }
            else
            {
                string fullScenePath = Path.Combine(Directory.GetCurrentDirectory(), scenePath);
                string? sceneDir = Path.GetDirectoryName(fullScenePath);
                _dataPath = string.IsNullOrEmpty(sceneDir)
                    ? Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "Assets"))
                    : sceneDir;
            }

            ExecutionOrderSettings.LoadProject(projectRoot);

            if (!TryReadNativeIdentity(RuntimeInterop.unitas_runtime_company_name, out string company))
            {
                company = string.Empty;
            }

            if (!TryReadNativeIdentity(RuntimeInterop.unitas_runtime_product_name, out string product))
            {
                product = string.Empty;
            }

            if (!TryReadNativeIdentity(RuntimeInterop.unitas_runtime_project_version, out string version))
            {
                version = string.Empty;
            }

            if (string.IsNullOrEmpty(product))
            {
                if (!string.IsNullOrEmpty(projectRoot))
                {
                    string folderName = Path.GetFileName(projectRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
                    product = string.IsNullOrEmpty(folderName) ? "UnitasProduct" : folderName;
                }
                else
                {
                    product = "UnitasProduct";
                }
            }

            _companyName = string.IsNullOrEmpty(company) ? "DefaultCompany" : company;
            _productName = product;
            _projectVersion = string.IsNullOrEmpty(version) ? "0.1" : version;
            ScanProjectResources(!string.IsNullOrEmpty(projectRoot) ? projectRoot : _dataPath);
        }

        public int ScanProjectResources(string projectOrAssetsRoot)
        {
            ThrowIfDisposed();
            if (string.IsNullOrEmpty(projectOrAssetsRoot) || !Directory.Exists(projectOrAssetsRoot))
            {
                return 0;
            }

            string fullRoot = Path.GetFullPath(projectOrAssetsRoot);
            if (!TryWriteStringToPathBuffer(fullRoot, out IntPtr buffer, out ulong length))
            {
                return 0;
            }

            int byteLength;
            try
            {
                byteLength = RuntimeInterop.unitas_runtime_resources_scan(_runtimeHandle, buffer, length);
            }
            catch (EntryPointNotFoundException)
            {
                return 0;
            }
            catch (DllNotFoundException)
            {
                return 0;
            }

            if (byteLength <= 0)
            {
                return 0;
            }

            IntPtr manifestBuffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)byteLength);
            if (manifestBuffer == IntPtr.Zero)
            {
                return 0;
            }

            string? manifest = Marshal.PtrToStringUTF8(manifestBuffer, byteLength);
            if (string.IsNullOrEmpty(manifest))
            {
                return 0;
            }

            int discoveredCount = 0;
            string[] lines = manifest.Split('\n', StringSplitOptions.RemoveEmptyEntries);
            for (int index = 0; index < lines.Length; index++)
            {
                string line = lines[index].TrimEnd('\r');
                string[] columns = line.Split('\t');
                if (columns.Length < 3)
                {
                    continue;
                }

                string resourcePath = columns[1];
                string filePath = columns[2];
                string? resourcesRoot = ExtractResourcesRootDirectory(filePath, resourcePath);
                if (!string.IsNullOrEmpty(resourcesRoot))
                {
                    Resources.AddSearchRoot(resourcesRoot);
                }

                discoveredCount++;
            }

            return discoveredCount;
        }

        private static string? ExtractResourcesRootDirectory(string filePath, string resourcePath)
        {
            if (string.IsNullOrEmpty(filePath) || string.IsNullOrEmpty(resourcePath))
            {
                return null;
            }

            string[] segments = resourcePath.Split('/', StringSplitOptions.RemoveEmptyEntries);
            if (segments.Length == 0)
            {
                return null;
            }

            string? current = filePath;
            for (int index = 0; index < segments.Length; index++)
            {
                current = Path.GetDirectoryName(current);
                if (string.IsNullOrEmpty(current))
                {
                    return null;
                }
            }

            string leaf = Path.GetFileName(current.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
            if (string.Equals(leaf, "Resources", StringComparison.OrdinalIgnoreCase))
            {
                return current;
            }

            return null;
        }

        private bool TryReadNativeIdentity(Func<ulong, int> nativeQuery, out string value)
        {
            value = string.Empty;
            int byteLength;
            try
            {
                byteLength = nativeQuery(_runtimeHandle);
            }
            catch (EntryPointNotFoundException)
            {
                return false;
            }
            catch (DllNotFoundException)
            {
                return false;
            }

            if (byteLength <= 0)
            {
                return false;
            }

            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)byteLength);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            string? decoded = Marshal.PtrToStringUTF8(buffer, byteLength);
            if (string.IsNullOrEmpty(decoded))
            {
                return false;
            }

            value = decoded;
            return true;
        }

        private static string? ResolveProjectRoot(string scenePath, out string dataPath)
        {
            string fullScenePath = File.Exists(scenePath)
                ? Path.GetFullPath(scenePath)
                : Path.Combine(Directory.GetCurrentDirectory(), scenePath);
            string? current = Path.GetDirectoryName(fullScenePath);
            string? assetsDirectory = null;

            while (!string.IsNullOrEmpty(current))
            {
                string dirName = Path.GetFileName(current);
                if (string.Equals(dirName, "Assets", StringComparison.OrdinalIgnoreCase))
                {
                    assetsDirectory = current;
                }

                string settingsCandidate = Path.Combine(current, "ProjectSettings");
                if (Directory.Exists(settingsCandidate))
                {
                    string candidateAssets = Path.Combine(current, "Assets");
                    dataPath = Directory.Exists(candidateAssets) ? candidateAssets : current;
                    return current;
                }

                current = Path.GetDirectoryName(current);
            }

            if (!string.IsNullOrEmpty(assetsDirectory))
            {
                dataPath = assetsDirectory;
                return Path.GetDirectoryName(assetsDirectory);
            }

            string? sceneDir = Path.GetDirectoryName(fullScenePath);
            dataPath = string.IsNullOrEmpty(sceneDir)
                ? Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "Assets"))
                : sceneDir;
            return null;
        }

        public void Dispose()
        {
            Shutdown();
            GC.SuppressFinalize(this);
        }

        public string? GetErrorMessage()
        {
            IntPtr errorPtr = RuntimeInterop.unitas_runtime_error_message(_runtimeHandle);
            if (errorPtr == IntPtr.Zero)
            {
                return null;
            }
            return Marshal.PtrToStringUTF8(errorPtr);
        }

        public Collider? GetCollider(ulong entity)
        {
            return ResolveColliderByEntity(entity);
        }

        public bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_raycast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask, QueryTriggerInteraction queryTriggerInteraction)
        {
            hitInfo = default;
            Vector3 delta = end - start;
            float maxDistance = delta.magnitude;
            Vector3 direction = maxDistance > 0f ? delta / maxDistance : Vector3.zero;
            int count = RuntimeInterop.unitas_runtime_linecast(
                _runtimeHandle,
                start.x,
                start.y,
                start.z,
                direction.x,
                direction.y,
                direction.z,
                maxDistance,
                (uint)layerMask,
                queryTriggerInteraction.IncludesTriggers() ? 1 : 0);
            return TakeNearestHit(count, out hitInfo);
        }

        public bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_sphere_cast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                radius,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_box_cast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                halfExtents.x,
                halfExtents.y,
                halfExtents.z,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_sphere(
                _runtimeHandle,
                position.x,
                position.y,
                position.z,
                radius,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            if (count <= 0)
            {
                return Array.Empty<Collider>();
            }
            QueryHit[] hits = ReadQueryHits(count);
            Collider[] colliders = new Collider[hits.Length];
            for (int index = 0; index < hits.Length; index++)
            {
                colliders[index] = GetCollider(hits[index].Entity)!;
            }
            return colliders;
        }

        public bool TryGetBodyPosition(ulong entity, out Vector3 position)
        {
            return TryReadBodyVector(entity, RuntimeInterop.unitas_runtime_body_position, out position);
        }

        public bool TryGetBodyVelocity(ulong entity, out Vector3 velocity)
        {
            return TryReadBodyVector(entity, RuntimeInterop.unitas_runtime_body_velocity, out velocity);
        }

        public bool TrySetBodyPosition(ulong entity, Vector3 position)
        {
            return TryWriteBodyVector(entity, RuntimeInterop.unitas_runtime_set_body_position, position)
                || TryWriteBodyVector(entity, RuntimeInterop.unitas_runtime_teleport_body_position, position);
        }

        public bool TrySetBodyVelocity(ulong entity, Vector3 velocity)
        {
            return TryWriteBodyVector(entity, RuntimeInterop.unitas_runtime_set_body_velocity, velocity);
        }

        public bool TrySetBodyRotation(ulong entity, Quaternion rotation)
        {
            return TryWriteBodyQuaternion(entity, RuntimeInterop.unitas_runtime_set_body_rotation, rotation);
        }

        public bool TrySetBodyCollisionDetectionMode(ulong entity, CollisionDetectionMode mode)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            int nativeMode = mode == CollisionDetectionMode.Discrete ? 0 : 1;
            return RuntimeInterop.unitas_runtime_body_set_collision_detection_mode(_runtimeHandle, entity, nativeMode) == 0;
        }

        public bool TryAddForce(ulong entity, Vector3 force, ForceMode mode)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[32];
            BitConverter.GetBytes((double)force.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)force.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)force.z).CopyTo(bytes, 16);
            BitConverter.GetBytes((double)mode).CopyTo(bytes, 24);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return RuntimeInterop.unitas_runtime_add_body_force(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryGetCollisionSummary(ulong a, ulong b, out CollisionSummary summary)
        {
            summary = default;
            if (_isDisposed || a == 0 || b == 0)
            {
                return false;
            }
            if (!TryReadCollisionSummary(a, b, out RuntimeInterop.AbiCollisionSummary native))
            {
                return false;
            }
            summary = new CollisionSummary
            {
                impulse = native.Impulse,
                relativeVelocity = native.RelativeVelocity,
                contactCount = (int)native.ContactCount,
                point = native.Point,
                normal = native.Normal,
            };
            return true;
        }

        public bool TryGetCollisionContact(ulong a, ulong b, uint index, out ContactPoint contact)
        {
            contact = default;
            if (_isDisposed || a == 0 || b == 0)
            {
                return false;
            }
            if (!TryReadCollisionContact(a, b, index, out RuntimeInterop.AbiCollisionContactPoint native))
            {
                return false;
            }
            contact = new ContactPoint
            {
                point = new Vector3((float)native.PointX, (float)native.PointY, (float)native.PointZ),
                normal = new Vector3((float)native.NormalX, (float)native.NormalY, (float)native.NormalZ),
                impulse = native.Impulse,
            };
            return true;
        }

        public bool TryMoveBodyPosition(ulong entity, Vector3 position)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_move_body_position(entity, position.x, position.y, position.z) == 0;
        }

        private bool TryReadCollisionSummary(ulong a, ulong b, out RuntimeInterop.AbiCollisionSummary native)
        {
            if (RuntimeInterop.unitas_runtime_collision_summary(a, b, out native) == 0)
            {
                return true;
            }
            if (a != b && RuntimeInterop.unitas_runtime_collision_summary(b, a, out native) == 0)
            {
                return true;
            }
            native = default;
            return false;
        }

        private bool TryReadCollisionContact(ulong a, ulong b, uint index, out RuntimeInterop.AbiCollisionContactPoint native)
        {
            if (RuntimeInterop.unitas_runtime_collision_contact(a, b, index, out native) == 0)
            {
                return true;
            }
            if (a != b && RuntimeInterop.unitas_runtime_collision_contact(b, a, index, out native) == 0)
            {
                return true;
            }
            native = default;
            return false;
        }

        public bool TryMove(CharacterMoveInput input, out CharacterMoveOutput output)
        {
            output = default;
            if (_isDisposed)
            {
                return false;
            }
            RuntimeInterop.AbiCharacterMoveInput abiInput = new RuntimeInterop.AbiCharacterMoveInput
            {
                EntityId = input.Entity,
                PositionX = input.Position.x,
                PositionY = input.Position.y,
                PositionZ = input.Position.z,
                MotionX = input.Motion.x,
                MotionY = input.Motion.y,
                MotionZ = input.Motion.z,
                Radius = input.Radius,
                Height = input.Height,
                CenterX = input.Center.x,
                CenterY = input.Center.y,
                CenterZ = input.Center.z,
                SlopeLimitDeg = input.SlopeLimit,
                StepOffset = input.StepOffset,
                SkinWidth = input.SkinWidth,
                MinMoveDistance = input.MinMoveDistance,
                DeltaSeconds = input.DeltaSeconds,
                LayerMask = input.LayerMask,
                Enabled = input.Enabled ? (byte)1 : (byte)0,
            };
            if (RuntimeInterop.unitas_runtime_character_move(_runtimeHandle, ref abiInput, out RuntimeInterop.AbiCharacterMoveOutput abiOutput) != 0)
            {
                return false;
            }
            output = new CharacterMoveOutput(
                new Vector3((float)abiOutput.PositionX, (float)abiOutput.PositionY, (float)abiOutput.PositionZ),
                abiOutput.GroundEntityId,
                (CollisionFlags)abiOutput.CollisionFlags,
                abiOutput.IsGrounded != 0);
            return true;
        }

        public bool TryTeleport(ulong entity, Vector3 position)
        {
            if (_isDisposed)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            BitConverter.GetBytes((double)position.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)position.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)position.z).CopyTo(bytes, 16);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return RuntimeInterop.unitas_runtime_character_teleport(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryGetConfig(ulong entity, out CharacterControllerConfig config)
        {
            config = default;
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 76);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_character_controller_config(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[76];
            Marshal.Copy(buffer, bytes, 0, bytes.Length);
            float radius = (float)BitConverter.ToDouble(bytes, 0);
            float height = (float)BitConverter.ToDouble(bytes, 8);
            float centerX = (float)BitConverter.ToDouble(bytes, 16);
            float centerY = (float)BitConverter.ToDouble(bytes, 24);
            float centerZ = (float)BitConverter.ToDouble(bytes, 32);
            float slopeLimit = (float)BitConverter.ToDouble(bytes, 40);
            float stepOffset = (float)BitConverter.ToDouble(bytes, 48);
            float skinWidth = (float)BitConverter.ToDouble(bytes, 56);
            float minMoveDistance = (float)BitConverter.ToDouble(bytes, 64);
            bool enabled = bytes[72] != 0;
            config = new CharacterControllerConfig(
                radius,
                height,
                new Vector3(centerX, centerY, centerZ),
                slopeLimit,
                stepOffset,
                skinWidth,
                minMoveDistance,
                enabled);
            return true;
        }

        private bool TryReadBodyVector(ulong entity, Func<ulong, ulong, IntPtr, int> query, out Vector3 value)
        {
            value = Vector3.zero;
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero || query(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            Marshal.Copy(buffer, bytes, 0, 24);
            float x = (float)BitConverter.ToDouble(bytes, 0);
            float y = (float)BitConverter.ToDouble(bytes, 8);
            float z = (float)BitConverter.ToDouble(bytes, 16);
            value = new Vector3(x, y, z);
            return true;
        }

        private bool TryWriteBodyVector(ulong entity, Func<ulong, ulong, IntPtr, int> setter, Vector3 value)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            BitConverter.GetBytes((double)value.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)value.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)value.z).CopyTo(bytes, 16);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return setter(_runtimeHandle, entity, buffer) == 0;
        }

        private bool TryWriteBodyQuaternion(ulong entity, Func<ulong, ulong, IntPtr, int> setter, Quaternion value)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[32];
            BitConverter.GetBytes((double)value.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)value.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)value.z).CopyTo(bytes, 16);
            BitConverter.GetBytes((double)value.w).CopyTo(bytes, 24);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return setter(_runtimeHandle, entity, buffer) == 0;
        }

        public ulong CreateSource(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0)
            {
                return 0UL;
            }
            return RuntimeInterop.unitas_runtime_audio_create(_runtimeHandle, entityId);
        }

        public void DestroySource(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_destroy(_runtimeHandle, entityId);
        }

        public void Play(ulong entityId, bool oneShot)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_play(_runtimeHandle, entityId, oneShot ? 1U : 0U);
        }

        public void Pause(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_pause(_runtimeHandle, entityId);
        }

        public void UnPause(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_unpause(_runtimeHandle, entityId);
        }

        public void Stop(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_stop(_runtimeHandle, entityId);
        }

        public void SetVolume(ulong entityId, float volume)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_volume(_runtimeHandle, entityId, volume);
        }

        public void SetPitch(ulong entityId, float pitch)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_pitch(_runtimeHandle, entityId, pitch);
        }

        public void SetMute(ulong entityId, bool mute)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_mute(_runtimeHandle, entityId, mute ? 1U : 0U);
        }

        public void SetLoop(ulong entityId, bool loop)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_loop(_runtimeHandle, entityId, loop ? 1U : 0U);
        }

        public void SetPriority(ulong entityId, int priority)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            uint clamped = (uint)Math.Clamp(priority, 0, 255);
            RuntimeInterop.unitas_runtime_audio_set_priority(_runtimeHandle, entityId, clamped);
        }

        public void SetSpatialBlend(ulong entityId, float spatialBlend)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_spatial_blend(_runtimeHandle, entityId, spatialBlend);
        }

        public void SetMinMaxDistance(ulong entityId, float minDistance, float maxDistance)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_min_max_distance(_runtimeHandle, entityId, minDistance, maxDistance);
        }

        public void SetTime(ulong entityId, double time)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_time(_runtimeHandle, entityId, time);
        }

        public void SetClip(ulong entityId, ulong clipId, double fallbackDuration)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_clip(_runtimeHandle, entityId, clipId);
            if (clipId == 0UL && fallbackDuration > 0.0)
            {
                RuntimeInterop.unitas_runtime_audio_set_custom_duration(_runtimeHandle, entityId, fallbackDuration);
            }
        }

        public void SetPosition(ulong entityId, Vector3 position)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_position(_runtimeHandle, entityId, position.x, position.y, position.z);
        }

        public AudioSourceState GetState(ulong entityId)
        {
            return GetAudioState(entityId);
        }

        public double GetTime(ulong entityId)
        {
            return GetAudioTime(entityId);
        }

        public AudioSilenceReason GetSilenceReason(ulong entityId)
        {
            return GetAudioSilenceReason(entityId);
        }

        public AudioClip CreateClip(string name, int lengthSamples, int channels, int frequency)
        {
            int clampedSamples = Math.Max(0, lengthSamples);
            int clampedChannels = Math.Max(1, channels);
            int clampedFrequency = Math.Max(1, frequency);
            float duration = (float)clampedSamples / clampedFrequency;
            ulong clipId = 0UL;
            if (!_isDisposed && _runtimeHandle != 0 && clampedSamples > 0)
            {
                ulong totalSamples = (ulong)clampedSamples * (ulong)clampedChannels;
                ulong byteLength = totalSamples * 4UL;
                IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, byteLength);
                if (buffer != IntPtr.Zero)
                {
                    byte[] sampleBytes = new byte[(int)byteLength];
                    for (int frame = 0; frame < clampedSamples; frame++)
                    {
                        float sample = (float)Math.Sin(2.0 * Math.PI * 440.0 * frame / clampedFrequency) * 0.5f;
                        byte[] floatBytes = BitConverter.GetBytes(sample);
                        for (int ch = 0; ch < clampedChannels; ch++)
                        {
                            int byteOffset = ((frame * clampedChannels) + ch) * 4;
                            Buffer.BlockCopy(floatBytes, 0, sampleBytes, byteOffset, 4);
                        }
                    }
                    Marshal.Copy(sampleBytes, 0, buffer, sampleBytes.Length);
                    clipId = RuntimeInterop.unitas_runtime_audio_register_clip_samples(
                        _runtimeHandle,
                        (uint)clampedChannels,
                        (uint)clampedFrequency,
                        (ulong)clampedSamples,
                        (ulong)buffer.ToInt64());
                }
            }

            return new AudioClip(name ?? string.Empty, duration, clampedChannels, clampedFrequency, clipId);
        }

        public AudioClip? LoadClip(string path)
        {
            if (string.IsNullOrEmpty(path) || _isDisposed || _runtimeHandle == 0)
            {
                return null;
            }
            ulong clipId = LoadAudioClip(path);
            if (clipId == 0UL)
            {
                return null;
            }
            string clipName = Path.GetFileNameWithoutExtension(path);
            if (TryGetAudioClipInfo(clipId, out uint channels, out uint sampleRate, out ulong frameCount, out double durationSeconds))
            {
                return new AudioClip(clipName, (float)durationSeconds, (int)channels, (int)sampleRate, clipId);
            }
            return new AudioClip(clipName, 0f, 1, 44100, clipId);
        }

        public ulong LoadAudioClip(string path)
        {
            ThrowIfDisposed();
            if (!TryWriteStringToPathBuffer(path, out IntPtr buffer, out ulong length))
            {
                return 0UL;
            }
            return RuntimeInterop.unitas_runtime_audio_load_clip(_runtimeHandle, (ulong)buffer.ToInt64(), length);
        }

        public AudioSourceState GetAudioState(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return AudioSourceState.Stopped;
            }
            int raw = RuntimeInterop.unitas_runtime_audio_state(_runtimeHandle, entityId);
            if (raw < 0 || raw > (int)AudioSourceState.Failed)
            {
                return AudioSourceState.Stopped;
            }
            return (AudioSourceState)raw;
        }

        public AudioSilenceReason GetAudioSilenceReason(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return AudioSilenceReason.Stopped;
            }
            int raw = RuntimeInterop.unitas_runtime_audio_silence_reason(_runtimeHandle, entityId);
            if (raw < -1 || raw > (int)AudioSilenceReason.Completed)
            {
                return AudioSilenceReason.None;
            }
            return (AudioSilenceReason)raw;
        }

        public double GetAudioTime(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return 0.0;
            }
            double time = RuntimeInterop.unitas_runtime_audio_time(_runtimeHandle, entityId);
            return time >= 0.0 ? time : 0.0;
        }

        public float[] MixAudio(int frames)
        {
            ThrowIfDisposed();
            if (frames <= 0)
            {
                return Array.Empty<float>();
            }
            ulong byteCount = (ulong)frames * 2UL * 4UL;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, byteCount);
            if (buffer == IntPtr.Zero)
            {
                return Array.Empty<float>();
            }
            ulong mixedFrames = RuntimeInterop.unitas_runtime_audio_mix(_runtimeHandle, (ulong)frames, (ulong)buffer.ToInt64());
            int totalSamples = (int)mixedFrames * 2;
            if (totalSamples <= 0)
            {
                return Array.Empty<float>();
            }
            float[] output = new float[totalSamples];
            Marshal.Copy(buffer, output, 0, totalSamples);
            return output;
        }

        public bool TryGetAudioClipInfo(ulong clipId, out uint channels, out uint sampleRate, out ulong frameCount, out double durationSeconds)
        {
            channels = 0;
            sampleRate = 0;
            frameCount = 0;
            durationSeconds = 0.0;
            if (_isDisposed || _runtimeHandle == 0 || clipId == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 16);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_audio_clip_info(_runtimeHandle, clipId, (ulong)buffer.ToInt64()) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[16];
            Marshal.Copy(buffer, bytes, 0, 16);
            durationSeconds = BitConverter.ToDouble(bytes, 0);
            sampleRate = BitConverter.ToUInt32(bytes, 8);
            channels = BitConverter.ToUInt32(bytes, 12);
            frameCount = sampleRate > 0 && durationSeconds > 0.0
                ? (ulong)Math.Round(durationSeconds * sampleRate)
                : 0UL;
            return true;
        }

        public bool TryGetAudioSourceConfig(ulong entity, out AudioSourceConfig config)
        {
            config = default;
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 44);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_audio_source_config(_runtimeHandle, entity, (ulong)buffer.ToInt64()) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[44];
            Marshal.Copy(buffer, bytes, 0, 44);
            float volume = BitConverter.ToSingle(bytes, 0);
            float pitch = BitConverter.ToSingle(bytes, 4);
            float spatialBlend = BitConverter.ToSingle(bytes, 8);
            float minDistance = BitConverter.ToSingle(bytes, 12);
            float maxDistance = BitConverter.ToSingle(bytes, 16);
            ulong clipId = BitConverter.ToUInt64(bytes, 20);
            double clipDuration = BitConverter.ToDouble(bytes, 28);
            int priority = bytes[36];
            bool looping = bytes[37] != 0;
            bool playOnAwake = bytes[38] != 0;
            bool mute = bytes[39] != 0;
            bool enabled = bytes[40] != 0;
            config = new AudioSourceConfig(
                volume,
                pitch,
                spatialBlend,
                minDistance,
                maxDistance,
                clipId,
                clipDuration,
                priority,
                looping,
                playOnAwake,
                mute,
                enabled);
            return true;
        }

        private bool _hasManagedAudioListener;

        private void SyncAudioBeforeTick()
        {
            if (_isDisposed || _runtimeHandle == 0)
            {
                return;
            }

            if (AudioListener.pause && !_isPaused)
            {
                RuntimeInterop.unitas_runtime_audio_set_paused(_runtimeHandle, 1U);
            }
            else if (!AudioListener.pause && !_isPaused)
            {
                RuntimeInterop.unitas_runtime_audio_set_paused(_runtimeHandle, 0U);
            }

            bool foundListener = false;
            AudioListener[] listeners = UnityEngine.Object.FindObjectsOfType<AudioListener>();
            for (int index = 0; index < listeners.Length; index++)
            {
                AudioListener listener = listeners[index];
                if (listener != null && listener.isActiveAndEnabled)
                {
                    Vector3 pos = listener.transform.position;
                    Vector3 fwd = listener.forward;
                    RuntimeInterop.unitas_runtime_audio_set_listener(_runtimeHandle, pos.x, pos.y, pos.z, fwd.x, fwd.y, fwd.z);
                    foundListener = true;
                    _hasManagedAudioListener = true;
                    break;
                }
            }

            if (!foundListener && _hasManagedAudioListener)
            {
                RuntimeInterop.unitas_runtime_audio_clear_listener(_runtimeHandle);
            }

            SyncAudioSourceTransforms();
        }

        public void InjectKey(KeyCode key, bool pressed)
        {
            InjectKey((uint)key, pressed);
        }

        public void InjectKey(uint keyCode, bool pressed)
        {
            int result = RuntimeInterop.unitas_runtime_input_key_event(_runtimeHandle, keyCode, pressed ? 1u : 0u);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject key event", result);
            }
        }

        public void InjectMouseButton(int button, bool pressed)
        {
            ArgumentOutOfRangeException.ThrowIfNegative(button);
            int result = RuntimeInterop.unitas_runtime_input_mouse_button_event(_runtimeHandle, (uint)button, pressed ? 1u : 0u);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject mouse button event", result);
            }
        }

        public void InjectPointerPosition(double x, double y)
        {
            int result = RuntimeInterop.unitas_runtime_input_pointer_move(_runtimeHandle, x, y);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject pointer position", result);
            }
        }

        public void InjectScroll(double x, double y)
        {
            int result = RuntimeInterop.unitas_runtime_input_scroll(_runtimeHandle, x, y);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject scroll delta", result);
            }
        }

        public void InjectAxis(string axisName, double value)
        {
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                throw new ArgumentException("axis name must be a non-empty string", nameof(axisName));
            }

            int result = RuntimeInterop.unitas_runtime_input_axis_event(_runtimeHandle, buffer, length, value);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject axis value", result);
            }
        }

        public bool GetKey(KeyCode key)
        {
            return (RuntimeInterop.unitas_runtime_input_key_state(_runtimeHandle, (uint)key) & InputStateDown) != 0;
        }

        public bool GetKeyDown(KeyCode key)
        {
            return (RuntimeInterop.unitas_runtime_input_key_state(_runtimeHandle, (uint)key) & InputStatePressed) != 0;
        }

        public bool GetKeyUp(KeyCode key)
        {
            return (RuntimeInterop.unitas_runtime_input_key_state(_runtimeHandle, (uint)key) & InputStateReleased) != 0;
        }

        public bool GetMouseButton(int button)
        {
            return button >= 0 && (RuntimeInterop.unitas_runtime_input_mouse_state(_runtimeHandle, (uint)button) & InputStateDown) != 0;
        }

        public bool GetMouseButtonDown(int button)
        {
            return button >= 0 && (RuntimeInterop.unitas_runtime_input_mouse_state(_runtimeHandle, (uint)button) & InputStatePressed) != 0;
        }

        public bool GetMouseButtonUp(int button)
        {
            return button >= 0 && (RuntimeInterop.unitas_runtime_input_mouse_state(_runtimeHandle, (uint)button) & InputStateReleased) != 0;
        }

        public Vector3 GetMousePosition()
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_input_pointer(_runtimeHandle, buffer) != 0)
            {
                return Vector3.zero;
            }
            byte[] bytes = new byte[32];
            Marshal.Copy(buffer, bytes, 0, 32);
            float x = (float)BitConverter.ToDouble(bytes, 0);
            float y = (float)BitConverter.ToDouble(bytes, 8);
            return new Vector3(x, y, 0f);
        }

        public Vector3 GetMousePositionDelta()
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_input_pointer(_runtimeHandle, buffer) != 0)
            {
                return Vector3.zero;
            }
            byte[] bytes = new byte[32];
            Marshal.Copy(buffer, bytes, 0, 32);
            float dx = (float)BitConverter.ToDouble(bytes, 16);
            float dy = (float)BitConverter.ToDouble(bytes, 24);
            return new Vector3(dx, dy, 0f);
        }

        public Vector3 GetMouseScrollDelta()
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 16);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_input_scroll_state(_runtimeHandle, buffer) != 0)
            {
                return Vector3.zero;
            }
            byte[] bytes = new byte[16];
            Marshal.Copy(buffer, bytes, 0, 16);
            float sx = (float)BitConverter.ToDouble(bytes, 0);
            float sy = (float)BitConverter.ToDouble(bytes, 8);
            return new Vector3(sx, sy, 0f);
        }

        public bool TryGetAxis(string axisName, out float value)
        {
            value = 0f;
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            double result = RuntimeInterop.unitas_runtime_input_axis(_runtimeHandle, buffer, length);
            if (RuntimeInterop.unitas_runtime_error_message(_runtimeHandle) != IntPtr.Zero)
            {
                return false;
            }
            value = (float)result;
            return true;
        }

        public bool TryGetAxisRaw(string axisName, out float value)
        {
            value = 0f;
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            double result = RuntimeInterop.unitas_runtime_input_axis_raw(_runtimeHandle, buffer, length);
            if (RuntimeInterop.unitas_runtime_error_message(_runtimeHandle) != IntPtr.Zero)
            {
                return false;
            }
            value = (float)result;
            return true;
        }

        public bool TryGetInjectedAxis(string axisName, out float value)
        {
            value = 0f;
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            double result = RuntimeInterop.unitas_runtime_input_injected_axis(_runtimeHandle, buffer, length);
            if (RuntimeInterop.unitas_runtime_error_message(_runtimeHandle) != IntPtr.Zero)
            {
                return false;
            }
            value = (float)result;
            return true;
        }

        public bool TryGetButton(string buttonName, out bool value)
        {
            return TryQueryButtonState(buttonName, InputStateDown, out value);
        }

        public bool TryGetButtonDown(string buttonName, out bool value)
        {
            return TryQueryButtonState(buttonName, InputStatePressed, out value);
        }

        public bool TryGetButtonUp(string buttonName, out bool value)
        {
            return TryQueryButtonState(buttonName, InputStateReleased, out value);
        }

        private bool TryQueryButtonState(string buttonName, uint stateMask, out bool value)
        {
            value = false;
            if (!TryWriteStringToPathBuffer(buttonName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            int mask = RuntimeInterop.unitas_runtime_input_button_state(_runtimeHandle, buffer, length);
            if (mask < 0)
            {
                return false;
            }
            value = ((uint)mask & stateMask) != 0;
            return true;
        }

        private bool TryWriteStringToPathBuffer(string text, out IntPtr buffer, out ulong length)
        {
            buffer = IntPtr.Zero;
            length = 0;
            if (string.IsNullOrEmpty(text))
            {
                return false;
            }
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

        private void ThrowRuntimeError(string prefix, int errorCode)
        {
            string? error = GetErrorMessage();
            string codeStr = errorCode.ToString(CultureInfo.InvariantCulture);
            if (string.IsNullOrEmpty(error))
            {
                throw new InvalidOperationException(prefix + " (error code " + codeStr + ").");
            }
            throw new InvalidOperationException(prefix + " (error code " + codeStr + "): " + error);
        }

        private bool TakeNearestHit(int count, out RaycastHit hit)
        {
            hit = default;
            if (count <= 0)
            {
                return false;
            }
            QueryHit[] hits = ReadQueryHits(count);
            if (hits.Length == 0)
            {
                return false;
            }
            QueryHit nearest = hits[0];
            Collider? collider = GetCollider(nearest.Entity);
            hit.collider = collider!;
            hit.point = new Vector3((float)nearest.X, (float)nearest.Y, (float)nearest.Z);
            hit.normal = new Vector3((float)nearest.NX, (float)nearest.NY, (float)nearest.NZ);
            hit.distance = (float)nearest.Distance;
            return true;
        }

        private QueryHit[] ReadQueryHits(int count)
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_query_buffer(
                _runtimeHandle,
                (uint)(count * RuntimeInterop.QueryRecordSize));
            if (buffer == IntPtr.Zero)
            {
                return Array.Empty<QueryHit>();
            }
            return RuntimeInterop.ReadQueryHits(buffer, count);
        }

        private static ulong CreateRuntimeHandle()
        {
            ulong handle = RuntimeInterop.unitas_runtime_new();
            if (handle == 0)
            {
                throw new InvalidOperationException("Failed to create native runtime instance.");
            }
            return handle;
        }

        private void WireSceneParent(ulong entity, GameObject owner)
        {
            ulong current = entity;
            GameObject currentObject = owner;
            while (_sceneParentEntities.TryGetValue(current, out ulong parentEntity))
            {
                GameObject parentObject = EnsureSceneGameObject(parentEntity);
                if (currentObject.transform.parent == null)
                {
                    currentObject.transform.SetParent(parentObject.transform, false);
                }
                current = parentEntity;
                currentObject = parentObject;
            }
        }

        private GameObject EnsureSceneGameObject(ulong entity)
        {
            if (_entityGameObjects.TryGetValue(entity, out GameObject? existing) && existing != null && !existing.IsDestroyed)
            {
                return existing;
            }
            string name = _sceneObjectNames.TryGetValue(entity, out string? parsedName) && !string.IsNullOrEmpty(parsedName)
                ? parsedName
                : "GameObject";
            GameObject created;
            using (GameObject.SuppressRuntimeCreation())
            {
                created = new GameObject(name);
            }
            created.AssignNativeEntityId(entity);
            ApplySceneTransform(entity, created);
            _entityGameObjects[entity] = created;
            _materializedSceneEntities.Add(entity);
            return created;
        }

        private void ApplySceneTransform(ulong entity, GameObject owner)
        {
            if (!_sceneTransforms.TryGetValue(entity, out SceneTransformMetadata metadata))
            {
                return;
            }

            owner.transform.SeedLocalTransformFromScene(metadata.Position, metadata.Rotation, metadata.Scale);
        }

        private GameObject? ResolveGameObjectByEntity(ulong entity)
        {
            if (_entityGameObjects.TryGetValue(entity, out GameObject? existing) && existing != null && !existing.IsDestroyed)
            {
                if (IsSceneEntity(entity))
                {
                    ApplyScenePhysicsComponents(entity, existing);
                }
                return existing;
            }

            if (!IsSceneEntity(entity))
            {
                return null;
            }

            GameObject created = EnsureSceneGameObject(entity);
            ApplyScenePhysicsComponents(entity, created);
            return created;
        }

        private Collider? ResolveColliderByEntity(ulong entity)
        {
            return ResolveGameObjectByEntity(entity)?.GetComponent<Collider>();
        }

        private bool IsSceneEntity(ulong entity)
        {
            return _sceneObjectNames.ContainsKey(entity)
                || _sceneRigidbodies.ContainsKey(entity)
                || _sceneSphereColliders.ContainsKey(entity)
                || _sceneBoxColliders.ContainsKey(entity)
                || _sceneCapsuleColliders.ContainsKey(entity)
                || _sceneMeshColliders.ContainsKey(entity);
        }

        private void ApplyScenePhysicsComponents(ulong entity, GameObject owner)
        {
            if (_sceneRigidbodies.TryGetValue(entity, out SceneRigidbodyMetadata metadata))
            {
                Rigidbody body = owner.GetComponent<Rigidbody>() ?? owner.AddComponent<Rigidbody>();
                body.mass = metadata.Mass;
                body.useGravity = metadata.UseGravity;
                body.isKinematic = metadata.IsKinematic;
            }
            if (_sceneSphereColliders.TryGetValue(entity, out SceneSphereColliderMetadata sphereMeta))
            {
                SphereCollider sphere = owner.GetComponent<SphereCollider>() ?? owner.AddComponent<SphereCollider>();
                sphere.radius = sphereMeta.Radius;
                sphere.center = sphereMeta.Center;
                sphere.isTrigger = sphereMeta.IsTrigger;
            }
            if (_sceneBoxColliders.TryGetValue(entity, out SceneBoxColliderMetadata boxMeta))
            {
                BoxCollider box = owner.GetComponent<BoxCollider>() ?? owner.AddComponent<BoxCollider>();
                box.size = boxMeta.Size;
                box.center = boxMeta.Center;
                box.isTrigger = boxMeta.IsTrigger;
            }
            if (_sceneCapsuleColliders.TryGetValue(entity, out SceneCapsuleColliderMetadata capsuleMeta))
            {
                CapsuleCollider capsule = owner.GetComponent<CapsuleCollider>() ?? owner.AddComponent<CapsuleCollider>();
                capsule.radius = capsuleMeta.Radius;
                capsule.height = capsuleMeta.Height;
                capsule.direction = capsuleMeta.Direction;
                capsule.center = capsuleMeta.Center;
                capsule.isTrigger = capsuleMeta.IsTrigger;
            }
            if (_sceneMeshColliders.TryGetValue(entity, out SceneMeshColliderMetadata meshMeta))
            {
                MeshCollider mesh = owner.GetComponent<MeshCollider>() ?? owner.AddComponent<MeshCollider>();
                mesh.convex = meshMeta.Convex;
                mesh.center = meshMeta.Center;
                mesh.isTrigger = meshMeta.IsTrigger;
                mesh.sharedMesh = ResolveSceneMesh(meshMeta.MeshGuid, meshMeta.MeshFileId);
            }
        }

        /// <summary>
        /// Resolves the mesh a scene collider names. The managed mesh is an identity handle for the
        /// asset pointer, so colliders naming the same mesh share one handle and the geometry stays
        /// owned by the native side.
        /// </summary>
        private Mesh? ResolveSceneMesh(string meshGuid, string meshFileId)
        {
            if (string.IsNullOrEmpty(meshFileId) || meshFileId == "0")
            {
                return null;
            }

            string identity = meshGuid + ":" + meshFileId;
            if (_sceneMeshes.TryGetValue(identity, out Mesh? existing) && existing != null)
            {
                return existing;
            }

            Mesh created = new Mesh();
            created.name = identity;
            _sceneMeshes[identity] = created;
            return created;
        }

        private void DropMaterializedSceneObjects()
        {
            for (int index = 0; index < _materializedSceneEntities.Count; index++)
            {
                _entityGameObjects.Remove(_materializedSceneEntities[index]);
            }
            _materializedSceneEntities.Clear();
        }

        private void DropMaterializedSceneCameras()
        {
            for (int index = 0; index < _materializedSceneCameras.Count; index++)
            {
                GameObject cameraOwner = _materializedSceneCameras[index];
                if (cameraOwner != null && !cameraOwner.IsDestroyed)
                {
                    UnityEngine.Object.Destroy(cameraOwner);
                }
            }
            _materializedSceneCameras.Clear();
            FlushDestroyedObjects();
        }

        private void ApplySceneCameraComponent(ulong entity, GameObject owner)
        {
            if (!_sceneCameras.TryGetValue(entity, out SceneCameraSettings settings))
            {
                return;
            }

            Camera camera = owner.GetComponent<Camera>() ?? owner.AddComponent<Camera>();
            camera.orthographic = settings.Orthographic;
            camera.orthographicSize = settings.OrthographicSize;
            camera.fieldOfView = settings.FieldOfView;
            camera.nearClipPlane = settings.NearClipPlane;
            camera.farClipPlane = settings.FarClipPlane;
            camera.backgroundColor = settings.BackgroundColor;
        }

        private void RecordSceneScriptOwner(
            ulong guidHi,
            ulong guidLo,
            string gameObjectFileId,
            Dictionary<string, ulong> gameObjectEntities)
        {
            if (string.IsNullOrEmpty(gameObjectFileId) || !gameObjectEntities.TryGetValue(gameObjectFileId, out ulong localEntity))
            {
                return;
            }

            if (!_sceneScriptOwners.TryGetValue((guidHi, guidLo), out List<ulong>? owners))
            {
                owners = new List<ulong>();
                _sceneScriptOwners[(guidHi, guidLo)] = owners;
            }

            owners.Add(_nextSceneEntityBase + localEntity);
        }

        /// <summary>
        /// Aligns the parsed scene's entity ids with the ids the runtime gave the
        /// scene it loaded. The runtime shifts the ids of a scene it loads after
        /// another one by the entities it already handed out, so the parsed
        /// metadata has to move with them: without the shift the managed objects
        /// lose their names, their transforms and their physics.
        /// </summary>
        private void AlignSceneEntityBase(ulong nativeEntity, ulong guidHi, ulong guidLo)
        {
            if (!_alignSceneEntityBase)
            {
                return;
            }

            if (_sceneObjectNames.ContainsKey(nativeEntity))
            {
                _alignSceneEntityBase = false;
                return;
            }

            if (!_sceneScriptOwners.TryGetValue((guidHi, guidLo), out List<ulong>? owners))
            {
                return;
            }

            for (int index = 0; index < owners.Count; index++)
            {
                if (nativeEntity < owners[index])
                {
                    continue;
                }

                ulong shift = nativeEntity - owners[index];
                if (shift != _sceneEntityKeyShift)
                {
                    ShiftSceneEntityKeys((long)shift - (long)_sceneEntityKeyShift);
                    _sceneEntityKeyShift = shift;
                }

                _alignSceneEntityBase = false;
                return;
            }
        }

        private void ShiftSceneEntityKeys(long delta)
        {
            if (delta == 0)
            {
                return;
            }

            ShiftEntityKeys(_sceneObjectNames, delta);
            ShiftEntityKeys(_sceneTransforms, delta);
            ShiftEntityKeys(_sceneRigidbodies, delta);
            ShiftEntityKeys(_sceneSphereColliders, delta);
            ShiftEntityKeys(_sceneBoxColliders, delta);
            ShiftEntityKeys(_sceneCapsuleColliders, delta);
            ShiftEntityKeys(_sceneMeshColliders, delta);
            ShiftEntityKeys(_sceneCameras, delta);
            ShiftParentEntities(delta);
            ShiftUiComponents(delta);
            ShiftMaterializedSceneEntities(delta);
            ShiftMaterializedSceneCameras();
        }

        private static void ShiftEntityKeys<T>(Dictionary<ulong, T> table, long delta)
        {
            if (table.Count == 0)
            {
                return;
            }

            List<KeyValuePair<ulong, T>> entries = new List<KeyValuePair<ulong, T>>(table);
            table.Clear();
            for (int index = 0; index < entries.Count; index++)
            {
                table[ShiftEntityKey(entries[index].Key, delta)] = entries[index].Value;
            }
        }

        private void ShiftParentEntities(long delta)
        {
            if (_sceneParentEntities.Count == 0)
            {
                return;
            }

            List<KeyValuePair<ulong, ulong>> entries = new List<KeyValuePair<ulong, ulong>>(_sceneParentEntities);
            _sceneParentEntities.Clear();
            for (int index = 0; index < entries.Count; index++)
            {
                _sceneParentEntities[ShiftEntityKey(entries[index].Key, delta)] = ShiftEntityKey(entries[index].Value, delta);
            }
        }

        private void ShiftUiComponents(long delta)
        {
            if (_sceneUiComponents.Count == 0)
            {
                return;
            }

            List<KeyValuePair<(ulong, ScriptGlueRegistry.BuiltInUiComponent), ParsedUiComponentRecord>> entries =
                new List<KeyValuePair<(ulong, ScriptGlueRegistry.BuiltInUiComponent), ParsedUiComponentRecord>>(_sceneUiComponents);
            _sceneUiComponents.Clear();
            for (int index = 0; index < entries.Count; index++)
            {
                (ulong entity, ScriptGlueRegistry.BuiltInUiComponent kind) = entries[index].Key;
                _sceneUiComponents[(ShiftEntityKey(entity, delta), kind)] = entries[index].Value;
            }
        }

        private void ShiftMaterializedSceneEntities(long delta)
        {
            for (int index = 0; index < _materializedSceneEntities.Count; index++)
            {
                ulong previous = _materializedSceneEntities[index];
                ulong shifted = ShiftEntityKey(previous, delta);
                _materializedSceneEntities[index] = shifted;
                if (_entityGameObjects.TryGetValue(previous, out GameObject? owner) && owner != null)
                {
                    _entityGameObjects.Remove(previous);
                    _entityGameObjects[shifted] = owner;
                    owner.AssignNativeEntityId(shifted);
                }
            }
        }

        private void ShiftMaterializedSceneCameras()
        {
            for (int index = 0; index < _materializedSceneCameras.Count; index++)
            {
                GameObject cameraOwner = _materializedSceneCameras[index];
                if (cameraOwner == null || cameraOwner.IsDestroyed)
                {
                    continue;
                }

                ulong shifted = cameraOwner.NativeEntityId;
                ApplySceneTransform(shifted, cameraOwner);
                ApplySceneCameraComponent(shifted, cameraOwner);
            }
        }

        private static ulong ShiftEntityKey(ulong key, long delta)
        {
            return delta >= 0 ? key + (ulong)delta : key - (ulong)(-delta);
        }

        private ulong OnCreateScript(ulong entity, ulong guidHi, ulong guidLo)
        {
            Console.Error.WriteLine("DIAG OnCreateScript entity=" + entity + " hi=" + guidHi + " lo=" + guidLo);
            AlignSceneEntityBase(entity, guidHi, guidLo);
            if (!ScriptGlueRegistry.TryCreate(guidHi, guidLo, out MonoBehaviour? script, out IUnitasMessageHandler? _) || script is null)
            {
                Console.Error.WriteLine("DIAG OnCreateScript FAILED to create for entity=" + entity);
                return 0;
            }
            _lastCreatedScript = script;
            ulong handle = _nextScriptHandle++;
            _scripts[handle] = script;
            _entityScripts[entity] = handle;
            if (!_entityScriptHandles.TryGetValue(entity, out List<ulong>? handleList))
            {
                handleList = new List<ulong>();
                _entityScriptHandles[entity] = handleList;
            }
            handleList.Add(handle);
            GameObject? owner = script.gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                if (!_entityGameObjects.TryGetValue(entity, out owner) || owner == null || owner.IsDestroyed)
                {
                    string objectName = string.IsNullOrEmpty(script.name)
                        ? (_sceneObjectNames.TryGetValue(entity, out string? parsedName) && !string.IsNullOrEmpty(parsedName)
                            ? parsedName
                            : "GameObject")
                        : script.name;
                    using (GameObject.SuppressRuntimeCreation())
                    {
                        owner = new GameObject(objectName);
                    }
                }
                owner.AttachComponent(script, script.AttachedRuntimeType ?? typeof(MonoBehaviour));
            }
            _entityGameObjects[entity] = owner;
            Console.Error.WriteLine("DIAG OnCreateScript entity=" + entity + " ownerName=" + owner.name + " ownerIsDestroyed=" + owner.IsDestroyed + " scriptName=" + script.name);
            owner.AssignNativeEntityId(entity);
            if (IsSceneEntity(entity) && !_materializedSceneEntities.Contains(entity))
            {
                _materializedSceneEntities.Add(entity);
            }
            ApplySceneTransform(entity, owner);
            WireSceneParent(entity, owner);
            ApplyScenePhysicsComponents(entity, owner);
            if (TryGetConfig(entity, out CharacterControllerConfig ccConfig))
            {
                CharacterController controller = owner.GetComponent<CharacterController>() ?? owner.AddComponent<CharacterController>();
                controller.radius = ccConfig.Radius;
                controller.height = ccConfig.Height;
                controller.center = ccConfig.Center;
                controller.slopeLimit = ccConfig.SlopeLimit;
                controller.stepOffset = ccConfig.StepOffset;
                controller.skinWidth = ccConfig.SkinWidth;
                controller.minMoveDistance = ccConfig.MinMoveDistance;
                controller.enabled = ccConfig.Enabled;
            }
            if (TryGetAudioSourceConfig(entity, out AudioSourceConfig audioConfig))
            {
                AudioSource audioSource = owner.GetComponent<AudioSource>() ?? owner.AddComponent<AudioSource>();
                AudioClip? loadedClip = null;
                if (audioConfig.ClipId != 0UL)
                {
                    if (TryGetAudioClipInfo(audioConfig.ClipId, out uint channels, out uint sampleRate, out ulong _, out double durationSeconds))
                    {
                        loadedClip = new AudioClip("SceneAudioClip", (float)durationSeconds, (int)channels, (int)sampleRate, audioConfig.ClipId);
                    }
                    else
                    {
                        loadedClip = new AudioClip("SceneAudioClip", (float)audioConfig.ClipDuration, 1, 44100, audioConfig.ClipId);
                    }
                }
                else if (audioConfig.ClipDuration > 0.0)
                {
                    loadedClip = new AudioClip("SceneAudioClip", (float)audioConfig.ClipDuration, 1, 44100, 0UL);
                }
                audioSource.ApplyNativeConfig(entity, audioConfig, loadedClip);
            }
            SyncSimulatedTransform(owner);
            ApplySceneUiComponent(entity, guidHi, guidLo, script);
            return handle;
        }

        /// <summary>
        /// Seeds the managed uGUI component with the state the loaded scene states
        /// for it, so a button that is not interactable stays that way and a label
        /// shows the text the scene gave it.
        /// </summary>
        private void ApplySceneUiComponent(ulong entity, ulong guidHi, ulong guidLo, MonoBehaviour script)
        {
            ScriptGlueRegistry.BuiltInUiComponent kind = ScriptGlueRegistry.GetBuiltInUiComponent(guidHi, guidLo);
            if (kind == ScriptGlueRegistry.BuiltInUiComponent.None)
            {
                return;
            }

            if (!_sceneUiComponents.TryGetValue((entity, kind), out ParsedUiComponentRecord state))
            {
                return;
            }

            script.enabled = state.Enabled;
            switch (kind)
            {
                case ScriptGlueRegistry.BuiltInUiComponent.Button:
                    if (script is UnityEngine.UI.Button button)
                    {
                        button.interactable = state.Interactable;
                    }

                    break;
                case ScriptGlueRegistry.BuiltInUiComponent.Text:
                    if (script is UnityEngine.UI.Text text)
                    {
                        text.fontSize = (int)state.FontSize;
                        text.color = state.Color;
                        text.text = state.Text;
                    }

                    break;
                case ScriptGlueRegistry.BuiltInUiComponent.Image:
                    if (script is UnityEngine.UI.Image image)
                    {
                        image.color = state.Color;
                        image.sprite = state.Sprite;
                    }

                    break;
                case ScriptGlueRegistry.BuiltInUiComponent.CanvasScaler:
                    if (script is UnityEngine.UI.CanvasScaler scaler)
                    {
                        scaler.uiScaleMode = (UnityEngine.UI.CanvasScaler.ScaleMode)state.UiScaleMode;
                        scaler.referenceResolution = state.ReferenceResolution;
                        scaler.screenMatchMode = (UnityEngine.UI.CanvasScaler.ScreenMatchMode)state.ScreenMatchMode;
                        scaler.matchWidthOrHeight = state.MatchWidthOrHeight;
                    }

                    break;
            }
        }

        /// <summary>Reads the high and low half of a script GUID without failing on a malformed one.</summary>
        private static bool TryParseScriptGuid(string guidHex, out ulong guidHi, out ulong guidLo)
        {
            guidHi = 0;
            guidLo = 0;
            if (guidHex.Length != 32)
            {
                return false;
            }

            if (!ulong.TryParse(guidHex.AsSpan(0, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out guidHi))
            {
                return false;
            }

            return ulong.TryParse(guidHex.AsSpan(16, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out guidLo);
        }

        private void OnAwake(ulong scriptHandle)
        {
            SyncSimulatedTransforms();
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.Awake);
            }
        }

        private void OnOnEnable(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.OnEnable);
            }
        }

        private void OnStart(ulong scriptHandle)
        {
            if (_pendingAfterSceneLoadRuntimeInit)
            {
                _pendingAfterSceneLoadRuntimeInit = false;
                RunRuntimeInitializeLoadTypeOnce(RuntimeInitializeLoadType.AfterSceneLoad);
            }
            if (_deferStartDuringSceneLoad)
            {
                _deferredStartHandles.Add(scriptHandle);
                return;
            }
            SyncSimulatedTransforms();
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.Start);
            }
        }

        private void OnFixedUpdate(ulong scriptHandle, double fixedDeltaSeconds)
        {
            SyncSimulatedTransforms();
            _fixedUpdatePhaseHandles.Add(scriptHandle);
            if (_scripts.Count > 0 && _fixedUpdatePhaseHandles.Count >= _scripts.Count)
            {
                DispatchPhaseInExecutionOrder(_fixedUpdatePhaseHandles, script => script.FixedUpdate());
                _fixedUpdatePhaseHandles.Clear();
                PumpFixedSafely();
            }
        }

        private void OnUpdate(ulong scriptHandle, double deltaSeconds)
        {
            SyncSimulatedTransforms();
            _updatePhaseHandles.Add(scriptHandle);
            if (_scripts.Count > 0 && _updatePhaseHandles.Count >= _scripts.Count)
            {
                DispatchPhaseInExecutionOrder(_updatePhaseHandles, script => script.Update());
                _updatePhaseHandles.Clear();
            }
        }

        private void OnLateUpdate(ulong scriptHandle, double deltaSeconds)
        {
            SyncSimulatedTransforms();
            _lateUpdatePhaseHandles.Add(scriptHandle);
            if (_scripts.Count > 0 && _lateUpdatePhaseHandles.Count >= _scripts.Count)
            {
                DispatchPhaseInExecutionOrder(_lateUpdatePhaseHandles, script => script.LateUpdate());
                _lateUpdatePhaseHandles.Clear();
                PumpFrameSafely();
            }
        }

        private void DispatchPhaseInExecutionOrder(List<ulong> phaseHandles, Action<MonoBehaviour> callback)
        {
            List<(MonoBehaviour Script, int Order, int Index)> pending =
                new List<(MonoBehaviour Script, int Order, int Index)>(phaseHandles.Count);
            for (int i = 0; i < phaseHandles.Count; i++)
            {
                if (_scripts.TryGetValue(phaseHandles[i], out MonoBehaviour? script) && IsEligible(script))
                {
                    pending.Add((script!, MonoBehaviour.GetEffectiveExecutionOrder(script!), i));
                }
            }
            pending.Sort((left, right) =>
            {
                int cmp = left.Order.CompareTo(right.Order);
                return cmp != 0 ? cmp : left.Index.CompareTo(right.Index);
            });
            for (int i = 0; i < pending.Count; i++)
            {
                MonoBehaviour script = pending[i].Script;
                if (!IsEligible(script))
                {
                    continue;
                }
                InvokeWithOrigin(script, () => callback(script));
            }
        }

        private void WarnOnLeftoverPhaseHandles()
        {
            if (_fixedUpdatePhaseHandles.Count + _updatePhaseHandles.Count + _lateUpdatePhaseHandles.Count > 0)
            {
                Debug.LogWarning("BridgeHost: dropping incomplete script phase from the previous tick");
            }
        }

        private void OnOnDisable(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.OnDisable);
            }
        }

        private void OnDestroy(ulong scriptHandle)
        {
            if (!_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                return;
            }

            ulong entity = EntityForScript(scriptHandle);
            if (entity != 0 && _entityScriptHandles.TryGetValue(entity, out List<ulong>? list))
            {
                list.Remove(scriptHandle);
                if (list.Count == 0)
                {
                    _entityScriptHandles.Remove(entity);
                }
            }

            if (script == null)
            {
                OwnerDestroyedSafely(script!);
                _scripts.Remove(scriptHandle);
                return;
            }

            InvokeWithOrigin(script, script.OnDestroy);
            OwnerDestroyedSafely(script);
            _scripts.Remove(scriptHandle);
        }

        private void OnCollision(ulong entityA, ulong entityB, uint phase)
        {
            SyncSimulatedTransforms();
            GameObject? otherB = ResolveGameObjectByEntity(entityB);
            GameObject? otherA = ResolveGameObjectByEntity(entityA);

            if (_entityScriptHandles.TryGetValue(entityA, out List<ulong>? handlesA))
            {
                List<ulong> snapshotA = new List<ulong>(handlesA);
                for (int i = 0; i < snapshotA.Count; i++)
                {
                    if (_scripts.TryGetValue(snapshotA[i], out MonoBehaviour? scriptA) && IsEligible(scriptA))
                    {
                        DispatchCollision(scriptA!, new Collision(otherB, entityA, entityB), phase);
                    }
                }
            }
            else
            {
                MonoBehaviour? scriptA = FindScriptByEntity(entityA);
                if (IsEligible(scriptA))
                {
                    DispatchCollision(scriptA!, new Collision(otherB, entityA, entityB), phase);
                }
            }

            if (entityB != entityA)
            {
                if (_entityScriptHandles.TryGetValue(entityB, out List<ulong>? handlesB))
                {
                    List<ulong> snapshotB = new List<ulong>(handlesB);
                    for (int i = 0; i < snapshotB.Count; i++)
                    {
                        if (_scripts.TryGetValue(snapshotB[i], out MonoBehaviour? scriptB) && IsEligible(scriptB))
                        {
                            DispatchCollision(scriptB!, new Collision(otherA, entityA, entityB), phase);
                        }
                    }
                }
                else
                {
                    MonoBehaviour? scriptB = FindScriptByEntity(entityB);
                    if (IsEligible(scriptB))
                    {
                        DispatchCollision(scriptB!, new Collision(otherA, entityA, entityB), phase);
                    }
                }
            }
        }

        private void OnTrigger(ulong entityA, ulong entityB, uint phase)
        {
            SyncSimulatedTransforms();
            Collider? colliderB = ResolveColliderByEntity(entityB);
            Collider? colliderA = ResolveColliderByEntity(entityA);

            if (_entityScriptHandles.TryGetValue(entityA, out List<ulong>? handlesA))
            {
                List<ulong> snapshotA = new List<ulong>(handlesA);
                for (int i = 0; i < snapshotA.Count; i++)
                {
                    if (_scripts.TryGetValue(snapshotA[i], out MonoBehaviour? scriptA) && IsEligible(scriptA))
                    {
                        DispatchTrigger(scriptA!, colliderB, phase);
                    }
                }
            }
            else
            {
                MonoBehaviour? scriptA = FindScriptByEntity(entityA);
                if (IsEligible(scriptA))
                {
                    DispatchTrigger(scriptA!, colliderB, phase);
                }
            }

            if (entityB != entityA)
            {
                if (_entityScriptHandles.TryGetValue(entityB, out List<ulong>? handlesB))
                {
                    List<ulong> snapshotB = new List<ulong>(handlesB);
                    for (int i = 0; i < snapshotB.Count; i++)
                    {
                        if (_scripts.TryGetValue(snapshotB[i], out MonoBehaviour? scriptB) && IsEligible(scriptB))
                        {
                            DispatchTrigger(scriptB!, colliderA, phase);
                        }
                    }
                }
                else
                {
                    MonoBehaviour? scriptB = FindScriptByEntity(entityB);
                    if (IsEligible(scriptB))
                    {
                        DispatchTrigger(scriptB!, colliderA, phase);
                    }
                }
            }
        }

        private void SyncSimulatedTransforms()
        {
            foreach (KeyValuePair<ulong, MonoBehaviour> entry in _scripts)
            {
                MonoBehaviour? script = entry.Value;
                GameObject? owner = script == null ? null : script.gameObject;
                if (owner != null)
                {
                    SyncSimulatedTransform(owner);
                }
            }
        }

        private void SyncSimulatedTransform(GameObject owner)
        {
            if (owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return;
            }
            Rigidbody? body = owner.GetComponent<Rigidbody>();
            bool hasDynamicBody = body != null && !body.isKinematic;
            bool hasDynamicMetadata = _sceneRigidbodies.TryGetValue(owner.NativeEntityId, out SceneRigidbodyMetadata metadata) && !metadata.IsKinematic;
            if (!hasDynamicBody && !hasDynamicMetadata)
            {
                return;
            }
            if (body != null && body.isKinematic)
            {
                return;
            }
            if (TryGetBodyPosition(owner.NativeEntityId, out Vector3 simulatedPosition))
            {
                owner.transform.SeedPositionFromNative(simulatedPosition);
            }
        }

        private void ParseLoadedSceneRigidbodies(string scenePath, LoadSceneMode mode)
        {
            if (mode == LoadSceneMode.Single)
            {
                _sceneObjectNames.Clear();
                _sceneRigidbodies.Clear();
                _sceneSphereColliders.Clear();
                _sceneBoxColliders.Clear();
                _sceneCapsuleColliders.Clear();
                _sceneMeshColliders.Clear();
                _sceneTransforms.Clear();
                _sceneCameras.Clear();
                _sceneUiComponents.Clear();
                _sceneScriptOwners.Clear();
                _sceneEntityKeyShift = 0;
                _alignSceneEntityBase = true;
                DropMaterializedSceneObjects();
                DropMaterializedSceneCameras();
                _nextSceneEntityBase = 1;
                foreach (KeyValuePair<ulong, MonoBehaviour> entry in _scripts)
                {
                    GameObject? owner = entry.Value == null ? null : entry.Value.gameObject;
                    if (owner != null && owner.IsPersistent && owner.NativeEntityId >= _nextSceneEntityBase)
                    {
                        _nextSceneEntityBase = owner.NativeEntityId + 1;
                    }
                }
            }

            if (string.IsNullOrEmpty(scenePath) || !File.Exists(scenePath))
            {
                return;
            }

            string[] lines = File.ReadAllLines(scenePath);
            Dictionary<string, ulong> gameObjectEntities = new Dictionary<string, ulong>(StringComparer.Ordinal);
            Dictionary<string, string> gameObjectNames = new Dictionary<string, string>(StringComparer.Ordinal);
            ulong localEntityIndex = 0;

            int currentClassId = 0;
            string currentFileId = string.Empty;
            string currentGameObjectFileId = string.Empty;
            string currentName = string.Empty;
            float currentMass = 1f;
            bool currentUseGravity = true;
            bool currentIsKinematic = false;
            float currentRadius = 0.5f;
            float currentHeight = 2f;
            int currentDirection = 1;
            bool currentIsTrigger = false;
            Vector3 currentCenter = Vector3.zero;
            Vector3 currentSize = new Vector3(1f, 1f, 1f);
            bool currentConvex = false;
            string currentMeshFileId = string.Empty;
            string currentMeshGuid = string.Empty;
            List<ParsedRigidbodyRecord> rigidbodies = new List<ParsedRigidbodyRecord>();
            List<ParsedSphereColliderRecord> sphereColliders = new List<ParsedSphereColliderRecord>();
            List<ParsedBoxColliderRecord> boxColliders = new List<ParsedBoxColliderRecord>();
            List<ParsedCapsuleColliderRecord> capsuleColliders = new List<ParsedCapsuleColliderRecord>();
            List<ParsedMeshColliderRecord> meshColliders = new List<ParsedMeshColliderRecord>();
            List<KeyValuePair<string, SceneCameraSettings>> cameras = new List<KeyValuePair<string, SceneCameraSettings>>();
            List<ParsedUiComponentRecord> uiComponents = new List<ParsedUiComponentRecord>();
            string currentUiGuid = string.Empty;
            string currentUiGameObjectFileId = string.Empty;
            bool currentUiEnabled = true;
            bool currentUiInteractable = true;
            string currentUiText = string.Empty;
            float currentUiFontSize = 14f;
            Color currentUiColor = Color.white;
            string currentUiSprite = string.Empty;
            int currentUiScaleMode = 1;
            Vector2 currentUiReferenceResolution = new Vector2(800f, 600f);
            int currentUiScreenMatchMode = 0;
            float currentUiMatchWidthOrHeight = 0.5f;
            List<ParsedTransformRecord> transforms = new List<ParsedTransformRecord>();
            List<PrefabInstanceInfo> prefabInstances = new List<PrefabInstanceInfo>();
            PrefabInstanceInfo? currentPrefabInstance = null;
            string currentModificationTarget = string.Empty;
            string? currentModificationProperty = null;
            string currentFatherFileId = string.Empty;
            Vector3 currentLocalPosition = Vector3.zero;
            Quaternion currentLocalRotation = Quaternion.identity;
            Vector3 currentLocalScale = Vector3.one;
            Color currentCameraBackground = Color.black;
            float currentCameraFieldOfView = 60f;
            float currentCameraNearClipPlane = 0.3f;
            float currentCameraFarClipPlane = 1000f;
            bool currentCameraOrthographic = false;
            float currentCameraOrthographicSize = 5f;

            for (int index = 0; index < lines.Length; index++)
            {
                string line = lines[index].Trim();
                if (line.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    if (currentClassId == 4 && currentFileId.Length > 0)
                    {
                        transforms.Add(new ParsedTransformRecord(
                            currentFileId,
                            currentGameObjectFileId,
                            currentFatherFileId,
                            currentLocalPosition,
                            currentLocalRotation,
                            currentLocalScale));
                    }
                    if (currentClassId == MONO_BEHAVIOUR_CLASS
                        && !string.IsNullOrEmpty(currentUiGuid)
                        && TryParseScriptGuid(currentUiGuid, out ulong scriptGuidHi, out ulong scriptGuidLo))
                    {
                        RecordSceneScriptOwner(scriptGuidHi, scriptGuidLo, currentUiGameObjectFileId, gameObjectEntities);
                        ScriptGlueRegistry.BuiltInUiComponent kind = ScriptGlueRegistry.GetBuiltInUiComponent(scriptGuidHi, scriptGuidLo);
                        if (kind != ScriptGlueRegistry.BuiltInUiComponent.None)
                        {
                            uiComponents.Add(new ParsedUiComponentRecord(
                                currentUiGameObjectFileId,
                                kind,
                                currentUiEnabled,
                                currentUiInteractable,
                                currentUiText,
                                currentUiFontSize,
                                currentUiColor,
                                currentUiSprite,
                                currentUiScaleMode,
                                currentUiReferenceResolution,
                                currentUiScreenMatchMode,
                                currentUiMatchWidthOrHeight));
                        }
                    }

                    FlushDocument(
                        currentClassId,
                        currentFileId,
                        currentGameObjectFileId,
                        currentName,
                        currentMass,
                        currentUseGravity,
                        currentIsKinematic,
                        currentRadius,
                        currentHeight,
                        currentDirection,
                        currentIsTrigger,
                        currentCenter,
                        currentSize,
                        currentConvex,
                        currentMeshFileId,
                        currentMeshGuid,
                        new SceneCameraSettings(currentCameraBackground, currentCameraFieldOfView, currentCameraNearClipPlane, currentCameraFarClipPlane, currentCameraOrthographic, currentCameraOrthographicSize),
                        ref localEntityIndex,
                        gameObjectEntities,
                        gameObjectNames,
                        rigidbodies,
                        sphereColliders,
                        boxColliders,
                        capsuleColliders,
                        meshColliders,
                        cameras);

                    ParseDocumentHeader(line, out currentClassId, out currentFileId);
                    currentGameObjectFileId = string.Empty;
                    currentName = string.Empty;
                    currentMass = 1f;
                    currentUseGravity = true;
                    currentIsKinematic = false;
                    currentRadius = 0.5f;
                    currentHeight = 2f;
                    currentDirection = 1;
                    currentIsTrigger = false;
                    currentCenter = Vector3.zero;
                    currentSize = new Vector3(1f, 1f, 1f);
                    currentConvex = false;
                    currentMeshFileId = string.Empty;
                    currentMeshGuid = string.Empty;
                    currentFatherFileId = string.Empty;
                    currentLocalPosition = Vector3.zero;
                    currentLocalRotation = Quaternion.identity;
                    currentLocalScale = Vector3.one;
                    currentCameraBackground = Color.black;
                    currentCameraFieldOfView = 60f;
                    currentCameraNearClipPlane = 0.3f;
                    currentCameraFarClipPlane = 1000f;
                    currentCameraOrthographic = false;
                    currentCameraOrthographicSize = 5f;
                    currentUiGuid = string.Empty;
                    currentUiGameObjectFileId = string.Empty;
                    currentUiEnabled = true;
                    currentUiInteractable = true;
                    currentUiText = string.Empty;
                    currentUiFontSize = 14f;
                    currentUiColor = Color.white;
                    currentUiSprite = string.Empty;
                    currentUiScaleMode = 1;
                    currentUiReferenceResolution = new Vector2(800f, 600f);
                    currentUiScreenMatchMode = 0;
                    currentUiMatchWidthOrHeight = 0.5f;
                    currentPrefabInstance = null;
                    currentModificationTarget = string.Empty;
                    currentModificationProperty = null;
                    if (currentClassId == PREFAB_INSTANCE_CLASS)
                    {
                        currentPrefabInstance = new PrefabInstanceInfo { FileId = currentFileId };
                        prefabInstances.Add(currentPrefabInstance);
                    }
                    continue;
                }

                if (currentClassId == 1)
                {
                    if (line.StartsWith("m_Name:", StringComparison.Ordinal))
                    {
                        currentName = ParseStringValue(line);
                    }
                }
                else if (currentClassId == 4)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        currentGameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_Father:", StringComparison.Ordinal))
                    {
                        currentFatherFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_LocalPosition:", StringComparison.Ordinal))
                    {
                        currentLocalPosition = ParseVector3Value(line, Vector3.zero);
                    }
                    else if (line.StartsWith("m_LocalRotation:", StringComparison.Ordinal))
                    {
                        currentLocalRotation = ParseQuaternionValue(line, Quaternion.identity);
                    }
                    else if (line.StartsWith("m_LocalScale:", StringComparison.Ordinal))
                    {
                        currentLocalScale = ParseVector3Value(line, Vector3.one);
                    }
                }
                else if (currentClassId == 54)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        currentGameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_Mass:", StringComparison.Ordinal))
                    {
                        currentMass = ParseFloatValue(line, 1f);
                    }
                    else if (line.StartsWith("m_UseGravity:", StringComparison.Ordinal))
                    {
                        currentUseGravity = ParseBoolValue(line, true);
                    }
                    else if (line.StartsWith("m_IsKinematic:", StringComparison.Ordinal))
                    {
                        currentIsKinematic = ParseBoolValue(line, false);
                    }
                }
                else if (currentClassId == 65 || currentClassId == 135 || currentClassId == 136)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        currentGameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_IsTrigger:", StringComparison.Ordinal))
                    {
                        currentIsTrigger = ParseBoolValue(line, false);
                    }
                    else if (line.StartsWith("m_Radius:", StringComparison.Ordinal))
                    {
                        currentRadius = ParseFloatValue(line, 0.5f);
                    }
                    else if (line.StartsWith("m_Height:", StringComparison.Ordinal))
                    {
                        currentHeight = ParseFloatValue(line, 2f);
                    }
                    else if (line.StartsWith("m_Direction:", StringComparison.Ordinal))
                    {
                        currentDirection = (int)ParseFloatValue(line, 1f);
                    }
                    else if (line.StartsWith("m_Center:", StringComparison.Ordinal))
                    {
                        currentCenter = ParseVector3Value(line, Vector3.zero);
                    }
                    else if (line.StartsWith("m_Size:", StringComparison.Ordinal))
                    {
                        currentSize = ParseVector3Value(line, new Vector3(1f, 1f, 1f));
                    }
                }
                else if (currentClassId == MESH_COLLIDER_CLASS)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        currentGameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_IsTrigger:", StringComparison.Ordinal))
                    {
                        currentIsTrigger = ParseBoolValue(line, false);
                    }
                    else if (line.StartsWith("m_Center:", StringComparison.Ordinal))
                    {
                        currentCenter = ParseVector3Value(line, Vector3.zero);
                    }
                    else if (line.StartsWith("m_Convex:", StringComparison.Ordinal))
                    {
                        currentConvex = ParseBoolValue(line, false);
                    }
                    else if (line.StartsWith("m_Mesh:", StringComparison.Ordinal))
                    {
                        currentMeshFileId = ExtractFileId(line);
                        currentMeshGuid = ExtractGuidValue(line);
                    }
                }
                else if (currentClassId == 20)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        currentGameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_BackGroundColor:", StringComparison.Ordinal))
                    {
                        currentCameraBackground = ParseColorValue(line, currentCameraBackground);
                    }
                    else if (line.StartsWith("near clip plane:", StringComparison.Ordinal))
                    {
                        currentCameraNearClipPlane = ParseFloatValue(line, currentCameraNearClipPlane);
                    }
                    else if (line.StartsWith("far clip plane:", StringComparison.Ordinal))
                    {
                        currentCameraFarClipPlane = ParseFloatValue(line, currentCameraFarClipPlane);
                    }
                    else if (line.StartsWith("field of view:", StringComparison.Ordinal))
                    {
                        currentCameraFieldOfView = ParseFloatValue(line, currentCameraFieldOfView);
                    }
                    else if (line.StartsWith("orthographic:", StringComparison.Ordinal))
                    {
                        currentCameraOrthographic = ParseBoolValue(line, currentCameraOrthographic);
                    }
                    else if (line.StartsWith("orthographic size:", StringComparison.Ordinal))
                    {
                        currentCameraOrthographicSize = ParseFloatValue(line, currentCameraOrthographicSize);
                    }
                }
                else if (currentClassId == MONO_BEHAVIOUR_CLASS)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        currentUiGameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_Script:", StringComparison.Ordinal))
                    {
                        currentUiGuid = ExtractGuidValue(line);
                    }
                    else if (line.StartsWith("m_Enabled:", StringComparison.Ordinal))
                    {
                        currentUiEnabled = ParseBoolValue(line, true);
                    }
                    else if (line.StartsWith("m_Interactable:", StringComparison.Ordinal))
                    {
                        currentUiInteractable = ParseBoolValue(line, true);
                    }
                    else if (line.StartsWith("m_Text:", StringComparison.Ordinal))
                    {
                        currentUiText = ParseStringValue(line);
                    }
                    else if (line.StartsWith("m_FontSize:", StringComparison.Ordinal))
                    {
                        currentUiFontSize = ParseFloatValue(line, 14f);
                    }
                    else if (line.StartsWith("m_Color:", StringComparison.Ordinal))
                    {
                        currentUiColor = ParseColorValue(line, currentUiColor);
                    }
                    else if (line.StartsWith("m_Sprite:", StringComparison.Ordinal))
                    {
                        currentUiSprite = ExtractGuidValue(line);
                    }
                    else if (line.StartsWith("m_UiScaleMode:", StringComparison.Ordinal))
                    {
                        currentUiScaleMode = (int)ParseFloatValue(line, 1f);
                    }
                    else if (line.StartsWith("m_ReferenceResolution:", StringComparison.Ordinal))
                    {
                        Vector3 resolution = ParseVector3Value(line, Vector3.zero);
                        currentUiReferenceResolution = new Vector2(resolution.x, resolution.y);
                    }
                    else if (line.StartsWith("m_ScreenMatchMode:", StringComparison.Ordinal))
                    {
                        currentUiScreenMatchMode = (int)ParseFloatValue(line, 0f);
                    }
                    else if (line.StartsWith("m_MatchWidthOrHeight:", StringComparison.Ordinal))
                    {
                        currentUiMatchWidthOrHeight = ParseFloatValue(line, 0.5f);
                    }
                }
                else if (currentClassId == PREFAB_INSTANCE_CLASS)
                {
                    PrefabInstanceInfo instance = currentPrefabInstance ?? new PrefabInstanceInfo();
                    if (line.StartsWith("m_SourcePrefab:", StringComparison.Ordinal))
                    {
                        instance.SourceGuid = ExtractGuidValue(line);
                    }
                    else if (line.StartsWith("m_TransformParent:", StringComparison.Ordinal))
                    {
                        instance.TransformParentFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("- target:", StringComparison.Ordinal))
                    {
                        currentModificationTarget = ExtractFileId(line);
                        currentModificationProperty = null;
                    }
                    else if (line.StartsWith("propertyPath:", StringComparison.Ordinal))
                    {
                        currentModificationProperty = ParseStringValue(line);
                    }
                    else if (line.StartsWith("value:", StringComparison.Ordinal) && currentModificationProperty != null)
                    {
                        instance.Modifications.Add(new PrefabModification(
                            currentModificationTarget,
                            currentModificationProperty,
                            ParseStringValue(line)));
                        currentModificationProperty = null;
                    }
                    else if (line.StartsWith("addedObject:", StringComparison.Ordinal))
                    {
                        string addedFileId = ExtractFileId(line);
                        if (addedFileId.Length > 0)
                        {
                            instance.AddedGameObjectFileIds.Add(addedFileId);
                        }
                    }
                }
            }

            if (currentClassId == 4 && currentFileId.Length > 0)
            {
                transforms.Add(new ParsedTransformRecord(
                    currentFileId,
                    currentGameObjectFileId,
                    currentFatherFileId,
                    currentLocalPosition,
                    currentLocalRotation,
                    currentLocalScale));
            }
            if (currentClassId == MONO_BEHAVIOUR_CLASS
                && !string.IsNullOrEmpty(currentUiGuid)
                && TryParseScriptGuid(currentUiGuid, out ulong finalScriptGuidHi, out ulong finalScriptGuidLo))
            {
                RecordSceneScriptOwner(finalScriptGuidHi, finalScriptGuidLo, currentUiGameObjectFileId, gameObjectEntities);
                ScriptGlueRegistry.BuiltInUiComponent finalKind = ScriptGlueRegistry.GetBuiltInUiComponent(finalScriptGuidHi, finalScriptGuidLo);
                if (finalKind != ScriptGlueRegistry.BuiltInUiComponent.None)
                {
                    uiComponents.Add(new ParsedUiComponentRecord(
                        currentUiGameObjectFileId,
                        finalKind,
                        currentUiEnabled,
                        currentUiInteractable,
                        currentUiText,
                        currentUiFontSize,
                        currentUiColor,
                        currentUiSprite,
                        currentUiScaleMode,
                        currentUiReferenceResolution,
                        currentUiScreenMatchMode,
                        currentUiMatchWidthOrHeight));
                }
            }
            FlushDocument(
                currentClassId,
                currentFileId,
                currentGameObjectFileId,
                currentName,
                currentMass,
                currentUseGravity,
                currentIsKinematic,
                currentRadius,
                currentHeight,
                currentDirection,
                currentIsTrigger,
                currentCenter,
                currentSize,
                currentConvex,
                currentMeshFileId,
                currentMeshGuid,
                new SceneCameraSettings(currentCameraBackground, currentCameraFieldOfView, currentCameraNearClipPlane, currentCameraFarClipPlane, currentCameraOrthographic, currentCameraOrthographicSize),
                ref localEntityIndex,
                gameObjectEntities,
                gameObjectNames,
                rigidbodies,
                sphereColliders,
                boxColliders,
                capsuleColliders,
                meshColliders,
                cameras);

            foreach (KeyValuePair<string, ulong> entry in gameObjectEntities)
            {
                ulong shiftedEntity = _nextSceneEntityBase + entry.Value;
                if (gameObjectNames.TryGetValue(entry.Key, out string? name) && !string.IsNullOrEmpty(name))
                {
                    _sceneObjectNames[shiftedEntity] = name;
                }
            }

            for (int index = 0; index < rigidbodies.Count; index++)
            {
                ParsedRigidbodyRecord record = rigidbodies[index];
                if (gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneRigidbodies[shiftedEntity] = new SceneRigidbodyMetadata(
                        record.Mass,
                        record.UseGravity,
                        record.IsKinematic);
                }
            }

            for (int index = 0; index < sphereColliders.Count; index++)
            {
                ParsedSphereColliderRecord record = sphereColliders[index];
                if (gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneSphereColliders[shiftedEntity] = new SceneSphereColliderMetadata(
                        record.Radius,
                        record.Center,
                        record.IsTrigger);
                }
            }

            for (int index = 0; index < boxColliders.Count; index++)
            {
                ParsedBoxColliderRecord record = boxColliders[index];
                if (gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneBoxColliders[shiftedEntity] = new SceneBoxColliderMetadata(
                        record.Size,
                        record.Center,
                        record.IsTrigger);
                }
            }

            for (int index = 0; index < capsuleColliders.Count; index++)
            {
                ParsedCapsuleColliderRecord record = capsuleColliders[index];
                if (gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneCapsuleColliders[shiftedEntity] = new SceneCapsuleColliderMetadata(
                        record.Radius,
                        record.Height,
                        record.Direction,
                        record.Center,
                        record.IsTrigger);
                }
            }

            for (int index = 0; index < meshColliders.Count; index++)
            {
                ParsedMeshColliderRecord record = meshColliders[index];
                if (gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneMeshColliders[shiftedEntity] = new SceneMeshColliderMetadata(
                        record.MeshFileId,
                        record.MeshGuid,
                        record.Convex,
                        record.IsTrigger,
                        record.Center);
                }
            }

            for (int index = 0; index < transforms.Count; index++)
            {
                ParsedTransformRecord record = transforms[index];
                if (gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneTransforms[shiftedEntity] = new SceneTransformMetadata(
                        record.Position,
                        record.Rotation,
                        record.Scale);
                }
            }

            for (int index = 0; index < uiComponents.Count; index++)
            {
                ParsedUiComponentRecord record = uiComponents[index];
                if (!gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong uiLocalEntity))
                {
                    continue;
                }

                _sceneUiComponents[(_nextSceneEntityBase + uiLocalEntity, record.Kind)] = record;
            }

            Dictionary<string, string> transformGameObjects = new Dictionary<string, string>(StringComparer.Ordinal);
            for (int index = 0; index < transforms.Count; index++)
            {
                transformGameObjects[transforms[index].TransformFileId] = transforms[index].GameObjectFileId;
            }
            for (int index = 0; index < transforms.Count; index++)
            {
                ParsedTransformRecord record = transforms[index];
                if (record.FatherFileId.Length == 0 || record.FatherFileId == "0")
                {
                    continue;
                }
                if (!transformGameObjects.TryGetValue(record.FatherFileId, out string? parentGameObjectFileId))
                {
                    continue;
                }
                if (!gameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong childLocal)
                    || !gameObjectEntities.TryGetValue(parentGameObjectFileId, out ulong parentLocal))
                {
                    continue;
                }
                _sceneParentEntities[_nextSceneEntityBase + childLocal] = _nextSceneEntityBase + parentLocal;
            }

            SeedPrefabInstanceTransforms(scenePath, prefabInstances, gameObjectEntities, transformGameObjects);

            for (int index = 0; index < cameras.Count; index++)
            {
                KeyValuePair<string, SceneCameraSettings> cameraEntry = cameras[index];
                if (!gameObjectEntities.TryGetValue(cameraEntry.Key, out ulong cameraLocalEntity))
                {
                    continue;
                }

                ulong cameraEntity = _nextSceneEntityBase + cameraLocalEntity;
                _sceneCameras[cameraEntity] = cameraEntry.Value;
                bool created = !_entityGameObjects.ContainsKey(cameraEntity);
                GameObject cameraOwner = EnsureSceneGameObject(cameraEntity);
                WireSceneParent(cameraEntity, cameraOwner);
                ApplySceneCameraComponent(cameraEntity, cameraOwner);
                if (created)
                {
                    _materializedSceneCameras.Add(cameraOwner);
                }
            }

            _nextSceneEntityBase += localEntityIndex;
            ulong minKey = ulong.MaxValue;
            ulong maxKey = 0;
            foreach (KeyValuePair<ulong, string> entry in _sceneObjectNames)
            {
                if (entry.Key < minKey) { minKey = entry.Key; }
                if (entry.Key > maxKey) { maxKey = entry.Key; }
            }
            Console.Error.WriteLine("DIAG ParseLoadedSceneRigidbodies base=" + _nextSceneEntityBase + " names=" + _sceneObjectNames.Count + " min=" + minKey + " max=" + maxKey);
            List<ulong> bridgeKeys = new List<ulong>(_sceneObjectNames.Keys);
            bridgeKeys.Sort();
            foreach (ulong key in bridgeKeys)
            {
                Console.Error.WriteLine("DIAG bridge key=" + key + " name=" + _sceneObjectNames[key]);
            }
        }

        private void SeedPrefabInstanceTransforms(
            string scenePath,
            List<PrefabInstanceInfo> instances,
            Dictionary<string, ulong> gameObjectEntities,
            Dictionary<string, string> transformGameObjects)
        {
            if (instances.Count == 0)
            {
                return;
            }

            PrefabGuidIndex guidIndex = PrefabGuidIndex.Build(scenePath);
            ulong nextEntity = _nextSceneEntityBase + (ulong)gameObjectEntities.Count;
            for (int index = 0; index < instances.Count; index++)
            {
                PrefabInstanceInfo instance = instances[index];
                string? prefabPath = guidIndex.Resolve(instance.SourceGuid);
                if (prefabPath == null)
                {
                    continue;
                }

                PrefabRootResolution resolution = ResolvePrefabRoot(prefabPath, guidIndex, 0);
                Vector3 position = resolution.Position;
                Quaternion rotation = resolution.Rotation;
                Vector3 scale = resolution.Scale;
                ApplyPrefabModifications(
                    ref position,
                    ref rotation,
                    ref scale,
                    instance.Modifications,
                    resolution.RootTransformFileId);
                _sceneTransforms[nextEntity] = new SceneTransformMetadata(position, rotation, scale);
                if (instance.TransformParentFileId.Length > 0
                    && transformGameObjects.TryGetValue(instance.TransformParentFileId, out string? parentGameObjectFileId)
                    && gameObjectEntities.TryGetValue(parentGameObjectFileId, out ulong parentLocalEntity))
                {
                    _sceneParentEntities[nextEntity] = _nextSceneEntityBase + parentLocalEntity;
                }

                nextEntity += (ulong)resolution.EntityCount + (ulong)instance.AddedGameObjectFileIds.Count;
            }
        }

        private static PrefabRootResolution ResolvePrefabRoot(string prefabPath, PrefabGuidIndex guidIndex, int depth)
        {
            PrefabFileInfo file = ParsePrefabFile(prefabPath);
            if (file.Instances.Count == 1 && depth < MaxPrefabInstanceDepth)
            {
                PrefabInstanceInfo variant = file.Instances[0];
                string? sourcePath = guidIndex.Resolve(variant.SourceGuid);
                if (sourcePath != null)
                {
                    PrefabRootResolution inner = ResolvePrefabRoot(sourcePath, guidIndex, depth + 1);
                    Vector3 position = inner.Position;
                    Quaternion rotation = inner.Rotation;
                    Vector3 scale = inner.Scale;
                    ApplyPrefabModifications(
                        ref position,
                        ref rotation,
                        ref scale,
                        variant.Modifications,
                        inner.RootTransformFileId);
                    return new PrefabRootResolution(
                        position,
                        rotation,
                        scale,
                        inner.EntityCount + variant.AddedGameObjectFileIds.Count,
                        inner.RootTransformFileId);
                }
            }

            return new PrefabRootResolution(
                file.RootLocalPosition,
                file.RootLocalRotation,
                file.RootLocalScale,
                file.OwnEntityCount,
                file.RootTransformFileId);
        }

        private static void ApplyPrefabModifications(
            ref Vector3 position,
            ref Quaternion rotation,
            ref Vector3 scale,
            List<PrefabModification> modifications,
            string rootTransformFileId)
        {
            for (int index = 0; index < modifications.Count; index++)
            {
                PrefabModification modification = modifications[index];
                if (!string.Equals(modification.TargetFileId, rootTransformFileId, StringComparison.Ordinal))
                {
                    continue;
                }

                string propertyPath = modification.PropertyPath;
                if (!propertyPath.StartsWith("m_LocalPosition.", StringComparison.Ordinal)
                    && !propertyPath.StartsWith("m_LocalRotation.", StringComparison.Ordinal)
                    && !propertyPath.StartsWith("m_LocalScale.", StringComparison.Ordinal))
                {
                    continue;
                }

                if (!float.TryParse(modification.Value, NumberStyles.Float, CultureInfo.InvariantCulture, out float value))
                {
                    continue;
                }

                string axis = propertyPath.Substring(propertyPath.LastIndexOf('.') + 1);
                if (propertyPath.StartsWith("m_LocalPosition.", StringComparison.Ordinal))
                {
                    position = axis switch
                    {
                        "x" => new Vector3(value, position.y, position.z),
                        "y" => new Vector3(position.x, value, position.z),
                        "z" => new Vector3(position.x, position.y, value),
                        _ => position,
                    };
                }
                else if (propertyPath.StartsWith("m_LocalRotation.", StringComparison.Ordinal))
                {
                    rotation = axis switch
                    {
                        "x" => new Quaternion(value, rotation.y, rotation.z, rotation.w),
                        "y" => new Quaternion(rotation.x, value, rotation.z, rotation.w),
                        "z" => new Quaternion(rotation.x, rotation.y, value, rotation.w),
                        "w" => new Quaternion(rotation.x, rotation.y, rotation.z, value),
                        _ => rotation,
                    };
                }
                else
                {
                    scale = axis switch
                    {
                        "x" => new Vector3(value, scale.y, scale.z),
                        "y" => new Vector3(scale.x, value, scale.z),
                        "z" => new Vector3(scale.x, scale.y, value),
                        _ => scale,
                    };
                }
            }
        }

        private static PrefabFileInfo ParsePrefabFile(string path)
        {
            PrefabFileInfo info = new PrefabFileInfo();
            string[] lines = File.ReadAllLines(path);
            int classId = 0;
            string fileId = string.Empty;
            string gameObjectFileId = string.Empty;
            string fatherFileId = string.Empty;
            string prefabInstanceOwner = string.Empty;
            Vector3 localPosition = Vector3.zero;
            Quaternion localRotation = Quaternion.identity;
            Vector3 localScale = Vector3.one;
            PrefabInstanceInfo? instance = null;
            string modificationTarget = string.Empty;
            string? modificationProperty = null;

            for (int index = 0; index < lines.Length; index++)
            {
                string line = lines[index].Trim();
                if (line.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    FlushPrefabDocument(
                        info,
                        classId,
                        fileId,
                        gameObjectFileId,
                        fatherFileId,
                        prefabInstanceOwner,
                        localPosition,
                        localRotation,
                        localScale);
                    ParseDocumentHeader(line, out classId, out fileId);
                    gameObjectFileId = string.Empty;
                    fatherFileId = string.Empty;
                    prefabInstanceOwner = string.Empty;
                    localPosition = Vector3.zero;
                    localRotation = Quaternion.identity;
                    localScale = Vector3.one;
                    instance = null;
                    modificationTarget = string.Empty;
                    modificationProperty = null;
                    if (classId == PREFAB_INSTANCE_CLASS)
                    {
                        instance = new PrefabInstanceInfo { FileId = fileId };
                        info.Instances.Add(instance);
                    }
                    continue;
                }

                if (classId == GAME_OBJECT_CLASS)
                {
                    if (line.StartsWith("m_PrefabInstance:", StringComparison.Ordinal))
                    {
                        prefabInstanceOwner = ExtractFileId(line);
                    }
                }
                else if (classId == TRANSFORM_CLASS)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        gameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_Father:", StringComparison.Ordinal))
                    {
                        fatherFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_LocalPosition:", StringComparison.Ordinal))
                    {
                        localPosition = ParseVector3Value(line, Vector3.zero);
                    }
                    else if (line.StartsWith("m_LocalRotation:", StringComparison.Ordinal))
                    {
                        localRotation = ParseQuaternionValue(line, Quaternion.identity);
                    }
                    else if (line.StartsWith("m_LocalScale:", StringComparison.Ordinal))
                    {
                        localScale = ParseVector3Value(line, Vector3.one);
                    }
                }
                else if (classId == PREFAB_INSTANCE_CLASS && instance != null)
                {
                    if (line.StartsWith("m_SourcePrefab:", StringComparison.Ordinal))
                    {
                        instance.SourceGuid = ExtractGuidValue(line);
                    }
                    else if (line.StartsWith("m_TransformParent:", StringComparison.Ordinal))
                    {
                        instance.TransformParentFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("- target:", StringComparison.Ordinal))
                    {
                        modificationTarget = ExtractFileId(line);
                        modificationProperty = null;
                    }
                    else if (line.StartsWith("propertyPath:", StringComparison.Ordinal))
                    {
                        modificationProperty = ParseStringValue(line);
                    }
                    else if (line.StartsWith("value:", StringComparison.Ordinal) && modificationProperty != null)
                    {
                        instance.Modifications.Add(new PrefabModification(
                            modificationTarget,
                            modificationProperty,
                            ParseStringValue(line)));
                        modificationProperty = null;
                    }
                    else if (line.StartsWith("addedObject:", StringComparison.Ordinal))
                    {
                        string addedFileId = ExtractFileId(line);
                        if (addedFileId.Length > 0)
                        {
                            instance.AddedGameObjectFileIds.Add(addedFileId);
                        }
                    }
                }
            }

            FlushPrefabDocument(
                info,
                classId,
                fileId,
                gameObjectFileId,
                fatherFileId,
                prefabInstanceOwner,
                localPosition,
                localRotation,
                localScale);

            foreach (PrefabInstanceInfo prefabInstance in info.Instances)
            {
                foreach (string addedFileId in prefabInstance.AddedGameObjectFileIds)
                {
                    info.AddedGameObjectFileIds.Add(addedFileId);
                }
            }

            return info;
        }

        private static void FlushPrefabDocument(
            PrefabFileInfo info,
            int classId,
            string fileId,
            string gameObjectFileId,
            string fatherFileId,
            string prefabInstanceOwner,
            Vector3 localPosition,
            Quaternion localRotation,
            Vector3 localScale)
        {
            if (classId == GAME_OBJECT_CLASS && fileId.Length > 0)
            {
                info.GameObjectFileIds.Add(fileId);
                if (prefabInstanceOwner.Length > 0 && prefabInstanceOwner != "0")
                {
                    info.GameObjectPrefabInstanceFileIds[fileId] = prefabInstanceOwner;
                }
            }
            else if (classId == TRANSFORM_CLASS && fileId.Length > 0)
            {
                if (fatherFileId.Length > 0 && fatherFileId != "0")
                {
                    return;
                }

                info.RootTransformFileId = fileId;
                info.RootGameObjectFileId = gameObjectFileId;
                info.RootLocalPosition = localPosition;
                info.RootLocalRotation = localRotation;
                info.RootLocalScale = localScale;
            }
        }

        private static string ExtractGuidValue(string line)
        {
            int index = line.IndexOf("guid:", StringComparison.Ordinal);
            if (index < 0)
            {
                return string.Empty;
            }

            string tail = line.Substring(index + 5).Trim();
            int endIndex = tail.IndexOfAny(new char[] { ',', '}', ' ' });
            return endIndex >= 0 ? tail.Substring(0, endIndex).Trim() : tail;
        }

        private static void FlushDocument(
            int classId,
            string fileId,
            string gameObjectFileId,
            string name,
            float mass,
            bool useGravity,
            bool isKinematic,
            float radius,
            float height,
            int direction,
            bool isTrigger,
            Vector3 center,
            Vector3 size,
            bool convex,
            string meshFileId,
            string meshGuid,
            SceneCameraSettings camera,
            ref ulong localEntityIndex,
            Dictionary<string, ulong> gameObjectEntities,
            Dictionary<string, string> gameObjectNames,
            List<ParsedRigidbodyRecord> rigidbodies,
            List<ParsedSphereColliderRecord> sphereColliders,
            List<ParsedBoxColliderRecord> boxColliders,
            List<ParsedCapsuleColliderRecord> capsuleColliders,
            List<ParsedMeshColliderRecord> meshColliders,
            List<KeyValuePair<string, SceneCameraSettings>> cameras)
        {
            if (classId == 1 && !string.IsNullOrEmpty(fileId))
            {
                gameObjectEntities[fileId] = localEntityIndex;
                gameObjectNames[fileId] = name;
                localEntityIndex++;
            }
            else if (classId == 54 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                rigidbodies.Add(new ParsedRigidbodyRecord(gameObjectFileId, mass, useGravity, isKinematic));
            }
            else if (classId == 65 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                boxColliders.Add(new ParsedBoxColliderRecord(gameObjectFileId, size, center, isTrigger));
            }
            else if (classId == 135 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                sphereColliders.Add(new ParsedSphereColliderRecord(gameObjectFileId, radius, center, isTrigger));
            }
            else if (classId == 136 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                capsuleColliders.Add(new ParsedCapsuleColliderRecord(gameObjectFileId, radius, height, direction, center, isTrigger));
            }
            else if (classId == MESH_COLLIDER_CLASS && !string.IsNullOrEmpty(gameObjectFileId))
            {
                meshColliders.Add(new ParsedMeshColliderRecord(gameObjectFileId, meshFileId, meshGuid, convex, isTrigger, center));
            }
            else if (classId == 20 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                cameras.Add(new KeyValuePair<string, SceneCameraSettings>(gameObjectFileId, camera));
            }
        }

        private static void ParseDocumentHeader(string headerLine, out int classId, out string fileId)
        {
            classId = 0;
            fileId = string.Empty;
            if (headerLine.EndsWith("stripped", StringComparison.Ordinal))
            {
                return;
            }
            int tagStart = headerLine.IndexOf("!u!", StringComparison.Ordinal);
            int ampIndex = headerLine.IndexOf('&', StringComparison.Ordinal);
            if (tagStart < 0 || ampIndex <= tagStart + 3)
            {
                return;
            }
            string classPart = headerLine.Substring(tagStart + 3, ampIndex - (tagStart + 3)).Trim();
            if (!int.TryParse(classPart, NumberStyles.Integer, CultureInfo.InvariantCulture, out classId))
            {
                classId = 0;
                return;
            }
            fileId = headerLine.Substring(ampIndex + 1).Trim();
        }

        private static string ExtractFileId(string line)
        {
            int keyIndex = line.IndexOf("fileID:", StringComparison.Ordinal);
            if (keyIndex < 0)
            {
                return string.Empty;
            }
            string tail = line.Substring(keyIndex + 7).Trim();
            int endIndex = tail.IndexOfAny(new char[] { ',', '}', ' ' });
            return endIndex >= 0 ? tail.Substring(0, endIndex).Trim() : tail;
        }

        private static float ParseFloatValue(string line, float fallback)
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return fallback;
            }
            string raw = line.Substring(colon + 1).Trim();
            if (float.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out float parsed))
            {
                return parsed;
            }
            return fallback;
        }

        private static bool ParseBoolValue(string line, bool fallback)
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return fallback;
            }
            string raw = line.Substring(colon + 1).Trim();
            if (string.Equals(raw, "true", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            if (string.Equals(raw, "false", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }
            if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out double numeric))
            {
                return numeric != 0.0;
            }
            return fallback;
        }

        private static string ParseStringValue(string line)
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return string.Empty;
            }
            return line.Substring(colon + 1).Trim().Trim('"', '\'');
        }

        private static Color ParseColorValue(string line, Color fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float red = fallback.r;
            float green = fallback.g;
            float blue = fallback.b;
            float alpha = fallback.a;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("r:", StringComparison.Ordinal))
                {
                    red = ParseFloatValue(part, fallback.r);
                }
                else if (part.StartsWith("g:", StringComparison.Ordinal))
                {
                    green = ParseFloatValue(part, fallback.g);
                }
                else if (part.StartsWith("b:", StringComparison.Ordinal))
                {
                    blue = ParseFloatValue(part, fallback.b);
                }
                else if (part.StartsWith("a:", StringComparison.Ordinal))
                {
                    alpha = ParseFloatValue(part, fallback.a);
                }
            }
            return new Color(red, green, blue, alpha);
        }

        private static Vector3 ParseVector3Value(string line, Vector3 fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float x = fallback.x;
            float y = fallback.y;
            float z = fallback.z;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("x:", StringComparison.Ordinal))
                {
                    x = ParseFloatValue(part, fallback.x);
                }
                else if (part.StartsWith("y:", StringComparison.Ordinal))
                {
                    y = ParseFloatValue(part, fallback.y);
                }
                else if (part.StartsWith("z:", StringComparison.Ordinal))
                {
                    z = ParseFloatValue(part, fallback.z);
                }
            }
            return new Vector3(x, y, z);
        }

        private static Quaternion ParseQuaternionValue(string line, Quaternion fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float x = fallback.x;
            float y = fallback.y;
            float z = fallback.z;
            float w = fallback.w;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("x:", StringComparison.Ordinal))
                {
                    x = ParseFloatValue(part, fallback.x);
                }
                else if (part.StartsWith("y:", StringComparison.Ordinal))
                {
                    y = ParseFloatValue(part, fallback.y);
                }
                else if (part.StartsWith("z:", StringComparison.Ordinal))
                {
                    z = ParseFloatValue(part, fallback.z);
                }
                else if (part.StartsWith("w:", StringComparison.Ordinal))
                {
                    w = ParseFloatValue(part, fallback.w);
                }
            }
            return new Quaternion(x, y, z, w);
        }

        private readonly struct SceneRigidbodyMetadata
        {
            public SceneRigidbodyMetadata(float mass, bool useGravity, bool isKinematic)
            {
                Mass = mass;
                UseGravity = useGravity;
                IsKinematic = isKinematic;
            }

            public float Mass { get; }

            public bool UseGravity { get; }

            public bool IsKinematic { get; }
        }

        private readonly struct SceneSphereColliderMetadata
        {
            public SceneSphereColliderMetadata(float radius, Vector3 center, bool isTrigger)
            {
                Radius = radius;
                Center = center;
                IsTrigger = isTrigger;
            }

            public float Radius { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneBoxColliderMetadata
        {
            public SceneBoxColliderMetadata(Vector3 size, Vector3 center, bool isTrigger)
            {
                Size = size;
                Center = center;
                IsTrigger = isTrigger;
            }

            public Vector3 Size { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneCapsuleColliderMetadata
        {
            public SceneCapsuleColliderMetadata(float radius, float height, int direction, Vector3 center, bool isTrigger)
            {
                Radius = radius;
                Height = height;
                Direction = direction;
                Center = center;
                IsTrigger = isTrigger;
            }

            public float Radius { get; }

            public float Height { get; }

            public int Direction { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneMeshColliderMetadata
        {
            public SceneMeshColliderMetadata(string meshFileId, string meshGuid, bool convex, bool isTrigger, Vector3 center)
            {
                MeshFileId = meshFileId;
                MeshGuid = meshGuid;
                Convex = convex;
                IsTrigger = isTrigger;
                Center = center;
            }

            public string MeshFileId { get; }

            public string MeshGuid { get; }

            public bool Convex { get; }

            public bool IsTrigger { get; }

            public Vector3 Center { get; }
        }

        private readonly struct ParsedTransformRecord
        {
            public ParsedTransformRecord(
                string transformFileId,
                string gameObjectFileId,
                string fatherFileId,
                Vector3 position,
                Quaternion rotation,
                Vector3 scale)
            {
                TransformFileId = transformFileId;
                GameObjectFileId = gameObjectFileId;
                FatherFileId = fatherFileId;
                Position = position;
                Rotation = rotation;
                Scale = scale;
            }

            public string TransformFileId { get; }

            public string GameObjectFileId { get; }

            public string FatherFileId { get; }

            public Vector3 Position { get; }

            public Quaternion Rotation { get; }

            public Vector3 Scale { get; }
        }

        private readonly struct SceneCameraSettings
        {
            public SceneCameraSettings(
                Color backgroundColor,
                float fieldOfView,
                float nearClipPlane,
                float farClipPlane,
                bool orthographic,
                float orthographicSize)
            {
                BackgroundColor = backgroundColor;
                FieldOfView = fieldOfView;
                NearClipPlane = nearClipPlane;
                FarClipPlane = farClipPlane;
                Orthographic = orthographic;
                OrthographicSize = orthographicSize;
            }

            public Color BackgroundColor { get; }

            public float FieldOfView { get; }

            public float NearClipPlane { get; }

            public float FarClipPlane { get; }

            public bool Orthographic { get; }

            public float OrthographicSize { get; }
        }

        private sealed class PrefabInstanceInfo
        {
            public string FileId { get; set; } = string.Empty;

            public string SourceGuid { get; set; } = string.Empty;

            public string TransformParentFileId { get; set; } = string.Empty;

            public List<PrefabModification> Modifications { get; } = new List<PrefabModification>();

            public List<string> AddedGameObjectFileIds { get; } = new List<string>();
        }

        private readonly struct PrefabModification
        {
            public PrefabModification(string targetFileId, string propertyPath, string value)
            {
                TargetFileId = targetFileId;
                PropertyPath = propertyPath;
                Value = value;
            }

            public string TargetFileId { get; }

            public string PropertyPath { get; }

            public string Value { get; }
        }

        private sealed class PrefabFileInfo
        {
            public List<string> GameObjectFileIds { get; } = new List<string>();

            public Dictionary<string, string> GameObjectPrefabInstanceFileIds { get; } = new Dictionary<string, string>(StringComparer.Ordinal);

            public HashSet<string> AddedGameObjectFileIds { get; } = new HashSet<string>(StringComparer.Ordinal);

            public List<PrefabInstanceInfo> Instances { get; } = new List<PrefabInstanceInfo>();

            public string RootTransformFileId { get; set; } = string.Empty;

            public string RootGameObjectFileId { get; set; } = string.Empty;

            public Vector3 RootLocalPosition { get; set; } = Vector3.zero;

            public Quaternion RootLocalRotation { get; set; } = Quaternion.identity;

            public Vector3 RootLocalScale { get; set; } = Vector3.one;

            public int OwnEntityCount
            {
                get
                {
                    int count = 0;
                    for (int index = 0; index < GameObjectFileIds.Count; index++)
                    {
                        string fileId = GameObjectFileIds[index];
                        if (AddedGameObjectFileIds.Contains(fileId))
                        {
                            continue;
                        }

                        if (GameObjectPrefabInstanceFileIds.TryGetValue(fileId, out string? ownerFileId)
                            && IsInstanceDocument(ownerFileId))
                        {
                            continue;
                        }

                        count++;
                    }

                    return count;
                }
            }

            private bool IsInstanceDocument(string fileId)
            {
                for (int index = 0; index < Instances.Count; index++)
                {
                    if (string.Equals(Instances[index].FileId, fileId, StringComparison.Ordinal))
                    {
                        return true;
                    }
                }

                return false;
            }
        }

        private readonly struct PrefabRootResolution
        {
            public PrefabRootResolution(
                Vector3 position,
                Quaternion rotation,
                Vector3 scale,
                int entityCount,
                string rootTransformFileId)
            {
                Position = position;
                Rotation = rotation;
                Scale = scale;
                EntityCount = entityCount;
                RootTransformFileId = rootTransformFileId;
            }

            public Vector3 Position { get; }

            public Quaternion Rotation { get; }

            public Vector3 Scale { get; }

            public int EntityCount { get; }

            public string RootTransformFileId { get; }
        }

        private sealed class PrefabGuidIndex
        {
            private readonly Dictionary<string, string> _assetPaths = new Dictionary<string, string>(StringComparer.Ordinal);

            public static PrefabGuidIndex Build(string scenePath)
            {
                PrefabGuidIndex index = new PrefabGuidIndex();
                string? projectRoot = FindProjectRoot(scenePath);
                if (projectRoot == null)
                {
                    return index;
                }

                string assets = Path.Combine(projectRoot, "Assets");
                if (!Directory.Exists(assets))
                {
                    assets = projectRoot;
                }

                string[] metaFiles = Directory.GetFiles(assets, "*.meta", SearchOption.AllDirectories);
                Array.Sort(metaFiles, StringComparer.Ordinal);
                for (int metaIndex = 0; metaIndex < metaFiles.Length; metaIndex++)
                {
                    string? guid = ReadMetaGuid(metaFiles[metaIndex]);
                    if (guid == null)
                    {
                        continue;
                    }

                    index._assetPaths[guid] = metaFiles[metaIndex].Substring(0, metaFiles[metaIndex].Length - ".meta".Length);
                }

                return index;
            }

            public string? Resolve(string guid)
            {
                if (guid.Length == 0)
                {
                    return null;
                }

                return _assetPaths.TryGetValue(guid, out string? path) ? path : null;
            }

            private static string? FindProjectRoot(string scenePath)
            {
                string? directory = Path.GetDirectoryName(Path.GetFullPath(scenePath));
                while (!string.IsNullOrEmpty(directory))
                {
                    if (Directory.Exists(Path.Combine(directory, "ProjectSettings")))
                    {
                        return directory;
                    }

                    directory = Path.GetDirectoryName(directory);
                }

                return null;
            }

            private static string? ReadMetaGuid(string metaPath)
            {
                string[] lines = File.ReadAllLines(metaPath);
                for (int index = 0; index < lines.Length; index++)
                {
                    string line = lines[index].Trim();
                    if (!line.StartsWith("guid:", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string guid = line.Substring("guid:".Length).Trim();
                    return guid.Length > 0 ? guid : null;
                }

                return null;
            }
        }

        private readonly struct SceneTransformMetadata
        {
            public SceneTransformMetadata(Vector3 position, Quaternion rotation, Vector3 scale)
            {
                Position = position;
                Rotation = rotation;
                Scale = scale;
            }

            public Vector3 Position { get; }

            public Quaternion Rotation { get; }

            public Vector3 Scale { get; }
        }

        private readonly struct ParsedRigidbodyRecord
        {
            public ParsedRigidbodyRecord(string gameObjectFileId, float mass, bool useGravity, bool isKinematic)
            {
                GameObjectFileId = gameObjectFileId;
                Mass = mass;
                UseGravity = useGravity;
                IsKinematic = isKinematic;
            }

            public string GameObjectFileId { get; }

            public float Mass { get; }

            public bool UseGravity { get; }

            public bool IsKinematic { get; }
        }

        private readonly struct ParsedUiComponentRecord
        {
            public ParsedUiComponentRecord(
                string gameObjectFileId,
                ScriptGlueRegistry.BuiltInUiComponent kind,
                bool enabled,
                bool interactable,
                string text,
                float fontSize,
                Color color,
                string sprite,
                int uiScaleMode,
                Vector2 referenceResolution,
                int screenMatchMode,
                float matchWidthOrHeight)
            {
                GameObjectFileId = gameObjectFileId;
                Kind = kind;
                Enabled = enabled;
                Interactable = interactable;
                Text = text;
                FontSize = fontSize;
                Color = color;
                Sprite = sprite;
                UiScaleMode = uiScaleMode;
                ReferenceResolution = referenceResolution;
                ScreenMatchMode = screenMatchMode;
                MatchWidthOrHeight = matchWidthOrHeight;
            }

            public string GameObjectFileId { get; }

            public ScriptGlueRegistry.BuiltInUiComponent Kind { get; }

            public bool Enabled { get; }

            public bool Interactable { get; }

            public string Text { get; }

            public float FontSize { get; }

            public Color Color { get; }

            public string Sprite { get; }

            public int UiScaleMode { get; }

            public Vector2 ReferenceResolution { get; }

            public int ScreenMatchMode { get; }

            public float MatchWidthOrHeight { get; }
        }

        private readonly struct ParsedSphereColliderRecord
        {
            public ParsedSphereColliderRecord(string gameObjectFileId, float radius, Vector3 center, bool isTrigger)
            {
                GameObjectFileId = gameObjectFileId;
                Radius = radius;
                Center = center;
                IsTrigger = isTrigger;
            }

            public string GameObjectFileId { get; }

            public float Radius { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ParsedBoxColliderRecord
        {
            public ParsedBoxColliderRecord(string gameObjectFileId, Vector3 size, Vector3 center, bool isTrigger)
            {
                GameObjectFileId = gameObjectFileId;
                Size = size;
                Center = center;
                IsTrigger = isTrigger;
            }

            public string GameObjectFileId { get; }

            public Vector3 Size { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ParsedCapsuleColliderRecord
        {
            public ParsedCapsuleColliderRecord(string gameObjectFileId, float radius, float height, int direction, Vector3 center, bool isTrigger)
            {
                GameObjectFileId = gameObjectFileId;
                Radius = radius;
                Height = height;
                Direction = direction;
                Center = center;
                IsTrigger = isTrigger;
            }

            public string GameObjectFileId { get; }

            public float Radius { get; }

            public float Height { get; }

            public int Direction { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ParsedMeshColliderRecord
        {
            public ParsedMeshColliderRecord(string gameObjectFileId, string meshFileId, string meshGuid, bool convex, bool isTrigger, Vector3 center)
            {
                GameObjectFileId = gameObjectFileId;
                MeshFileId = meshFileId;
                MeshGuid = meshGuid;
                Convex = convex;
                IsTrigger = isTrigger;
                Center = center;
            }

            public string GameObjectFileId { get; }

            public string MeshFileId { get; }

            public string MeshGuid { get; }

            public bool Convex { get; }

            public bool IsTrigger { get; }

            public Vector3 Center { get; }
        }

        private MonoBehaviour? FindScriptByEntity(ulong entity)
        {
            if (_entityScripts.TryGetValue(entity, out ulong scriptHandle) && _scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                return script;
            }
            return null;
        }

        private static void DispatchCollision(MonoBehaviour script, Collision collision, uint phase)
        {
            switch (phase)
            {
                case CollisionPhaseEnter:
                    InvokeWithOrigin(script, () => script.OnCollisionEnter(collision));
                    break;
                case CollisionPhaseStay:
                    InvokeWithOrigin(script, () => script.OnCollisionStay(collision));
                    break;
                case CollisionPhaseExit:
                    InvokeWithOrigin(script, () => script.OnCollisionExit(collision));
                    break;
                default:
                    break;
            }
        }

        private static void DispatchTrigger(MonoBehaviour script, Collider? other, uint phase)
        {
            switch (phase)
            {
                case CollisionPhaseEnter:
                    InvokeWithOrigin(script, () => script.OnTriggerEnter(other));
                    break;
                case CollisionPhaseStay:
                    InvokeWithOrigin(script, () => script.OnTriggerStay(other));
                    break;
                case CollisionPhaseExit:
                    InvokeWithOrigin(script, () => script.OnTriggerExit(other));
                    break;
                default:
                    break;
            }
        }

        private static bool IsEligible(MonoBehaviour? script)
        {
            if (script == null || !script.isActiveAndEnabled)
            {
                return false;
            }

            return true;
        }

        private static void InvokeWithOrigin(MonoBehaviour script, Action callback)
        {
            UnityEngine.Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = script;
            try
            {
                callback();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, script);
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }

        private static void PumpFixedSafely()
        {
            try
            {
                CoroutineScheduler.PumpFixed();
                UnitySynchronizationContext.PumpFixed();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        private static void PumpFrameSafely()
        {
            try
            {
                CoroutineScheduler.PumpFrame();
                InvokeScheduler.PumpFrame();
                UnitySynchronizationContext.PumpFrame();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        private static void OwnerDestroyedSafely(MonoBehaviour script)
        {
            try
            {
                CoroutineScheduler.OwnerDestroyed(script);
                InvokeScheduler.OwnerDestroyed(script);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, script);
            }
        }

        private static void StopAllSafely(MonoBehaviour script)
        {
            try
            {
                CoroutineScheduler.StopAll(script);
                InvokeScheduler.CancelAll(script);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, script);
            }
        }

        private static void StopAllSafely()
        {
            try
            {
                CoroutineScheduler.StopAll();
                InvokeScheduler.StopAll();
                UnitySynchronizationContext.ReportAndClearSession();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        private void FlushDestroyedObjects()
        {
            _destroyedObjects.Clear();
            UnityEngine.Object.FlushPendingDestroys(_destroyedObjects);
            for (int index = 0; index < _destroyedObjects.Count; index++)
            {
                UnityEngine.Object destroyed = _destroyedObjects[index];
                if (destroyed is MonoBehaviour destroyedScript)
                {
                    RemoveDestroyedScriptInstance(destroyedScript);
                }
                else if (destroyed is GameObject destroyedGo)
                {
                    IReadOnlyList<Component> comps = destroyedGo.Components;
                    for (int c = 0; c < comps.Count; c++)
                    {
                        if (comps[c] is MonoBehaviour goScript)
                        {
                            RemoveDestroyedScriptInstance(goScript);
                        }
                    }
                }
                ulong entity = UnityEngine.Object.NativeEntityIdOf(destroyed);
                if (entity != 0 && OwnsDestroyedEntity(entity, destroyed))
                {
                    NativeDestroyInterop.unitas_runtime_mark_destroyed(_runtimeHandle, entity);
                    if (destroyed is GameObject)
                    {
                        _entityGameObjects.Remove(entity);
                        _entityScripts.Remove(entity);
                        _entityScriptHandles.Remove(entity);
                        _runtimeBodySnapshots.Remove(entity);
                        _runtimeCreatedEntities.Remove(entity);
                    }
                }
            }
        }

        private void RemoveDestroyedScriptInstance(MonoBehaviour destroyedScript)
        {
            ulong matchedHandle = 0;
            foreach (KeyValuePair<ulong, MonoBehaviour> entry in _scripts)
            {
                if (ReferenceEquals(entry.Value, destroyedScript))
                {
                    matchedHandle = entry.Key;
                    break;
                }
            }
            if (matchedHandle == 0)
            {
                return;
            }
            ulong entity = EntityForScript(matchedHandle);
            _scripts.Remove(matchedHandle);
            if (entity != 0)
            {
                if (_entityScriptHandles.TryGetValue(entity, out List<ulong>? list))
                {
                    list.Remove(matchedHandle);
                    if (list.Count == 0)
                    {
                        _entityScriptHandles.Remove(entity);
                    }
                }
                if (_entityScripts.TryGetValue(entity, out ulong primary) && primary == matchedHandle)
                {
                    _entityScripts.Remove(entity);
                }
            }
        }

        private bool OwnsDestroyedEntity(ulong entity, UnityEngine.Object destroyed)
        {
            if (!_entityGameObjects.TryGetValue(entity, out GameObject? current) || current == null)
            {
                return false;
            }

            if (ReferenceEquals(current, destroyed))
            {
                return true;
            }

            return destroyed is Component component && ReferenceEquals(component.OwnerOrNull, current);
        }

        private void RegisterRuntimeGameObject(ulong entity, GameObject gameObject)
        {
            if (entity == 0 || gameObject == null)
            {
                return;
            }

            _entityGameObjects[entity] = gameObject;
            _runtimeCreatedEntities.Add(entity);
        }

        public ulong CreateRuntimeObject(string name)
        {
            if (_isDisposed || _runtimeHandle == 0 || !ReferenceEquals(_sceneBackend, _defaultSceneBackend))
            {
                return 0;
            }

            string effectiveName = string.IsNullOrEmpty(name) ? "GameObject" : name;
            if (!TryWriteStringToPathBuffer(effectiveName, out IntPtr buffer, out ulong length))
            {
                return 0;
            }

            return RuntimeInterop.unitas_runtime_object_create(_runtimeHandle, (ulong)buffer.ToInt64(), length);
        }

        public Component? CreateRuntimeScript(ulong entity, Type scriptType)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0 || scriptType == null)
            {
                return null;
            }

            if (!ScriptGlueRegistry.TryGetOrRegisterGuidForType(scriptType, out ulong guidHi, out ulong guidLo))
            {
                return null;
            }

            _lastCreatedScript = null;
            int status = RuntimeInterop.unitas_runtime_object_attach_script(_runtimeHandle, entity, guidHi, guidLo);
            if (status != 0)
            {
                return null;
            }

            MonoBehaviour? created = _lastCreatedScript;
            _lastCreatedScript = null;
            return created;
        }

        public bool SetRuntimeTransform(ulong entity, Vector3 position, Quaternion rotation, Vector3 scale)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_object_set_transform(
                _runtimeHandle,
                entity,
                position.x,
                position.y,
                position.z,
                rotation.x,
                rotation.y,
                rotation.z,
                rotation.w,
                scale.x,
                scale.y,
                scale.z) == 0;
        }

        public bool CreateRuntimeBody(
            ulong entity,
            RuntimeColliderShape shape,
            float radius,
            Vector3 halfExtents,
            float height,
            bool isTrigger)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            RuntimeBodySnapshot snapshot = new RuntimeBodySnapshot(
                shape,
                radius,
                halfExtents,
                height,
                isTrigger);
            bool ok = RuntimeInterop.unitas_runtime_object_create_body(
                _runtimeHandle,
                entity,
                (uint)shape,
                radius,
                halfExtents.x,
                halfExtents.y,
                halfExtents.z,
                height,
                isTrigger ? 1U : 0U) == 0;
            if (ok)
            {
                _runtimeBodySnapshots[entity] = snapshot;
            }

            return ok;
        }

        public bool DestroyRuntimeObject(ulong entity)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            NativeDestroyInterop.unitas_runtime_bind_destroy_channel(_runtimeHandle);
            return NativeDestroyInterop.unitas_runtime_mark_destroyed(_runtimeHandle, entity) == 0;
        }

        private void SyncRuntimeObjectsBeforeTick()
        {
            if (_isDisposed || _runtimeHandle == 0 || _runtimeCreatedEntities.Count == 0)
            {
                return;
            }

            List<ulong> entities = new List<ulong>(_runtimeCreatedEntities);
            for (int i = 0; i < entities.Count; i++)
            {
                ulong entity = entities[i];
                if (!_entityGameObjects.TryGetValue(entity, out GameObject? go) || go == null || go.IsDestroyed)
                {
                    continue;
                }

                Transform t = go.transform;
                SetRuntimeTransform(entity, t.position, t.rotation, t.lossyScale);

                SphereCollider? sphere = go.GetComponent<SphereCollider>();
                if (sphere != null)
                {
                    SyncRuntimeColliderSnapshot(
                        entity,
                        new RuntimeBodySnapshot(
                            RuntimeColliderShape.Sphere,
                            sphere.radius,
                            Vector3.zero,
                            0f,
                            sphere.isTrigger));
                    continue;
                }

                BoxCollider? box = go.GetComponent<BoxCollider>();
                if (box != null)
                {
                    SyncRuntimeColliderSnapshot(
                        entity,
                        new RuntimeBodySnapshot(
                            RuntimeColliderShape.Box,
                            0f,
                            box.size * 0.5f,
                            0f,
                            box.isTrigger));
                    continue;
                }

                CapsuleCollider? capsule = go.GetComponent<CapsuleCollider>();
                if (capsule != null)
                {
                    SyncRuntimeColliderSnapshot(
                        entity,
                        new RuntimeBodySnapshot(
                            RuntimeColliderShape.Capsule,
                            capsule.radius,
                            Vector3.zero,
                            capsule.height,
                            capsule.isTrigger));
                }
            }
        }

        private void SyncRuntimeColliderSnapshot(ulong entity, RuntimeBodySnapshot desired)
        {
            if (_runtimeBodySnapshots.TryGetValue(entity, out RuntimeBodySnapshot existing)
                && existing.Matches(desired))
            {
                return;
            }

            CreateRuntimeBody(
                entity,
                desired.Shape,
                desired.Radius,
                desired.HalfExtents,
                desired.Height,
                desired.IsTrigger);
        }

        private readonly struct RuntimeBodySnapshot
        {
            public RuntimeBodySnapshot(
                RuntimeColliderShape shape,
                float radius,
                Vector3 halfExtents,
                float height,
                bool isTrigger)
            {
                Shape = shape;
                Radius = radius;
                HalfExtents = halfExtents;
                Height = height;
                IsTrigger = isTrigger;
            }

            public RuntimeColliderShape Shape { get; }

            public float Radius { get; }

            public Vector3 HalfExtents { get; }

            public float Height { get; }

            public bool IsTrigger { get; }

            public bool Matches(RuntimeBodySnapshot other)
            {
                return Shape == other.Shape
                    && Radius == other.Radius
                    && HalfExtents == other.HalfExtents
                    && Height == other.Height
                    && IsTrigger == other.IsTrigger;
            }
        }

        private static class NativeDestroyInterop
        {
            [DllImport(RuntimeInterop.LibraryName, CallingConvention = CallingConvention.Cdecl)]
            public static extern int unitas_runtime_bind_destroy_channel(ulong runtime);

            [DllImport(RuntimeInterop.LibraryName, CallingConvention = CallingConvention.Cdecl)]
            public static extern int unitas_runtime_mark_destroyed(ulong runtime, ulong entity);
        }
    }
}
