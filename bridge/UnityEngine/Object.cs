using System;
using System.Collections.Generic;
using System.Threading;
using UnityEngine.SceneManagement;

namespace UnityEngine
{
    public class MissingReferenceException : Exception
    {
        public MissingReferenceException() : base("The object of type 'Object' has been destroyed but you are still trying to access it.")
        {
        }

        public MissingReferenceException(string message) : base(message)
        {
        }

        public MissingReferenceException(string message, Exception innerException) : base(message, innerException)
        {
        }
    }

    public class UnassignedReferenceException : Exception
    {
        public UnassignedReferenceException() : base("The variable has not been assigned.")
        {
        }

        public UnassignedReferenceException(string message) : base(message)
        {
        }

        public UnassignedReferenceException(string message, Exception innerException) : base(message, innerException)
        {
        }
    }

    public class MissingComponentException : Exception
    {
        public MissingComponentException() : base("There is no component attached to the GameObject.")
        {
        }

        public MissingComponentException(string message) : base(message)
        {
        }

        public MissingComponentException(string message, Exception innerException) : base(message, innerException)
        {
        }
    }

    [AttributeUsage(AttributeTargets.Class, AllowMultiple = true, Inherited = true)]
    public sealed class RequireComponent : Attribute
    {
        public Type? m_Type0;

        public Type? m_Type1;

        public Type? m_Type2;

        public RequireComponent(Type requiredComponent)
        {
            m_Type0 = requiredComponent;
        }

        public RequireComponent(Type requiredComponent, Type requiredComponent2)
        {
            m_Type0 = requiredComponent;
            m_Type1 = requiredComponent2;
        }

        public RequireComponent(Type requiredComponent, Type requiredComponent2, Type requiredComponent3)
        {
            m_Type0 = requiredComponent;
            m_Type1 = requiredComponent2;
            m_Type2 = requiredComponent3;
        }
    }

    [AttributeUsage(AttributeTargets.Class, Inherited = true)]
    public sealed class DisallowMultipleComponent : Attribute
    {
        public DisallowMultipleComponent()
        {
        }
    }

    [AttributeUsage(AttributeTargets.Class, Inherited = true)]
    public sealed class DefaultExecutionOrder : Attribute
    {
        public DefaultExecutionOrder(int order)
        {
            this.order = order;
        }

        public int order { get; }
    }

    public partial class Object
    {
        private static readonly object s_registryLock = new object();
        private static readonly List<Object> s_liveObjects = new List<Object>();
        private static readonly List<PendingDestroy> s_pendingDestroys = new List<PendingDestroy>();
        private static int s_nextInstanceId = 1;
        private int _instanceId = Interlocked.Increment(ref s_nextInstanceId);
        private string _name = string.Empty;
        private bool _destroyed;
        private bool _destroying;
        private bool _persistent;
        private bool _unassignedReference;
        private string _unassignedFieldName = string.Empty;
        private string _unassignedOwnerName = string.Empty;
        private Scene _scene;

        public static bool ForcePlayerReferenceExceptions { get; set; }

        public Object()
        {
            RegisterLiveObject(this);
        }

        public virtual string name
        {
            get
            {
                ThrowIfDestroyed();
                return _name;
            }
            set
            {
                ThrowIfDestroyed();
                _name = value;
            }
        }

        public Scene scene
        {
            get => _scene;
            internal set => _scene = value;
        }

        public int GetInstanceID() => _instanceId;

        public override string ToString() => (_destroyed || _unassignedReference) ? "null" : _name;

        public override bool Equals(object? obj) => this == (obj as Object);

        public override int GetHashCode() => (_destroyed || _unassignedReference) ? 0 : _instanceId;

        internal bool IsDestroyed => _destroyed;

        internal bool IsUnassignedReference => _unassignedReference;

        internal bool IsPersistent => _persistent;

        internal void MarkAsUnassignedReference(string fieldName, string ownerName)
        {
            _unassignedReference = true;
            _unassignedFieldName = fieldName ?? string.Empty;
            _unassignedOwnerName = ownerName ?? string.Empty;
            DeregisterLiveObject(this);
        }

