using System;
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
        private static int s_nextInstanceId = 1;
        private int _instanceId = Interlocked.Increment(ref s_nextInstanceId);
        private string _name = string.Empty;
        private bool _destroyed;
        private bool _destroying;
        private bool _persistent;
        private Scene _scene;

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
        }

        internal Component ShallowClone() => (Component)MemberwiseClone();

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
        /// </summary>
        public static void DontDestroyOnLoad(Object target)
        {
            if (target == null)
            {
                throw new ArgumentNullException(nameof(target));
            }

            target._persistent = true;
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
                    for (int i = 0; i < count; i++)
                    {
                        DestroyComponent(gameObject.Components[i], notify[i]);
                    }
                }
                else
                {
                    obj._destroyed = true;
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
