using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public partial class Component : Object
    {
        private static readonly Dictionary<Type, Type[]> s_registeredRequiredComponents =
            new Dictionary<Type, Type[]>();

        private static readonly HashSet<Type> s_registeredDisallowMultiple =
            new HashSet<Type>();

        private GameObject? _gameObject;

        internal GameObject? OwnerOrNull => _gameObject;

        public GameObject gameObject
        {
            get
            {
                ThrowIfUnassignedReference();
                return _gameObject!;
            }
            internal set => _gameObject = value;
        }

        public Transform transform
        {
            get
            {
                ThrowIfUnassignedReference();
                return _gameObject!.transform;
            }
        }

        public virtual IReadOnlyList<Type> RequiredComponents => Array.Empty<Type>();

        public virtual bool DisallowMultiple => false;

        internal Type? AttachedRuntimeType { get; set; }

        internal string ComponentTypeName =>
            AttachedRuntimeType != null ? AttachedRuntimeType.Name : nameof(Component);

        public static void RegisterRequireComponent<TComponent>(params Type[] requiredTypes)
            where TComponent : Component
        {
            RegisterRequireComponent(typeof(TComponent), requiredTypes);
        }

        public static void RegisterRequireComponent(Type componentType, params Type[] requiredTypes)
        {
            if (componentType == null)
            {
                throw new ArgumentNullException(nameof(componentType));
            }

            if (requiredTypes == null || requiredTypes.Length == 0)
            {
                s_registeredRequiredComponents.Remove(componentType);
                return;
            }

            List<Type> valid = new List<Type>(requiredTypes.Length);
            for (int i = 0; i < requiredTypes.Length; i++)
            {
                if (requiredTypes[i] != null)
                {
                    valid.Add(requiredTypes[i]);
                }
            }

            s_registeredRequiredComponents[componentType] = valid.ToArray();
        }

        public static void RegisterDisallowMultipleComponent<TComponent>(bool disallow = true)
            where TComponent : Component
        {
            RegisterDisallowMultipleComponent(typeof(TComponent), disallow);
        }

        public static void RegisterDisallowMultipleComponent(Type componentType, bool disallow = true)
        {
            if (componentType == null)
            {
                throw new ArgumentNullException(nameof(componentType));
            }

            if (disallow)
            {
                s_registeredDisallowMultiple.Add(componentType);
            }
            else
            {
                s_registeredDisallowMultiple.Remove(componentType);
            }
        }

        public static void ClearAttributeRegistrations()
        {
            s_registeredRequiredComponents.Clear();
            s_registeredDisallowMultiple.Clear();
        }

        public static T CreateUnassignedReference<T>(string fieldName, string ownerName = "MonoBehaviour")
            where T : Component, new()
        {
            T placeholder = new T();
            placeholder.AttachedRuntimeType = typeof(T);
            placeholder.MarkAsUnassignedReference(fieldName, ownerName);
            return placeholder;
        }

        internal IReadOnlyList<Type> ResolveRequiredComponents(Type componentType)
        {
            IReadOnlyList<Type> fromVirtual = RequiredComponents;
            bool hasRegistered = s_registeredRequiredComponents.TryGetValue(componentType, out Type[]? registered)
                && registered != null
                && registered.Length > 0;
            if (!hasRegistered)
            {
                return fromVirtual ?? Array.Empty<Type>();
            }

            if (fromVirtual == null || fromVirtual.Count == 0)
            {
                return registered!;
            }

            List<Type> combined = new List<Type>(fromVirtual.Count + registered!.Length);
            for (int i = 0; i < fromVirtual.Count; i++)
            {
                if (fromVirtual[i] != null && !combined.Contains(fromVirtual[i]))
                {
                    combined.Add(fromVirtual[i]);
                }
            }

            for (int i = 0; i < registered.Length; i++)
            {
                if (registered[i] != null && !combined.Contains(registered[i]))
                {
                    combined.Add(registered[i]);
                }
            }

            return combined;
        }

        internal bool ResolveDisallowMultiple(Type componentType) =>
            DisallowMultiple || s_registeredDisallowMultiple.Contains(componentType);

        public override string name
        {
            get
            {
                ThrowIfDestroyed();
                return _gameObject != null ? _gameObject.name : base.name;
            }
            set
            {
                ThrowIfDestroyed();
                if (_gameObject != null)
                {
                    _gameObject.name = value;
                }
                else
                {
                    base.name = value;
                }
            }
        }

        public string tag
        {
            get
            {
                ThrowIfDestroyed();
                return _gameObject != null ? _gameObject.tag : "Untagged";
            }
            set
            {
                ThrowIfDestroyed();
                if (_gameObject != null)
                {
                    _gameObject.tag = value;
                }
            }
        }

        public bool CompareTag(string tag)
        {
            ThrowIfDestroyed();
            if (_gameObject != null)
            {
                return _gameObject.CompareTag(tag);
            }

            GameObject.ValidateTagDefined(tag);
            return string.Equals("Untagged", tag, StringComparison.Ordinal);
        }

        public T? GetComponent<T>() where T : class => gameObject != null ? gameObject.GetComponent<T>() : null;

        public Component? GetComponent(Type type)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponent(type);
            }

            GameObject.ValidateComponentType(type);
            return null;
        }

        public bool TryGetComponent<T>(out T component) where T : class
        {
            if (gameObject != null)
            {
                return gameObject.TryGetComponent<T>(out component);
            }

            component = null!;
            return false;
        }

        public bool TryGetComponent(Type type, out Component? component)
        {
            if (gameObject != null)
            {
                return gameObject.TryGetComponent(type, out component);
            }

            GameObject.ValidateComponentType(type);
            component = null;
            return false;
        }

        public T[] GetComponents<T>() where T : class => gameObject != null ? gameObject.GetComponents<T>() : Array.Empty<T>();

        public void GetComponents<T>(List<T> results) where T : class
        {
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            if (gameObject != null)
            {
                gameObject.GetComponents<T>(results);
            }
            else
            {
                results.Clear();
            }
        }

        public Component[] GetComponents(Type type)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponents(type);
            }

            GameObject.ValidateComponentType(type);
            return Array.Empty<Component>();
        }

        public void GetComponents(Type type, List<Component> results)
        {
            if (gameObject != null)
            {
                gameObject.GetComponents(type, results);
                return;
            }

            GameObject.ValidateComponentType(type);
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }
            results.Clear();
        }

        public T? GetComponentInChildren<T>(bool includeInactive = false) where T : class =>
            gameObject != null ? gameObject.GetComponentInChildren<T>(includeInactive) : null;

        public Component? GetComponentInChildren(Type type) => GetComponentInChildren(type, false);

        public Component? GetComponentInChildren(Type type, bool includeInactive)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponentInChildren(type, includeInactive);
            }

            GameObject.ValidateComponentType(type);
            return null;
        }

        public T[] GetComponentsInChildren<T>(bool includeInactive = false) where T : class =>
            gameObject != null ? gameObject.GetComponentsInChildren<T>(includeInactive) : Array.Empty<T>();

        public void GetComponentsInChildren<T>(bool includeInactive, List<T> results) where T : class
        {
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            if (gameObject != null)
            {
                gameObject.GetComponentsInChildren<T>(includeInactive, results);
            }
            else
            {
                results.Clear();
            }
        }

        public void GetComponentsInChildren<T>(List<T> results) where T : class =>
            GetComponentsInChildren<T>(false, results);

        public Component[] GetComponentsInChildren(Type type, bool includeInactive = false)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponentsInChildren(type, includeInactive);
            }

            GameObject.ValidateComponentType(type);
            return Array.Empty<Component>();
        }

        public T? GetComponentInParent<T>(bool includeInactive = false) where T : class =>
            gameObject != null ? gameObject.GetComponentInParent<T>(includeInactive) : null;

        public Component? GetComponentInParent(Type type) => GetComponentInParent(type, false);

        public Component? GetComponentInParent(Type type, bool includeInactive)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponentInParent(type, includeInactive);
            }

            GameObject.ValidateComponentType(type);
            return null;
        }

        public T[] GetComponentsInParent<T>(bool includeInactive = false) where T : class =>
            gameObject != null ? gameObject.GetComponentsInParent<T>(includeInactive) : Array.Empty<T>();

        public void GetComponentsInParent<T>(bool includeInactive, List<T> results) where T : class
        {
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            if (gameObject != null)
            {
                gameObject.GetComponentsInParent<T>(includeInactive, results);
            }
            else
            {
                results.Clear();
            }
        }

        public Component[] GetComponentsInParent(Type type, bool includeInactive = false)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponentsInParent(type, includeInactive);
            }

            GameObject.ValidateComponentType(type);
            return Array.Empty<Component>();
        }

        public T? AddComponent<T>() where T : Component, new() => gameObject != null ? gameObject.AddComponent<T>() : null;

        public Component? AddComponent(Type componentType) => gameObject != null ? gameObject.AddComponent(componentType) : null;
    }
}