        internal void AssignNewInstanceId()
        {
            _instanceId = Interlocked.Increment(ref s_nextInstanceId);
            RegisterLiveObject(this);
        }

        internal static void RegisterLiveObject(Object instance)
        {
            if (instance is null || instance._destroyed)
            {
                return;
            }

            lock (s_registryLock)
            {
                for (int i = 0; i < s_liveObjects.Count; i++)
                {
                    if (ReferenceEquals(s_liveObjects[i], instance))
                    {
                        return;
                    }
                }

                s_liveObjects.Add(instance);
            }
        }

        internal static void DeregisterLiveObject(Object instance)
        {
            if (instance is null)
            {
                return;
            }

            lock (s_registryLock)
            {
                for (int i = s_liveObjects.Count - 1; i >= 0; i--)
                {
                    if (ReferenceEquals(s_liveObjects[i], instance))
                    {
                        s_liveObjects.RemoveAt(i);
                    }
                }
            }
        }

        internal static List<Object> SnapshotLiveObjects()
        {
            lock (s_registryLock)
            {
                return new List<Object>(s_liveObjects);
            }
        }

        internal Component ShallowClone() => (Component)MemberwiseClone();

        public static T[] FindObjectsOfType<T>() where T : Object => FindObjectsOfType<T>(false);

        public static T[] FindObjectsOfType<T>(bool includeInactive) where T : Object
        {
            List<Object> snapshot = SnapshotLiveObjects();
            List<T> matches = new List<T>();
            for (int i = 0; i < snapshot.Count; i++)
            {
                Object candidate = snapshot[i];
                if (!IsDiscoverable(candidate, includeInactive))
                {
                    continue;
                }

                if (candidate is T typed)
                {
                    matches.Add(typed);
                }
            }

            return matches.ToArray();
        }

        public static T? FindObjectOfType<T>() where T : Object => FindObjectOfType<T>(false);

        public static T? FindObjectOfType<T>(bool includeInactive) where T : Object
        {
            List<Object> snapshot = SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                Object candidate = snapshot[i];
                if (!IsDiscoverable(candidate, includeInactive))
                {
                    continue;
                }

                if (candidate is T typed)
                {
                    return typed;
                }
            }

            return null;
        }

        public static Object[] FindObjectsOfType(Type type) => FindObjectsOfType(type, false);

        public static Object[] FindObjectsOfType(Type type, bool includeInactive)
        {
            ValidateSearchType(type);
            List<Object> snapshot = SnapshotLiveObjects();
            List<Object> matches = new List<Object>();
            for (int i = 0; i < snapshot.Count; i++)
            {
                Object candidate = snapshot[i];
                if (!IsDiscoverable(candidate, includeInactive))
                {
                    continue;
                }

                if (type.IsInstanceOfType(candidate))
                {
                    matches.Add(candidate);
                }
            }

            return matches.ToArray();
        }

        public static Object? FindObjectOfType(Type type) => FindObjectOfType(type, false);

        public static Object? FindObjectOfType(Type type, bool includeInactive)
        {
            ValidateSearchType(type);
            List<Object> snapshot = SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                Object candidate = snapshot[i];
                if (!IsDiscoverable(candidate, includeInactive))
                {
                    continue;
                }

                if (type.IsInstanceOfType(candidate))
                {
                    return candidate;
                }
            }

