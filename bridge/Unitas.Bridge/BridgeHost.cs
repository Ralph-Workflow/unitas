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
    public sealed partial class BridgeHost : IDisposable, IPhysicsProvider, IPhysics2DProvider, ICharacterControllerProvider, IInputProvider, ISceneHost, IApplicationHost, IAudioProvider, IRuntimeObjectProvider
    {
        private const uint CollisionPhaseEnter = 0;
        private const int GAME_OBJECT_CLASS = 1;
        private const int TRANSFORM_CLASS = 4;
        private const int AUDIO_SOURCE_CLASS = 82;
        private const int MONO_BEHAVIOUR_CLASS = 114;
        private const int MESH_COLLIDER_CLASS = 64;
        private const int PREFAB_INSTANCE_CLASS = 1001;
        private const int MaxPrefabInstanceDepth = 8;
        private const uint CollisionPhaseStay = 1;
        private const uint CollisionPhaseExit = 2;
        private const uint InputStateDown = 1;
        private const uint InputStatePressed = 2;
        private const uint InputStateReleased = 4;
        private const uint ContactPhase2DFlag = 0x100;
        private const int RIGIDBODY2D_CLASS = 50;
        private const int CIRCLE_COLLIDER2D_CLASS = 58;
        private const int BOX_COLLIDER2D_CLASS = 61;
        private const int PLATFORM_EFFECTOR2D_CLASS = 251;
        private const int SURFACE_EFFECTOR2D_CLASS = 252;

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
        private readonly Dictionary<ulong, int> _sceneObjectLayers = new Dictionary<ulong, int>();
        private readonly Dictionary<ulong, ulong> _sceneParentEntities = new Dictionary<ulong, ulong>();
        private readonly Dictionary<ulong, SceneRigidbodyMetadata> _sceneRigidbodies = new Dictionary<ulong, SceneRigidbodyMetadata>();
        private readonly Dictionary<ulong, SceneSphereColliderMetadata> _sceneSphereColliders = new Dictionary<ulong, SceneSphereColliderMetadata>();
        private readonly Dictionary<ulong, SceneBoxColliderMetadata> _sceneBoxColliders = new Dictionary<ulong, SceneBoxColliderMetadata>();
        private readonly Dictionary<ulong, SceneCapsuleColliderMetadata> _sceneCapsuleColliders = new Dictionary<ulong, SceneCapsuleColliderMetadata>();
        private readonly Dictionary<ulong, SceneMeshColliderMetadata> _sceneMeshColliders = new Dictionary<ulong, SceneMeshColliderMetadata>();
        private readonly Dictionary<ulong, SceneRigidbody2DMetadata> _sceneRigidbodies2D = new Dictionary<ulong, SceneRigidbody2DMetadata>();
        private readonly Dictionary<ulong, SceneBoxCollider2DMetadata> _sceneBoxColliders2D = new Dictionary<ulong, SceneBoxCollider2DMetadata>();
        private readonly Dictionary<ulong, SceneCircleCollider2DMetadata> _sceneCircleColliders2D = new Dictionary<ulong, SceneCircleCollider2DMetadata>();
        private readonly Dictionary<ulong, ScenePlatformEffector2DMetadata> _scenePlatformEffectors2D = new Dictionary<ulong, ScenePlatformEffector2DMetadata>();
        private readonly Dictionary<ulong, SceneSurfaceEffector2DMetadata> _sceneSurfaceEffectors2D = new Dictionary<ulong, SceneSurfaceEffector2DMetadata>();
        private readonly HashSet<ulong> _registeredBodies2D = new HashSet<ulong>();
        private readonly Dictionary<string, Mesh> _sceneMeshes = new Dictionary<string, Mesh>(StringComparer.Ordinal);
        private readonly Dictionary<ulong, SceneTransformMetadata> _sceneTransforms = new Dictionary<ulong, SceneTransformMetadata>();
        private readonly Dictionary<ulong, SceneCameraSettings> _sceneCameras = new Dictionary<ulong, SceneCameraSettings>();
        private readonly Dictionary<ulong, SceneAudioSourceMetadata> _sceneAudioSources = new Dictionary<ulong, SceneAudioSourceMetadata>();
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
        private readonly RuntimeTrailProvider _trailProvider;
        private readonly ScriptableObjectProvider _scriptableObjectProvider;
        private readonly RuntimeCinemachineProvider _cinemachineProvider;
        private ISceneProvider _sceneProvider;
        private ISceneBackend _sceneBackend;
        private static int _nextSceneHandle = 1;
        private ulong _nextScriptHandle = 1;
        private readonly List<ulong> _fixedUpdatePhaseHandles = new List<ulong>();
        private readonly List<ulong> _updatePhaseHandles = new List<ulong>();
        private readonly List<ulong> _lateUpdatePhaseHandles = new List<ulong>();
        private bool _inputSystemUpdatedThisTick;
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
            RuntimeInterop.AttachPlayerLoopHost(runtimeHandle);

            _defaultSceneProvider = new LoadedSceneFileProvider();
            _defaultSceneBackend = new NativeSceneBackend(runtimeHandle);
            _animationProvider = new RuntimeAnimationProvider(runtimeHandle);
            _renderProvider = new RuntimeRenderProvider(runtimeHandle);
            _uiProvider = new RuntimeUiProvider(runtimeHandle);
            _trailProvider = new RuntimeTrailProvider(runtimeHandle);
            _scriptableObjectProvider = new ScriptableObjectProvider(runtimeHandle);
            _cinemachineProvider = new RuntimeCinemachineProvider();
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
            Physics2D.Provider = this;
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
            ScriptableObject.Provider = _scriptableObjectProvider;
            Cinemachine.CinemachineProvider.Current = _cinemachineProvider;
            Animator.Provider = _animationProvider;
            Camera.Provider = _renderProvider;
            Mesh.Provider = _renderProvider;
            Material.Provider = _renderProvider;
            MeshFilter.Provider = _renderProvider;
            MeshRenderer.Provider = _renderProvider;
            Light.Provider = _renderProvider;
            UnityEngine.UI.Ui.Provider = _uiProvider;
            TrailRenderer.Provider = _trailProvider;
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
            _inputSystemUpdatedThisTick = false;
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

            EnsureInputSystemUpdatedForTick();

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
                RuntimeInterop.DetachPlayerLoopHost(_runtimeHandle);
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
                _sceneObjectLayers.Clear();
                _sceneParentEntities.Clear();
                _sceneRigidbodies.Clear();
                _sceneSphereColliders.Clear();
                _sceneBoxColliders.Clear();
                _sceneCapsuleColliders.Clear();
                _sceneMeshColliders.Clear();
                _sceneRigidbodies2D.Clear();
                _sceneBoxColliders2D.Clear();
                _sceneCircleColliders2D.Clear();
                _sceneCapsuleColliders2D.Clear();
                _scenePolygonColliders2D.Clear();
                _sceneEdgeColliders2D.Clear();
                _sceneCompositeColliders2D.Clear();
                _sceneTilemapColliders2D.Clear();
                _scenePlatformEffectors2D.Clear();
                _sceneSurfaceEffectors2D.Clear();
                _registeredBodies2D.Clear();
                _sceneTransforms.Clear();
                _sceneCameras.Clear();
                _sceneAudioSources.Clear();
                _sceneUiComponents.Clear();
                StopAllSafely();
                if (ReferenceEquals(Physics.Provider, this))
                {
                    Physics.Provider = null;
                }
                if (ReferenceEquals(Physics2D.Provider, this))
                {
                    Physics2D.Provider = null;
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
                if (ReferenceEquals(ScriptableObject.Provider, _scriptableObjectProvider))
                {
                    ScriptableObject.Provider = null;
                }
                if (ReferenceEquals(Cinemachine.CinemachineProvider.Current, _cinemachineProvider))
                {
                    Cinemachine.CinemachineProvider.Current = null;
                }
                if (ReferenceEquals(Animator.Provider, _animationProvider))
                {
                    Animator.Provider = null;
                }
                if (ReferenceEquals(UnityEngine.UI.Ui.Provider, _uiProvider))
                {
                    UnityEngine.UI.Ui.Provider = null;
                }
                if (ReferenceEquals(TrailRenderer.Provider, _trailProvider))
                {
                    TrailRenderer.Provider = null;
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

        private static ulong CreateRuntimeHandle()
        {
            ulong handle = RuntimeInterop.unitas_runtime_new();
            if (handle == 0)
            {
                throw new InvalidOperationException("Failed to create native runtime instance.");
            }
            return handle;
        }

        private ulong OnCreateScript(ulong entity, ulong guidHi, ulong guidLo)
        {
            AlignSceneEntityBase(entity, guidHi, guidLo);
            if (!ScriptGlueRegistry.TryCreate(guidHi, guidLo, out MonoBehaviour? script, out IUnitasMessageHandler? _) || script is null)
            {
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
            ApplySceneAudioSourceComponent(entity, owner);
            SyncSimulatedTransform(owner);
            ApplySceneUiComponent(entity, guidHi, guidLo, script);
            return handle;
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
            EnsureInputSystemUpdatedForTick();
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
            EnsureInputSystemUpdatedForTick();
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
            EnsureInputSystemUpdatedForTick();
            SyncSimulatedTransforms();
            _lateUpdatePhaseHandles.Add(scriptHandle);
            if (_scripts.Count > 0 && _lateUpdatePhaseHandles.Count >= _scripts.Count)
            {
                DispatchPhaseInExecutionOrder(_lateUpdatePhaseHandles, script => script.LateUpdate());
                _lateUpdatePhaseHandles.Clear();
                PumpFrameSafely();
            }
        }

        private void EnsureInputSystemUpdatedForTick()
        {
            if (_inputSystemUpdatedThisTick)
            {
                return;
            }
            _inputSystemUpdatedThisTick = true;
            UnityEngine.InputSystem.InputSystem.Update();
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
            bool is2d = (phase & ContactPhase2DFlag) != 0;
            uint phaseCode = phase & ~ContactPhase2DFlag;
            if (is2d)
            {
                DispatchCollision2DToRecipient(entityA, entityB, phaseCode);
                return;
            }
            GameObject? otherB = ResolveGameObjectByEntity(entityB);
            GameObject? otherA = ResolveGameObjectByEntity(entityA);

            if (_entityScriptHandles.TryGetValue(entityA, out List<ulong>? handlesA))
            {
                List<ulong> snapshotA = new List<ulong>(handlesA);
                for (int i = 0; i < snapshotA.Count; i++)
                {
                    if (_scripts.TryGetValue(snapshotA[i], out MonoBehaviour? scriptA) && IsEligible(scriptA))
                    {
                        DispatchCollision(scriptA!, new Collision(otherB, entityA, entityB), phaseCode);
                    }
                }
            }
            else
            {
                MonoBehaviour? scriptA = FindScriptByEntity(entityA);
                if (IsEligible(scriptA))
                {
                    DispatchCollision(scriptA!, new Collision(otherB, entityA, entityB), phaseCode);
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
                            DispatchCollision(scriptB!, new Collision(otherA, entityA, entityB), phaseCode);
                        }
                    }
                }
                else
                {
                    MonoBehaviour? scriptB = FindScriptByEntity(entityB);
                    if (IsEligible(scriptB))
                    {
                        DispatchCollision(scriptB!, new Collision(otherA, entityA, entityB), phaseCode);
                    }
                }
            }
        }

        private void OnTrigger(ulong entityA, ulong entityB, uint phase)
        {
            SyncSimulatedTransforms();
            bool is2d = (phase & ContactPhase2DFlag) != 0;
            uint phaseCode = phase & ~ContactPhase2DFlag;
            if (is2d)
            {
                DispatchTrigger2DToRecipient(entityA, entityB, phaseCode);
                return;
            }
            Collider? colliderB = ResolveColliderByEntity(entityB);
            Collider? colliderA = ResolveColliderByEntity(entityA);

            if (_entityScriptHandles.TryGetValue(entityA, out List<ulong>? handlesA))
            {
                List<ulong> snapshotA = new List<ulong>(handlesA);
                for (int i = 0; i < snapshotA.Count; i++)
                {
                    if (_scripts.TryGetValue(snapshotA[i], out MonoBehaviour? scriptA) && IsEligible(scriptA))
                    {
                        DispatchTrigger(scriptA!, colliderB, phaseCode);
                    }
                }
            }
            else
            {
                MonoBehaviour? scriptA = FindScriptByEntity(entityA);
                if (IsEligible(scriptA))
                {
                    DispatchTrigger(scriptA!, colliderB, phaseCode);
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
                            DispatchTrigger(scriptB!, colliderA, phaseCode);
                        }
                    }
                }
                else
                {
                    MonoBehaviour? scriptB = FindScriptByEntity(entityB);
                    if (IsEligible(scriptB))
                    {
                        DispatchTrigger(scriptB!, colliderA, phaseCode);
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
            Rigidbody2D? body2D = owner.GetComponent<Rigidbody2D>();
            bool hasSimulatedBody2D = body2D != null && body2D.bodyType != RigidbodyType2D.Static;
            bool hasSimulatedMetadata2D = _sceneRigidbodies2D.TryGetValue(owner.NativeEntityId, out SceneRigidbody2DMetadata metadata2D) && metadata2D.BodyType != RigidbodyType2D.Static;
            if (!hasDynamicBody && !hasDynamicMetadata && !hasSimulatedBody2D && !hasSimulatedMetadata2D)
            {
                return;
            }
            if (body != null && body.isKinematic && !hasSimulatedBody2D && !hasSimulatedMetadata2D)
            {
                return;
            }
            if (TryGetBodyPosition(owner.NativeEntityId, out Vector3 simulatedPosition))
            {
                owner.transform.SeedPositionFromNative(simulatedPosition);
            }
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

        private void DispatchCollision2DToRecipient(ulong entityA, ulong entityB, uint phase)
        {
            GameObject? other = ResolveGameObjectByEntity(entityB);
            if (_entityScriptHandles.TryGetValue(entityA, out List<ulong>? handles))
            {
                List<ulong> snapshot = new List<ulong>(handles);
                for (int index = 0; index < snapshot.Count; index++)
                {
                    if (_scripts.TryGetValue(snapshot[index], out MonoBehaviour? script) && IsEligible(script))
                    {
                        DispatchCollision2D(script!, new Collision2D(other, entityA, entityB), phase);
                    }
                }
                return;
            }
            MonoBehaviour? scriptA = FindScriptByEntity(entityA);
            if (IsEligible(scriptA))
            {
                DispatchCollision2D(scriptA!, new Collision2D(other, entityA, entityB), phase);
            }
        }

        private static void DispatchCollision2D(MonoBehaviour script, Collision2D collision, uint phase)
        {
            switch (phase)
            {
                case CollisionPhaseEnter:
                    InvokeWithOrigin(script, () => script.OnCollisionEnter2D(collision));
                    break;
                case CollisionPhaseStay:
                    InvokeWithOrigin(script, () => script.OnCollisionStay2D(collision));
                    break;
                case CollisionPhaseExit:
                    InvokeWithOrigin(script, () => script.OnCollisionExit2D(collision));
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

        private void DispatchTrigger2DToRecipient(ulong entityA, ulong entityB, uint phase)
        {
            Collider2D? other = ResolveCollider2DByEntity(entityB);
            if (_entityScriptHandles.TryGetValue(entityA, out List<ulong>? handles))
            {
                List<ulong> snapshot = new List<ulong>(handles);
                for (int index = 0; index < snapshot.Count; index++)
                {
                    if (_scripts.TryGetValue(snapshot[index], out MonoBehaviour? script) && IsEligible(script))
                    {
                        DispatchTrigger2D(script!, other, phase);
                    }
                }
                return;
            }
            MonoBehaviour? scriptA = FindScriptByEntity(entityA);
            if (IsEligible(scriptA))
            {
                DispatchTrigger2D(scriptA!, other, phase);
            }
        }

        private static void DispatchTrigger2D(MonoBehaviour script, Collider2D? other, uint phase)
        {
            switch (phase)
            {
                case CollisionPhaseEnter:
                    InvokeWithOrigin(script, () => script.OnTriggerEnter2D(other));
                    break;
                case CollisionPhaseStay:
                    InvokeWithOrigin(script, () => script.OnTriggerStay2D(other));
                    break;
                case CollisionPhaseExit:
                    InvokeWithOrigin(script, () => script.OnTriggerExit2D(other));
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
