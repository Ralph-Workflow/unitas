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

            DropReplacedScripts();
            DestroyReplacedSceneObjects();
            ParseLoadedSceneRigidbodies(path, LoadSceneMode.Single);
            RuntimeInterop.unitas_runtime_physics2d_clear(_runtimeHandle);
            MaterializeScenePhysics2D();
            _defaultSceneProvider.SetSceneRoot(path);
            UpdateProjectMetadata(path);
            RegisterLoadedScene(path, LoadSceneMode.Single);
            Scene loaded = FindLoadedScene(path);
            DeregisterReplacedSceneObjects(loaded, LoadSceneMode.Single);
            SceneManager.RaiseSceneLoaded(loaded, LoadSceneMode.Single);
            MaterializeLoadedSceneObjects(loaded);
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

            if (mode == LoadSceneMode.Single)
            {
                DropReplacedScripts();
                DestroyReplacedSceneObjects();
            }

            ParseLoadedSceneRigidbodies(scenePath, mode);
            if (mode == LoadSceneMode.Single)
            {
                RuntimeInterop.unitas_runtime_physics2d_clear(_runtimeHandle);
            }
            MaterializeScenePhysics2D();
            _defaultSceneProvider.SetSceneRoot(scenePath);
            UpdateProjectMetadata(scenePath);
            RegisterLoadedScene(scenePath, mode);
            Scene loaded = FindLoadedScene(scenePath);
            DeregisterReplacedSceneObjects(loaded, mode);
            SceneManager.RaiseSceneLoaded(loaded, mode);
            if (mode == LoadSceneMode.Single)
            {
                MaterializeLoadedSceneObjects(loaded);
            }
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
            created.scene = default;
            created.AssignNativeEntityId(entity);
            ApplySceneTransform(entity, created);
            _entityGameObjects[entity] = created;
            _materializedSceneEntities.Add(entity);
            return created;
        }

        /// <summary>
        /// Creates the objects of the loaded scene, so a lookup finds them right after
        /// the load, the way Unity creates the objects of a scene when it loads it.
        /// The objects of the replaced scene are already destroyed, so the lookup
        /// cannot hand back a stale object.
        /// </summary>
        private void MaterializeLoadedSceneObjects(Scene loadedScene)
        {
            List<ulong> entities = new List<ulong>(_sceneObjectNames.Keys);
            for (int index = 0; index < entities.Count; index++)
            {
                ulong entity = entities[index];
                GameObject? owner = ResolveGameObjectByEntity(entity);
                if (owner == null)
                {
                    continue;
                }

                owner.scene = loadedScene;
            }

            for (int index = 0; index < entities.Count; index++)
            {
                ulong entity = entities[index];
                if (_entityGameObjects.TryGetValue(entity, out GameObject? owner) && owner != null && !owner.IsDestroyed)
                {
                    WireSceneParent(entity, owner);
                }
            }
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
                    ApplySceneAudioSourceComponent(entity, existing);
                }
                return existing;
            }

            if (!IsSceneEntity(entity))
            {
                return null;
            }

            GameObject created = EnsureSceneGameObject(entity);
            ApplyScenePhysicsComponents(entity, created);
            ApplySceneAudioSourceComponent(entity, created);
            return created;
        }

        private Collider? ResolveColliderByEntity(ulong entity)
        {
            return ResolveGameObjectByEntity(entity)?.GetComponent<Collider>();
        }

        private Collider2D? ResolveCollider2DByEntity(ulong entity)
        {
            return ResolveGameObjectByEntity(entity)?.GetComponent<Collider2D>();
        }

        private bool IsSceneEntity(ulong entity)
        {
            return _sceneObjectNames.ContainsKey(entity)
                || _sceneRigidbodies.ContainsKey(entity)
                || _sceneSphereColliders.ContainsKey(entity)
                || _sceneBoxColliders.ContainsKey(entity)
                || _sceneCapsuleColliders.ContainsKey(entity)
                || _sceneMeshColliders.ContainsKey(entity)
                || _sceneRigidbodies2D.ContainsKey(entity)
                || _sceneBoxColliders2D.ContainsKey(entity)
                || _sceneCircleColliders2D.ContainsKey(entity)
                || _scenePlatformEffectors2D.ContainsKey(entity)
                || _sceneSurfaceEffectors2D.ContainsKey(entity)
                || _sceneAudioSources.ContainsKey(entity);
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
            ShiftEntityKeys(_sceneAudioSources, delta);
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
                    ApplySceneAudioSourceComponent(shifted, owner);
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
    }
}