            return null;
        }

        private static void ValidateSearchType(Type type)
        {
            if (type == null)
            {
                throw new ArgumentNullException(nameof(type));
            }

            if (!typeof(Object).IsAssignableFrom(type))
            {
                throw new ArgumentException("Type must derive from UnityEngine.Object.", nameof(type));
            }
        }

        private static bool IsDiscoverable(Object candidate, bool includeInactive)
        {
            if (candidate is null || candidate._destroyed || candidate._unassignedReference)
            {
                return false;
            }

            if (includeInactive)
            {
                return true;
            }

            if (candidate is GameObject gameObject)
            {
                return gameObject.activeInHierarchy;
            }

            if (candidate is Component component)
            {
                GameObject? owner = component.OwnerOrNull;
                return owner is not null && !owner._destroyed && !owner._unassignedReference && owner.activeInHierarchy;
            }

            return true;
        }

        private static bool IsNullOrDestroyedOrUnassigned(Object? obj) =>
            obj is null || obj._destroyed || obj._unassignedReference;

        public static bool operator ==(Object? left, Object? right)
        {
            bool leftNull = IsNullOrDestroyedOrUnassigned(left);
            bool rightNull = IsNullOrDestroyedOrUnassigned(right);
            if (leftNull && rightNull)
            {
                return true;
            }

            if (leftNull || rightNull)
            {
                return false;
            }

            return ReferenceEquals(left, right);
        }

        public static bool operator !=(Object? left, Object? right) => !(left == right);

        public static implicit operator bool(Object? value) => !IsNullOrDestroyedOrUnassigned(value);

        public static void DontDestroyOnLoad(Object target)
        {
            if (target == null)
            {
                throw new ArgumentNullException(nameof(target));
            }

            target._persistent = true;
            if (target is Component component)
            {
                GameObject? owner = component.OwnerOrNull;
                if (owner != null)
                {
                    owner._persistent = true;
                }
            }

            ulong entity = NativeEntityOf(target);
            if (entity != 0)
            {
                SceneManager.Host?.DontDestroyOnLoad(entity);
            }
        }

        private static ulong NativeEntityOf(Object target)
        {
            if (target is GameObject gameObject)
            {
                return gameObject.NativeEntityId;
            }

            if (target is Component component)
            {
                GameObject? owner = component.OwnerOrNull;
                return owner == null ? 0 : owner.NativeEntityId;
            }

            return 0;
        }

        internal static ulong NativeEntityIdOf(Object target) => NativeEntityOf(target);

        public static void Destroy(Object? obj, float t)
        {
            if (obj is null || obj._destroyed || obj._unassignedReference)
            {
                return;
            }

            if (t > 0f && float.IsFinite(t))
            {
                QueueDestroy(obj, Time.time + t);
                return;
            }

            Destroy(obj);
        }

        public static void DestroyImmediate(Object? obj)
        {
            Destroy(obj);
        }

        public static void DestroyImmediate(Object? obj, bool allowDestroyingAssets)
        {
            _ = allowDestroyingAssets;
            Destroy(obj);
        }

        public static void Destroy(Object? obj)
        {
            if (obj is null || obj._destroyed || obj._unassignedReference)
            {
                return;
            }

            DestroyNow(obj);
            QueueDestroy(obj, Time.time);
        }

        private static void QueueDestroy(Object target, float deadline)
        {
            lock (s_registryLock)
            {
                s_pendingDestroys.Add(new PendingDestroy(target, deadline));
            }
        }

        internal static void FlushPendingDestroys(List<Object> destroyed)
        {
            List<PendingDestroy>? due = null;
            lock (s_registryLock)
            {
                float now = Time.time;
                for (int index = 0; index < s_pendingDestroys.Count;)
                {
                    PendingDestroy request = s_pendingDestroys[index];
                    if (request.Deadline > now)
                    {
                        index++;
                        continue;
                    }

                    s_pendingDestroys.RemoveAt(index);
                    if (due is null)
                    {
                        due = new List<PendingDestroy>();
                    }

                    due.Add(request);
                }
            }

            if (due is null)
            {
                return;
            }

            for (int index = 0; index < due.Count; index++)
            {
                Object target = due[index].Target;
                if (!target._destroyed && !target._unassignedReference)
                {
                    DestroyNow(target);
                }

                destroyed.Add(target);
            }
        }

        private sealed class PendingDestroy
        {
            internal PendingDestroy(Object target, float deadline)
            {
                Target = target;
                Deadline = deadline;
            }

            internal Object Target { get; }

            internal float Deadline { get; }
        }

        private static void DestroyNow(Object? obj)
        {
            if (obj is null || obj._destroyed || obj._unassignedReference)
            {
                return;
            }

            if (obj is Component candidateComponent)
            {
                GameObject? owner = candidateComponent.OwnerOrNull;
                if (owner is not null
                    && !owner._destroyed
                    && !owner._destroying
                    && owner.TryFindDependentRequiring(candidateComponent, out Component? dependent))
                {
                    string targetTypeName = candidateComponent.ComponentTypeName;
                    string dependentTypeName = dependent != null ? dependent.ComponentTypeName : nameof(Component);
                    Debug.LogError(
                        "Can't remove "
                        + targetTypeName
                        + " because "
                        + dependentTypeName
                        + " depends on it",
                        owner);
                    return;
                }
            }

            obj._destroying = true;
            try
            {
                if (obj is Component component)
                {
                    bool notifyDisable = IsEffectivelyEnabled(component);
                    obj._destroyed = true;
                    DeregisterLiveObject(component);
                    DestroyComponent(component, notifyDisable);
                }
                else if (obj is GameObject gameObject)
                {
                    int count = gameObject.Components.Count;
                    bool[] notify = new bool[count];
                    for (int i = 0; i < count; i++)
                    {
                        Component c = gameObject.Components[i];
                        if (c is null || c._destroyed)
                        {
                            continue;
                        }

                        notify[i] = IsEffectivelyEnabled(c);
                    }

                    obj._destroyed = true;
                    DeregisterLiveObject(gameObject);
                    for (int i = 0; i < count; i++)
                    {
                        Component c = gameObject.Components[i];
                        if (c is null || c._destroyed)
                        {
                            continue;
                        }

                        DeregisterLiveObject(c);
                        DestroyComponent(c, notify[i]);
                    }
                }
                else
                {
                    obj._destroyed = true;
                    DeregisterLiveObject(obj);
                }
            }
            finally
            {
                obj._destroying = false;
            }
        }

        /// <summary>
        /// Destroys a game object and drops its components from the live registry without
        /// replaying their lifecycle callbacks. The native scene teardown already delivered
        /// the replaced scene's script callbacks, so the components stay readable for the
        /// code that still holds them.
        /// </summary>
        internal static void DestroySceneObject(GameObject target)
        {
            if (target is null || target._destroyed || target._unassignedReference)
            {
                return;
            }

            target._destroyed = true;
            DeregisterLiveObject(target);
            IReadOnlyList<Component> components = target.Components;
            for (int index = 0; index < components.Count; index++)
            {
                DeregisterLiveObject(components[index]);
            }

            QueueDestroy(target, Time.time);
        }

        private static bool IsEffectivelyEnabled(Component component)
        {
            if (component is not null && !component._destroyed && component is MonoBehaviour behaviour)
            {
                return behaviour.isActiveAndEnabled;
            }

            return false;
        }

        private static void DestroyComponent(Component component, bool notifyDisable)
        {
            component._destroyed = true;
            DeregisterLiveObject(component);
            if (component is AudioSource audioSource)
            {
                audioSource.OnComponentDestroyed();
            }
            if (!(component is MonoBehaviour behaviour))
            {
                return;
            }

            Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = component;
            try
            {
                CoroutineScheduler.OwnerDestroyed(behaviour);
                if (notifyDisable)
                {
                    behaviour.OnDisable();
                }

                behaviour.OnDestroy();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, component);
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }

        internal void ThrowIfUnassignedReference()
        {
            if (_unassignedReference)
            {
                bool isPlayer = ForcePlayerReferenceExceptions
                    || (Application.Host != null && !Application.isEditor);
                if (isPlayer)
                {
                    throw new NullReferenceException("Object reference not set to an instance of an object.");
                }

                string fieldLabel = string.IsNullOrEmpty(_unassignedFieldName) ? "reference" : _unassignedFieldName;
                string ownerLabel = string.IsNullOrEmpty(_unassignedOwnerName) ? "MonoBehaviour" : _unassignedOwnerName;
                throw new UnassignedReferenceException(
                    "The variable "
                    + fieldLabel
                    + " of "
                    + ownerLabel
                    + " has not been assigned.");
            }
        }

        internal void ThrowIfDestroyed()
        {
            ThrowIfUnassignedReference();

            if (_destroyed && !_destroying)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }
        }
    }
}
