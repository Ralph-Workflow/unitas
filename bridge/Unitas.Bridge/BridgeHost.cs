using System;
using System.Collections.Generic;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed class BridgeHost : IDisposable, IPhysicsProvider
    {
        private const uint CollisionPhaseEnter = 0;
        private const uint CollisionPhaseStay = 1;
        private const uint CollisionPhaseExit = 2;

        private readonly ulong _runtimeHandle;
        private readonly Dictionary<ulong, MonoBehaviour> _scripts = new Dictionary<ulong, MonoBehaviour>();
        private readonly Dictionary<ulong, ulong> _entityScripts = new Dictionary<ulong, ulong>();
        private ulong _nextScriptHandle = 1;
        private bool _isDisposed;

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
        }

        public ulong Handle => _runtimeHandle;

        public int ScriptCount => _scripts.Count;

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
            if (path == null)
            {
                throw new ArgumentNullException(nameof(path));
            }
            _scripts.Clear();
            _entityScripts.Clear();
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
        }

        public void Tick(double deltaSeconds)
        {
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
        }

        public void Shutdown()
        {
            if (!_isDisposed)
            {
                _isDisposed = true;
                int result = RuntimeInterop.unitas_runtime_destroy(_runtimeHandle);
                _scripts.Clear();
                _entityScripts.Clear();
                if (ReferenceEquals(Physics.Provider, this))
                {
                    Physics.Provider = null;
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
            return handle;
        }

        private void OnAwake(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.Awake);
            }
        }

        private void OnOnEnable(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.OnEnable);
            }
        }

        private void OnStart(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.Start);
            }
        }

        private void OnFixedUpdate(ulong scriptHandle, double fixedDeltaSeconds)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.FixedUpdate);
            }
        }

        private void OnUpdate(ulong scriptHandle, double deltaSeconds)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.Update);
            }
        }

        private void OnLateUpdate(ulong scriptHandle, double deltaSeconds)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.LateUpdate);
            }
        }

        private void OnOnDisable(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.OnDisable);
            }
        }

        private void OnDestroy(ulong scriptHandle)
        {
            if (_scripts.TryGetValue(scriptHandle, out MonoBehaviour? script))
            {
                InvokeWithOrigin(script, script.OnDestroy);
                _scripts.Remove(scriptHandle);
            }
        }

        private void OnCollision(ulong entityA, ulong entityB, uint phase)
        {
            MonoBehaviour? scriptA = FindScriptByEntity(entityA);
            MonoBehaviour? scriptB = FindScriptByEntity(entityB);

            if (scriptA != null)
            {
                DispatchCollision(scriptA, new Collision(scriptB != null ? scriptB.gameObject : null), phase);
            }

            if (scriptB != null && !ReferenceEquals(scriptA, scriptB))
            {
                DispatchCollision(scriptB, new Collision(scriptA != null ? scriptA.gameObject : null), phase);
            }
        }

        private void OnTrigger(ulong entityA, ulong entityB, uint phase)
        {
            MonoBehaviour? scriptA = FindScriptByEntity(entityA);
            MonoBehaviour? scriptB = FindScriptByEntity(entityB);

            if (scriptA != null)
            {
                DispatchTrigger(scriptA, scriptB?.GetComponent<Collider>(), phase);
            }

            if (scriptB != null && !ReferenceEquals(scriptA, scriptB))
            {
                DispatchTrigger(scriptB, scriptA?.GetComponent<Collider>(), phase);
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

        private static void InvokeWithOrigin(MonoBehaviour script, Action callback)
        {
            UnityEngine.Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = script;
            try
            {
                callback();
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }
    }
}
