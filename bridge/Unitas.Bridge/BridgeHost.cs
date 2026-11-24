using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed class BridgeHost : IDisposable, IPhysicsProvider, IInputProvider, ISceneHost, IApplicationHost
    {
        private const uint CollisionPhaseEnter = 0;
        private const uint CollisionPhaseStay = 1;
        private const uint CollisionPhaseExit = 2;
        private const uint InputStateDown = 1;
        private const uint InputStatePressed = 2;
        private const uint InputStateReleased = 4;

        private readonly ulong _runtimeHandle;
        private readonly Dictionary<ulong, MonoBehaviour> _scripts = new Dictionary<ulong, MonoBehaviour>();
        private readonly Dictionary<ulong, ulong> _entityScripts = new Dictionary<ulong, ulong>();
        private readonly Dictionary<ulong, SceneRigidbodyMetadata> _sceneRigidbodies = new Dictionary<ulong, SceneRigidbodyMetadata>();
        private ulong _nextSceneEntityBase = 1;
        private readonly List<Scene> _loadedScenes = new List<Scene>();
        private readonly Dictionary<string, int> _sceneHandles = new Dictionary<string, int>(StringComparer.Ordinal);
        private readonly LoadedSceneFileProvider _defaultSceneProvider;
        private readonly NativeSceneBackend _defaultSceneBackend;
        private ISceneProvider _sceneProvider;
        private ISceneBackend _sceneBackend;
        private int _nextSceneHandle = 1;
        private ulong _nextScriptHandle = 1;
        private int _fixedUpdateCallsInStep;
        private int _lateUpdateCallsInTick;
        private bool _isDisposed;
        private bool _isQuitting;
        private bool _isFocused = true;
        private bool _isPaused;
        private ApplicationBuildKind _buildKind = ApplicationBuildKind.Editor;
        private string _dataPath = string.Empty;
        private string _companyName = string.Empty;
        private string _productName = string.Empty;
        private string _projectVersion = string.Empty;

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
            Input.Provider = this;
            SceneManager.Host = this;
            Application.Host = this;
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

            DropReplacedScripts();
            ParseLoadedSceneRigidbodies(path, LoadSceneMode.Single);
            _defaultSceneProvider.SetSceneRoot(path);
            UpdateProjectMetadata(path);
            RegisterLoadedScene(path, LoadSceneMode.Single);
            SceneManager.RaiseSceneLoaded(FindLoadedScene(path), LoadSceneMode.Single);
        }

        public Scene LoadScene(string path, LoadSceneMode mode)
        {
            ThrowIfDisposed();
            ArgumentNullException.ThrowIfNull(path);

            try
            {
                Backend.LoadScene(path, mode);
            }
            catch (SceneManagementException)
            {
                throw;
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                throw new SceneManagementException("Failed to load scene '" + path + "': " + ex.Message, ex);
            }

            if (mode == LoadSceneMode.Single)
            {
                DropReplacedScripts();
            }

            ParseLoadedSceneRigidbodies(path, mode);
            _defaultSceneProvider.SetSceneRoot(path);
            UpdateProjectMetadata(path);
            RegisterLoadedScene(path, mode);
            Scene loaded = FindLoadedScene(path);
            SceneManager.RaiseSceneLoaded(loaded, mode);
            return loaded;
        }

        public void UnloadScene(string name)
        {
            ThrowIfDisposed();
            ArgumentNullException.ThrowIfNull(name);
            Scene scene = GetSceneByName(name);
            try
            {
                Backend.UnloadScene(name);
            }
            catch (SceneManagementException)
            {
                throw;
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                throw new SceneManagementException("Failed to unload scene '" + name + "': " + ex.Message, ex);
            }

            RemoveLoadedScene(name);
            SceneManager.RaiseSceneUnloaded(scene);
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
            _fixedUpdateCallsInStep = 0;
            _lateUpdateCallsInTick = 0;
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
                    _entityScripts.Remove(entity);
                }
            }
        }

        private ulong EntityForScript(ulong scriptHandle)
        {
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
            _fixedUpdateCallsInStep = 0;
            _lateUpdateCallsInTick = 0;
            Time.Advance(deltaSeconds);
            int result = RuntimeInterop.unitas_runtime_tick(_runtimeHandle, deltaSeconds * Time.timeScale);
            if (result != 0)
            {
                string? error = GetErrorMessage();
                if (string.IsNullOrEmpty(error))
                {
                    throw new InvalidOperationException("Failed to advance runtime tick (error code " + result.ToString(CultureInfo.InvariantCulture) + ").");
                }
                throw new InvalidOperationException("Failed to advance runtime tick (error code " + result.ToString(CultureInfo.InvariantCulture) + "): " + error);
            }

            SyncSimulatedTransforms();
            SceneManager.PumpAsyncOperations();
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
                _fixedUpdateCallsInStep = 0;
                _lateUpdateCallsInTick = 0;
                _scripts.Clear();
                _entityScripts.Clear();
                _sceneRigidbodies.Clear();
                StopAllSafely();
                if (ReferenceEquals(Physics.Provider, this))
                {
                    Physics.Provider = null;
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
            string? projectRoot = ResolveProjectRoot(scenePath, out string resolvedDataPath);
            _dataPath = resolvedDataPath;

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
            MonoBehaviour? script = FindScriptByEntity(entity);
            return script?.GetComponent<Collider>();
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

        public bool Linecast(Vector3 origin, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_linecast(
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
            if (button < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(button));
            }
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

        private ulong OnCreateScript(ulong entity, ulong guidHi, ulong guidLo)
        {
            MonoBehaviour? script = ScriptRegistry.Create(guidHi, guidLo);
            if (script == null)
            {
                return 0;
            }
            ulong handle = _nextScriptHandle++;
            _scripts[handle] = script;
            _entityScripts[entity] = handle;
            GameObject? owner = script.gameObject;
            if (owner != null)
            {
                owner.AssignNativeEntityId(entity);
                if (_sceneRigidbodies.TryGetValue(entity, out SceneRigidbodyMetadata metadata))
                {
                    Rigidbody body = owner.GetComponent<Rigidbody>() ?? owner.AddComponent<Rigidbody>();
                    body.mass = metadata.Mass;
                    body.useGravity = metadata.UseGravity;
                    body.isKinematic = metadata.IsKinematic;
                }
                SyncSimulatedTransform(owner);
            }
            return handle;
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
            SyncSimulatedTransforms();
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.Start);
            }
        }

        private void OnFixedUpdate(ulong scriptHandle, double fixedDeltaSeconds)
        {
            SyncSimulatedTransforms();
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.FixedUpdate);
            }
            _fixedUpdateCallsInStep++;
            if (_scripts.Count > 0 && _fixedUpdateCallsInStep >= _scripts.Count)
            {
                _fixedUpdateCallsInStep = 0;
                PumpFixedSafely();
            }
        }

        private void OnUpdate(ulong scriptHandle, double deltaSeconds)
        {
            SyncSimulatedTransforms();
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.Update);
            }
        }

        private void OnLateUpdate(ulong scriptHandle, double deltaSeconds)
        {
            SyncSimulatedTransforms();
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script) && IsEligible(script))
            {
                InvokeWithOrigin(script!, script!.LateUpdate);
            }
            _lateUpdateCallsInTick++;
            if (_scripts.Count > 0 && _lateUpdateCallsInTick >= _scripts.Count)
            {
                _lateUpdateCallsInTick = 0;
                PumpFrameSafely();
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
            MonoBehaviour? scriptA = FindScriptByEntity(entityA);
            MonoBehaviour? scriptB = FindScriptByEntity(entityB);

            if (IsEligible(scriptA))
            {
                DispatchCollision(scriptA!, new Collision(scriptB != null ? scriptB.gameObject : null), phase);
            }

            if (IsEligible(scriptB) && !ReferenceEquals(scriptA, scriptB))
            {
                DispatchCollision(scriptB!, new Collision(scriptA != null ? scriptA.gameObject : null), phase);
            }
        }

        private void OnTrigger(ulong entityA, ulong entityB, uint phase)
        {
            SyncSimulatedTransforms();
            MonoBehaviour? scriptA = FindScriptByEntity(entityA);
            MonoBehaviour? scriptB = FindScriptByEntity(entityB);

            if (IsEligible(scriptA))
            {
                DispatchTrigger(scriptA!, scriptB?.GetComponent<Collider>(), phase);
            }

            if (IsEligible(scriptB) && !ReferenceEquals(scriptA, scriptB))
            {
                DispatchTrigger(scriptB!, scriptA?.GetComponent<Collider>(), phase);
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
                owner.transform.position = simulatedPosition;
            }
        }

        private void ParseLoadedSceneRigidbodies(string scenePath, LoadSceneMode mode)
        {
            if (mode == LoadSceneMode.Single)
            {
                _sceneRigidbodies.Clear();
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
            ulong localEntityIndex = 0;

            int currentClassId = 0;
            string currentFileId = string.Empty;
            string currentGameObjectFileId = string.Empty;
            float currentMass = 1f;
            bool currentUseGravity = true;
            bool currentIsKinematic = false;
            List<ParsedRigidbodyRecord> rigidbodies = new List<ParsedRigidbodyRecord>();

            for (int index = 0; index < lines.Length; index++)
            {
                string line = lines[index].Trim();
                if (line.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    FlushDocument(
                        currentClassId,
                        currentFileId,
                        currentGameObjectFileId,
                        currentMass,
                        currentUseGravity,
                        currentIsKinematic,
                        ref localEntityIndex,
                        gameObjectEntities,
                        rigidbodies);

                    ParseDocumentHeader(line, out currentClassId, out currentFileId);
                    currentGameObjectFileId = string.Empty;
                    currentMass = 1f;
                    currentUseGravity = true;
                    currentIsKinematic = false;
                    continue;
                }

                if (currentClassId == 54)
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
            }

            FlushDocument(
                currentClassId,
                currentFileId,
                currentGameObjectFileId,
                currentMass,
                currentUseGravity,
                currentIsKinematic,
                ref localEntityIndex,
                gameObjectEntities,
                rigidbodies);

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

            _nextSceneEntityBase += localEntityIndex;
        }

        private static void FlushDocument(
            int classId,
            string fileId,
            string gameObjectFileId,
            float mass,
            bool useGravity,
            bool isKinematic,
            ref ulong localEntityIndex,
            Dictionary<string, ulong> gameObjectEntities,
            List<ParsedRigidbodyRecord> rigidbodies)
        {
            if (classId == 1 && !string.IsNullOrEmpty(fileId))
            {
                gameObjectEntities[fileId] = localEntityIndex;
                localEntityIndex++;
            }
            else if (classId == 54 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                rigidbodies.Add(new ParsedRigidbodyRecord(gameObjectFileId, mass, useGravity, isKinematic));
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
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }
    }
}
