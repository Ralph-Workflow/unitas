using System;

namespace UnityEngine
{
    /// <summary>
    /// Creates and destroys the ScriptableObject instances the native runtime world holds.
    /// </summary>
    public interface IScriptableObjectProvider
    {
        ulong CreateInstance(string name);

        void DestroyInstance(ulong instance);

        bool IsAlive(ulong instance);
    }

    /// <summary>
    /// A data object the runtime world owns instead of a scene, with the Unity
    /// lifecycle callbacks and the fake-null identity every Unity object has.
    /// </summary>
    public class ScriptableObject : Object
    {
        public static IScriptableObjectProvider? Provider { get; set; }

        private ulong _nativeInstance;
        private bool _destroyDispatched;

        /// <summary>
        /// The native world instance that holds this object, or zero when the
        /// runtime holds none.
        /// </summary>
        public ulong NativeInstanceId => _nativeInstance;

        public static ScriptableObject CreateInstance(Type type)
        {
            if (type == null)
            {
                throw new ArgumentNullException(nameof(type));
            }

            if (!typeof(ScriptableObject).IsAssignableFrom(type))
            {
                throw new ArgumentException(
                    "Type must derive from UnityEngine.ScriptableObject.",
                    nameof(type));
            }

            ScriptableObject instance = (ScriptableObject)Activator.CreateInstance(type)!;
            instance.name = type.Name;
            IScriptableObjectProvider? provider = Provider;
            if (provider != null)
            {
                ulong native = provider.CreateInstance(type.Name);
                if (native != 0UL)
                {
                    instance._nativeInstance = native;
                }
            }

            instance.Awake();
            instance.OnEnable();
            instance.OnValidate();
            return instance;
        }

        public static T CreateInstance<T>() where T : ScriptableObject
        {
            return (T)CreateInstance(typeof(T));
        }

        /// <summary>
        /// Destroys a ScriptableObject, dispatching OnDisable and then OnDestroy
        /// before the native world drops the instance. The base
        /// <see cref="Object.Destroy"/> routes a ScriptableObject here as well,
        /// so every destroy spelling reaches the same teardown.
        /// </summary>
        public new static void Destroy(Object? obj)
        {
            if (obj is ScriptableObject instance)
            {
                instance.DestroyInstance();
                return;
            }

            Object.Destroy(obj);
        }

        public virtual void Awake()
        {
        }

        public virtual void OnEnable()
        {
        }

        public virtual void OnValidate()
        {
        }

        public virtual void OnDisable()
        {
        }

        public virtual void OnDestroy()
        {
        }

        internal void DestroyInstance()
        {
            if (IsDestroyed || _destroyDispatched)
            {
                return;
            }

            _destroyDispatched = true;
            OnDisable();
            OnDestroy();
            ReleaseNativeInstance();
            Object.Destroy(this);
        }

        private void ReleaseNativeInstance()
        {
            if (_nativeInstance == 0UL)
            {
                return;
            }

            ulong instance = _nativeInstance;
            _nativeInstance = 0UL;
            Provider?.DestroyInstance(instance);
        }
    }
}
