using System;
using System.Collections.Generic;
using System.Threading;
using UnityEngine.SceneManagement;

namespace UnityEngine
{
    public class MissingReferenceException : Exception
    {
        public MissingReferenceException(string message) : base(message)
        {
        }
    }

    public partial class Object
    {
        private static readonly object s_registryLock = new object();
        private static readonly List<Object> s_liveObjects = new List<Object>();
        private static int s_nextInstanceId = 1;
        private int _instanceId = Interlocked.Increment(ref s_nextInstanceId);
        private string _name = string.Empty;
        private bool _destroyed;
        private bool _destroying;
        private bool _persistent;
        private Scene _scene;

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

        /// <summary>
        /// The scene the object belongs to. A new object takes the scene that was active when it was
        /// created; a component takes the scene of the game object it is attached to.
        /// </summary>
        public Scene scene
        {
            get => _scene;
            internal set => _scene = value;
        }

        public int GetInstanceID() => _instanceId;

        public override string ToString() => _destroyed ? "null" : _name;

        public override bool Equals(object? other) => this == (other as Object);

        public override int GetHashCode() => _destroyed ? 0 : _instanceId;

        internal bool IsDestroyed => _destroyed;

        /// <summary>True once the object was marked to survive scene changes.</summary>
        internal bool IsPersistent => _persistent;

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
            if (candidate is null || candidate._destroyed)
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
                GameObject? owner = component.gameObject;
                return owner is not null && !owner._destroyed && owner.activeInHierarchy;
            }

            return true;
        }

        public static bool operator ==(Object? left, Object? right)
        {
            if (left is null && right is null)
            {
                return true;
            }

            if (left is null)
            {
                return right!._destroyed;
            }

            if (right is null)
            {
                return left!._destroyed;
            }

            if (left._destroyed && right._destroyed)
            {
                return true;
            }

            if (left._destroyed || right._destroyed)
            {
                return false;
            }

            return ReferenceEquals(left, right);
        }

        public static bool operator !=(Object? left, Object? right) => !(left == right);

        public static implicit operator bool(Object? value) => value is not null && !value._destroyed;

        /// <summary>
        /// Marks the object so it survives scene changes. Objects backed by a native entity forward that
        /// entity to the native runtime; managed-only objects are flagged locally and are not forwarded yet.
        /// Marking a component also marks the game object that owns it, because the bridge keeps a component
        /// alive by the flag of its game object.
        /// </summary>
        public static void DontDestroyOnLoad(Object target)
        {
            if (target == null)
            {
                throw new ArgumentNullException(nameof(target));
            }

            target._persistent = true;
            if (target is Component component)
            {
                GameObject? owner = component.gameObject;
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
                GameObject? owner = component.gameObject;
                return owner == null ? 0 : owner.NativeEntityId;
            }

            return 0;
        }

        public static void Destroy(Object obj)
        {
            if (obj is null || obj._destroyed)
            {
                return;
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
                        notify[i] = IsEffectivelyEnabled(gameObject.Components[i]);
                    }

                    obj._destroyed = true;
                    DeregisterLiveObject(gameObject);
                    for (int i = 0; i < count; i++)
                    {
                        DeregisterLiveObject(gameObject.Components[i]);
                        DestroyComponent(gameObject.Components[i], notify[i]);
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

        private static bool IsEffectivelyEnabled(Component component)
        {
            if (component is MonoBehaviour behaviour)
            {
                return behaviour.enabled && behaviour.isActiveAndEnabled;
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

        private void ThrowIfDestroyed()
        {
            if (_destroyed && !_destroying)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }
        }
    }
}
