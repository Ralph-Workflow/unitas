using System;
using System.Collections.Generic;
using UnityEngine.SceneManagement;

namespace UnityEngine
{
    public partial class GameObject : Object
    {
        private static readonly HashSet<string> s_definedTags = CreateDefaultDefinedTags();
        private readonly List<Component> _components = new List<Component>();
        private string _tag = "Untagged";
        private ulong _nativeEntityId;
        private bool _isScriptedPlaceholder;

        static GameObject()
        {
            SceneManager.sceneUnloaded += OnSceneUnloaded;
            SceneManager.sceneLoaded += OnSceneLoaded;
        }

        private static HashSet<string> CreateDefaultDefinedTags()
        {
            return new HashSet<string>(StringComparer.Ordinal)
            {
                "Untagged",
                "Respawn",
                "Finish",
                "EditorOnly",
                "MainCamera",
                "Player",
                "GameController",
                "Enemy"
            };
        }

        public static void RegisterTag(string tagName)
        {
            if (string.IsNullOrEmpty(tagName))
            {
                throw new ArgumentException("Tag cannot be null or empty.", nameof(tagName));
            }

            s_definedTags.Add(tagName);
        }

        public static bool IsTagDefined(string tagName)
        {
            if (tagName == null)
            {
                return false;
            }

            return s_definedTags.Contains(tagName);
        }

        public static void ResetDefinedTags()
        {
            s_definedTags.Clear();
            foreach (string builtIn in CreateDefaultDefinedTags())
            {
                s_definedTags.Add(builtIn);
            }
        }

        internal static void ValidateTagDefined(string tagName)
        {
            if (tagName == null)
            {
                throw new ArgumentNullException(nameof(tagName));
            }

            if (tagName.Length == 0 || !s_definedTags.Contains(tagName))
            {
                throw new UnityException("Tag: " + tagName + " is not defined.");
            }
        }

        public static GameObject CreateUnassignedReference(string fieldName, string ownerName = "MonoBehaviour")
        {
            GameObject placeholder = new GameObject("UnassignedReferencePlaceholder");
            DeregisterObjectAndComponents(placeholder);
            placeholder.MarkAsUnassignedReference(fieldName, ownerName);
            return placeholder;
        }

        public Transform transform { get; }

        public bool activeSelf { get; set; }

        public int layer { get; set; }

        public string tag
        {
            get
            {
                ThrowIfDestroyed();
                return _tag;
            }
            set
            {
                ThrowIfDestroyed();
                if (value == null)
                {
                    throw new ArgumentNullException(nameof(value));
                }

                s_definedTags.Add(value);
                _tag = value;
            }
        }

        public bool CompareTag(string tag)
        {
            ThrowIfDestroyed();
            ValidateTagDefined(tag);
            return string.Equals(this.tag, tag, StringComparison.Ordinal);
        }

        internal IReadOnlyList<Component> Components => _components;

        internal ulong NativeEntityId => _nativeEntityId;

        internal void AssignNativeEntityId(ulong entity)
        {
            _nativeEntityId = entity;
        }

        public GameObject() : this("New Game Object")
        {
        }

        public GameObject(string name)
        {
            this.name = name;
            _isScriptedPlaceholder = string.Equals(name, "Scripted Game Object", StringComparison.Ordinal);
            activeSelf = true;
            scene = SceneManager.GetActiveScene();
            Transform t = new Transform();
            AttachComponent(t);
            transform = t;
        }

        public GameObject(string name, params Type[] components) : this(name)
        {
            if (components == null)
            {
                throw new ArgumentNullException(nameof(components));
            }

            for (int i = 0; i < components.Length; i++)
            {
                AddComponent(components[i]);
            }
        }

        public bool activeInHierarchy
        {
            get
            {
                if (!activeSelf)
                {
                    return false;
                }

                Transform? ancestor = transform.parent;
                while (ancestor != null)
                {
                    GameObject owner = ancestor.gameObject;
                    if (owner != null && !owner.activeSelf)
                    {
                        return false;
                    }

                    ancestor = ancestor.parent;
                }

                return true;
            }
        }

        public static GameObject? Find(string name)
        {
            if (name == null)
            {
                throw new ArgumentNullException(nameof(name));
            }

            if (name.Length == 0)
            {
                return null;
            }

            List<Object> snapshot = SnapshotLiveObjects();
            if (!name.Contains('/', StringComparison.Ordinal))
            {
                for (int i = 0; i < snapshot.Count; i++)
                {
                    if (snapshot[i] is GameObject candidate
                        && !candidate.IsDestroyed
                        && candidate.activeInHierarchy
                        && string.Equals(candidate.name, name, StringComparison.Ordinal))
                    {
                        return candidate;
                    }
                }

                return null;
            }

            bool requireRoot = name[0] == '/';
            string trimmed = requireRoot ? name.Substring(1) : name;
            if (trimmed.Length == 0)
            {
                return null;
            }

            string[] segments = trimmed.Split('/');
            for (int i = 0; i < segments.Length; i++)
            {
                if (segments[i].Length == 0)
                {
                    return null;
                }
            }

            for (int i = 0; i < snapshot.Count; i++)
            {
                if (!(snapshot[i] is GameObject start)
                    || start.IsDestroyed
                    || !start.activeInHierarchy
                    || !string.Equals(start.name, segments[0], StringComparison.Ordinal))
                {
                    continue;
                }

                if (requireRoot && start.transform.parent != null)
                {
                    continue;
                }

                GameObject? matched = MatchHierarchyPath(start, segments, 1);
                if (matched != null)
                {
                    return matched;
                }
            }

            return null;
        }

        public static GameObject? FindWithTag(string tag)
        {
            if (tag == null)
            {
                throw new ArgumentNullException(nameof(tag));
            }

            List<Object> snapshot = SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                if (snapshot[i] is GameObject candidate
                    && !candidate.IsDestroyed
                    && candidate.activeInHierarchy
                    && string.Equals(candidate._tag, tag, StringComparison.Ordinal))
                {
                    return candidate;
                }
            }

            return null;
        }

        public static GameObject? FindGameObjectWithTag(string tag) => FindWithTag(tag);

        public static GameObject[] FindGameObjectsWithTag(string tag)
        {
            if (tag == null)
            {
                throw new ArgumentNullException(nameof(tag));
            }

            List<Object> snapshot = SnapshotLiveObjects();
            List<GameObject> matches = new List<GameObject>();
            for (int i = 0; i < snapshot.Count; i++)
            {
                if (snapshot[i] is GameObject candidate
                    && !candidate.IsDestroyed
                    && candidate.activeInHierarchy
                    && string.Equals(candidate._tag, tag, StringComparison.Ordinal))
                {
                    matches.Add(candidate);
                }
            }

            return matches.ToArray();
        }

        public void SetActive(bool value)
        {
            if (value == activeSelf)
            {
                return;
            }

            bool wasEffective = activeInHierarchy;
            activeSelf = value;
            bool isEffective = activeInHierarchy;

            if (!value)
            {
                StopCoroutinesInSubtree();
            }

            if (wasEffective == isEffective)
            {
                return;
            }

            List<MonoBehaviour> disabling = new List<MonoBehaviour>();
            List<MonoBehaviour> enabling = new List<MonoBehaviour>();
            CollectActivationChanges(this, wasEffective, isEffective, disabling, enabling);

            for (int i = 0; i < disabling.Count; i++)
            {
                MonoBehaviour behaviour = disabling[i];
                if (behaviour.enabled)
                {
                    behaviour.OnDisable();
                }
            }

            for (int i = 0; i < enabling.Count; i++)
            {
                MonoBehaviour behaviour = enabling[i];
                if (behaviour.enabled)
                {
                    behaviour.OnEnable();
                }
            }
        }

        public T AddComponent<T>() where T : Component, new()
        {
            ThrowIfDestroyed();
            T component = new T();
            component.AttachedRuntimeType = typeof(T);
            if (!TryAttachWithRules(component, typeof(T)))
            {
                DeregisterLiveObject(component);
                return null!;
            }

            return component;
        }

        public Component AddComponent(Type componentType)
        {
            ThrowIfDestroyed();
            if (componentType == null)
            {
                throw new ArgumentNullException(nameof(componentType));
            }

            if (!typeof(Component).IsAssignableFrom(componentType) || componentType.IsAbstract)
            {
                throw new ArgumentException("Type must be a concrete UnityEngine.Component.", nameof(componentType));
            }

            object? created;
            try
            {
                created = Activator.CreateInstance(componentType);
            }
            catch (MissingMethodException ex)
            {
                throw new ArgumentException("Component type must have a parameterless constructor: " + componentType.FullName, nameof(componentType), ex);
            }

            if (!(created is Component component))
            {
                throw new ArgumentException("Failed to instantiate UnityEngine.Component.", nameof(componentType));
            }

            component.AttachedRuntimeType = componentType;
            if (!TryAttachWithRules(component, componentType))
            {
                DeregisterLiveObject(component);
                return null!;
            }

            return component;
        }

        private bool TryAttachWithRules(Component component, Type componentType)
        {
            if (component.ResolveDisallowMultiple(componentType) && GetComponent(componentType) != null)
            {
                Debug.LogError(
                    "Can't add component '"
                    + componentType.Name
                    + "' to "
                    + name
                    + " because such a component is already added to the game object!",
                    this);
                return false;
            }

            IReadOnlyList<Type> requiredTypes = component.ResolveRequiredComponents(componentType);
            for (int i = 0; i < requiredTypes.Count; i++)
            {
                Type reqType = requiredTypes[i];
                if (reqType == null || !typeof(Component).IsAssignableFrom(reqType))
                {
                    continue;
                }

                if (GetComponent(reqType) == null && !reqType.IsAbstract)
                {
                    AddComponent(reqType);
                }
            }

            AttachComponent(component);
            return true;
        }

        internal bool TryFindDependentRequiring(Component targetComponent, out Component? dependentComponent)
        {
            dependentComponent = null;
            for (int i = 0; i < _components.Count; i++)
            {
                Component candidate = _components[i];
                if (candidate is null || candidate.IsDestroyed || ReferenceEquals(candidate, targetComponent))
                {
                    continue;
                }

                Type candidateType = candidate.AttachedRuntimeType ?? typeof(Component);
                IReadOnlyList<Type> requiredTypes = candidate.ResolveRequiredComponents(candidateType);
                for (int r = 0; r < requiredTypes.Count; r++)
                {
                    Type reqType = requiredTypes[r];
                    if (reqType == null || !reqType.IsInstanceOfType(targetComponent))
                    {
                        continue;
                    }

                    bool anotherSatisfies = false;
                    for (int j = 0; j < _components.Count; j++)
                    {
                        Component other = _components[j];
                        if (other is not null
                            && !other.IsDestroyed
                            && !ReferenceEquals(other, targetComponent)
                            && reqType.IsInstanceOfType(other))
                        {
                            anotherSatisfies = true;
                            break;
                        }
                    }

                    if (!anotherSatisfies)
                    {
                        dependentComponent = candidate;
                        return true;
                    }
                }
            }

            return false;
        }

        internal void AttachComponent(Component component)
        {
            GameObject? previousOwner = component.OwnerOrNull;
            if (previousOwner is not null && !ReferenceEquals(previousOwner, this) && previousOwner._isScriptedPlaceholder)
            {
                previousOwner._components.Remove(component);
                DeregisterObjectAndComponents(previousOwner);
            }

            if (component.AttachedRuntimeType == null && component is Transform)
            {
                component.AttachedRuntimeType = typeof(Transform);
            }

            component.gameObject = this;
            component.scene = scene;
            _components.Add(component);
        }

        public T? GetComponent<T>() where T : class
        {
            for (int i = 0; i < _components.Count; i++)
            {
                Component candidate = _components[i];
                if (candidate is not null && !candidate.IsDestroyed && candidate is T match)
                {
                    return match;
                }
            }

            return null;
        }

        public Component? GetComponent(Type type)
        {
            ValidateComponentType(type);
            for (int i = 0; i < _components.Count; i++)
            {
                Component candidate = _components[i];
                if (candidate is not null && !candidate.IsDestroyed && type.IsInstanceOfType(candidate))
                {
                    return candidate;
                }
            }

            return null;
        }

        public bool TryGetComponent<T>(out T component) where T : class
        {
            T? found = GetComponent<T>();
            if (found is not null)
            {
                component = found;
                return true;
            }

            component = null!;
            return false;
        }

        public bool TryGetComponent(Type type, out Component? component)
        {
            component = GetComponent(type);
            return component is not null;
        }

        public T[] GetComponents<T>() where T : class
        {
            List<T> matches = new List<T>();
            GetComponents<T>(matches);
            return matches.ToArray();
        }

        public void GetComponents<T>(List<T> results) where T : class
        {
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            results.Clear();
            for (int i = 0; i < _components.Count; i++)
            {
                Component candidate = _components[i];
                if (candidate is not null && !candidate.IsDestroyed && candidate is T match)
                {
                    results.Add(match);
                }
            }
        }

        public Component[] GetComponents(Type type)
        {
            List<Component> matches = new List<Component>();
            GetComponents(type, matches);
            return matches.ToArray();
        }

        public void GetComponents(Type type, List<Component> results)
        {
            ValidateComponentType(type);
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            results.Clear();
            for (int i = 0; i < _components.Count; i++)
            {
                Component candidate = _components[i];
                if (candidate is not null && !candidate.IsDestroyed && type.IsInstanceOfType(candidate))
                {
                    results.Add(candidate);
                }
            }
        }

        public T? GetComponentInChildren<T>(bool includeInactive = false) where T : class
        {
            if (IsDestroyed)
            {
                return null;
            }

            if (!includeInactive && !activeInHierarchy)
            {
                return null;
            }

            return SearchComponentInChildren<T>(this, includeInactive);
        }

        public Component? GetComponentInChildren(Type type) => GetComponentInChildren(type, false);

        public Component? GetComponentInChildren(Type type, bool includeInactive)
        {
            ValidateComponentType(type);
            if (IsDestroyed)
            {
                return null;
            }

            if (!includeInactive && !activeInHierarchy)
            {
                return null;
            }

            return SearchComponentInChildren(this, type, includeInactive);
        }

        public T[] GetComponentsInChildren<T>(bool includeInactive = false) where T : class
        {
            List<T> results = new List<T>();
            GetComponentsInChildren<T>(includeInactive, results);
            return results.ToArray();
        }

        public void GetComponentsInChildren<T>(bool includeInactive, List<T> results) where T : class
        {
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            results.Clear();
            if (IsDestroyed)
            {
                return;
            }

            if (!includeInactive && !activeInHierarchy)
            {
                return;
            }

            CollectComponentsInChildren<T>(this, includeInactive, results);
        }

        public void GetComponentsInChildren<T>(List<T> results) where T : class =>
            GetComponentsInChildren<T>(false, results);

        public Component[] GetComponentsInChildren(Type type, bool includeInactive = false)
        {
            ValidateComponentType(type);
            if (IsDestroyed)
            {
                return Array.Empty<Component>();
            }

            if (!includeInactive && !activeInHierarchy)
            {
                return Array.Empty<Component>();
            }

            List<Component> results = new List<Component>();
            CollectComponentsInChildren(this, type, includeInactive, results);
            return results.ToArray();
        }

        public T? GetComponentInParent<T>(bool includeInactive = false) where T : class
        {
            GameObject? current = this;
            while (current is not null && !current.IsDestroyed)
            {
                if (includeInactive || current.activeInHierarchy)
                {
                    T? found = current.GetComponent<T>();
                    if (found is not null)
                    {
                        return found;
                    }
                }

                Transform? parentTransform = current.transform.parent;
                current = parentTransform is not null ? parentTransform.gameObject : null;
            }

            return null;
        }

        public Component? GetComponentInParent(Type type) => GetComponentInParent(type, false);

        public Component? GetComponentInParent(Type type, bool includeInactive)
        {
            ValidateComponentType(type);
            GameObject? current = this;
            while (current is not null && !current.IsDestroyed)
            {
                if (includeInactive || current.activeInHierarchy)
                {
                    Component? found = current.GetComponent(type);
                    if (found is not null)
                    {
                        return found;
                    }
                }

                Transform? parentTransform = current.transform.parent;
                current = parentTransform is not null ? parentTransform.gameObject : null;
            }

            return null;
        }

        public T[] GetComponentsInParent<T>(bool includeInactive = false) where T : class
        {
            List<T> results = new List<T>();
            GetComponentsInParent<T>(includeInactive, results);
            return results.ToArray();
        }

        public void GetComponentsInParent<T>(bool includeInactive, List<T> results) where T : class
        {
            if (results == null)
            {
                throw new ArgumentNullException(nameof(results));
            }

            results.Clear();
            GameObject? current = this;
            while (current is not null && !current.IsDestroyed)
            {
                if (includeInactive || current.activeInHierarchy)
                {
                    for (int i = 0; i < current._components.Count; i++)
                    {
                        Component candidate = current._components[i];
                        if (candidate is not null && !candidate.IsDestroyed && candidate is T match)
                        {
                            results.Add(match);
                        }
                    }
                }

                Transform? parentTransform = current.transform.parent;
                current = parentTransform is not null ? parentTransform.gameObject : null;
            }
        }

        public Component[] GetComponentsInParent(Type type, bool includeInactive = false)
        {
            ValidateComponentType(type);
            List<Component> results = new List<Component>();
            GameObject? current = this;
            while (current is not null && !current.IsDestroyed)
            {
                if (includeInactive || current.activeInHierarchy)
                {
                    for (int i = 0; i < current._components.Count; i++)
                    {
                        Component candidate = current._components[i];
                        if (candidate is not null && !candidate.IsDestroyed && type.IsInstanceOfType(candidate))
                        {
                            results.Add(candidate);
                        }
                    }
                }

                Transform? parentTransform = current.transform.parent;
                current = parentTransform is not null ? parentTransform.gameObject : null;
            }

            return results.ToArray();
        }

        private static T? SearchComponentInChildren<T>(GameObject node, bool includeInactive) where T : class
        {
            if (node is null || node.IsDestroyed)
            {
                return null;
            }

            if (!includeInactive && !node.activeInHierarchy)
            {
                return null;
            }

            T? onSelf = node.GetComponent<T>();
            if (onSelf is not null)
            {
                return onSelf;
            }

            for (int i = 0; i < node.transform.childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child is null)
                {
                    continue;
                }

                T? found = SearchComponentInChildren<T>(child, includeInactive);
                if (found is not null)
                {
                    return found;
                }
            }

            return null;
        }

        private static Component? SearchComponentInChildren(GameObject node, Type type, bool includeInactive)
        {
            if (node is null || node.IsDestroyed)
            {
                return null;
            }

            if (!includeInactive && !node.activeInHierarchy)
            {
                return null;
            }

            Component? onSelf = node.GetComponent(type);
            if (onSelf is not null)
            {
                return onSelf;
            }

            for (int i = 0; i < node.transform.childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child is null)
                {
                    continue;
                }

                Component? found = SearchComponentInChildren(child, type, includeInactive);
                if (found is not null)
                {
                    return found;
                }
            }

            return null;
        }

        private static void CollectComponentsInChildren<T>(GameObject node, bool includeInactive, List<T> results) where T : class
        {
            if (node is null || node.IsDestroyed)
            {
                return;
            }

            if (!includeInactive && !node.activeInHierarchy)
            {
                return;
            }

            for (int i = 0; i < node._components.Count; i++)
            {
                Component candidate = node._components[i];
                if (candidate is not null && !candidate.IsDestroyed && candidate is T match)
                {
                    results.Add(match);
                }
            }

            for (int i = 0; i < node.transform.childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child is null)
                {
                    continue;
                }

                CollectComponentsInChildren<T>(child, includeInactive, results);
            }
        }

        private static void CollectComponentsInChildren(GameObject node, Type type, bool includeInactive, List<Component> results)
        {
            if (node is null || node.IsDestroyed)
            {
                return;
            }

            if (!includeInactive && !node.activeInHierarchy)
            {
                return;
            }

            for (int i = 0; i < node._components.Count; i++)
            {
                Component candidate = node._components[i];
                if (candidate is not null && !candidate.IsDestroyed && type.IsInstanceOfType(candidate))
                {
                    results.Add(candidate);
                }
            }

            for (int i = 0; i < node.transform.childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child is null)
                {
                    continue;
                }

                CollectComponentsInChildren(child, type, includeInactive, results);
            }
        }

        private static GameObject? MatchHierarchyPath(GameObject current, string[] segments, int segmentIndex)
        {
            if (segmentIndex >= segments.Length)
            {
                return current;
            }

            string targetName = segments[segmentIndex];
            for (int i = 0; i < current.transform.childCount; i++)
            {
                GameObject child = current.transform.GetChild(i).gameObject;
                if (child is null
                    || child.IsDestroyed
                    || !child.activeInHierarchy
                    || !string.Equals(child.name, targetName, StringComparison.Ordinal))
                {
                    continue;
                }

                GameObject? match = MatchHierarchyPath(child, segments, segmentIndex + 1);
                if (match is not null)
                {
                    return match;
                }
            }

            return null;
        }

        internal static void ValidateComponentType(Type type)
        {
            if (type == null)
            {
                throw new ArgumentNullException(nameof(type));
            }

            if (!typeof(Component).IsAssignableFrom(type) && !type.IsInterface)
            {
                throw new ArgumentException("Type must derive from UnityEngine.Component.", nameof(type));
            }
        }

        private static void OnSceneUnloaded(Scene unloadedScene)
        {
            if (!unloadedScene.IsValid())
            {
                return;
            }

            List<Object> snapshot = SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                Object candidate = snapshot[i];
                if (candidate is null || candidate.IsPersistent || candidate.scene != unloadedScene)
                {
                    continue;
                }

                if (candidate is GameObject go)
                {
                    if (!go.IsPersistent)
                    {
                        DeregisterObjectAndComponents(go);
                    }
                }
                else if (candidate is Component comp)
                {
                    GameObject? owner = comp.OwnerOrNull;
                    if (owner is null || !owner.IsPersistent)
                    {
                        DeregisterLiveObject(comp);
                    }
                }
                else
                {
                    DeregisterLiveObject(candidate);
                }
            }
        }

        private static void OnSceneLoaded(Scene loadedScene, LoadSceneMode mode)
        {
            if (mode != LoadSceneMode.Single || !loadedScene.IsValid())
            {
                return;
            }

            List<Object> snapshot = SnapshotLiveObjects();
            for (int i = 0; i < snapshot.Count; i++)
            {
                Object candidate = snapshot[i];
                if (candidate is null || candidate.IsPersistent || !candidate.scene.IsValid() || candidate.scene == loadedScene)
                {
                    continue;
                }

                if (candidate is GameObject go)
                {
                    if (!go.IsPersistent)
                    {
                        DeregisterObjectAndComponents(go);
                    }
                }
                else if (candidate is Component comp)
                {
                    GameObject? owner = comp.OwnerOrNull;
                    if (owner is null || !owner.IsPersistent)
                    {
                        DeregisterLiveObject(comp);
                    }
                }
                else
                {
                    DeregisterLiveObject(candidate);
                }
            }
        }

        private static void DeregisterObjectAndComponents(GameObject go)
        {
            DeregisterLiveObject(go);
            for (int i = 0; i < go._components.Count; i++)
            {
                DeregisterLiveObject(go._components[i]);
            }
        }

        private void CollectActivationChanges(
            GameObject node,
            bool wasEffective,
            bool isEffective,
            List<MonoBehaviour> disabling,
            List<MonoBehaviour> enabling)
        {
            for (int i = 0; i < node._components.Count; i++)
            {
                if (node._components[i] is MonoBehaviour behaviour)
                {
                    if (wasEffective)
                    {
                        disabling.Add(behaviour);
                    }
                    else if (isEffective)
                    {
                        enabling.Add(behaviour);
                    }
                }
            }

            for (int i = 0; i < node.transform.childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child == null)
                {
                    continue;
                }

                CollectActivationChanges(
                    child,
                    wasEffective && child.activeSelf,
                    isEffective && child.activeSelf,
                    disabling,
                    enabling);
            }
        }

        private void StopCoroutinesInSubtree()
        {
            for (int i = 0; i < _components.Count; i++)
            {
                if (_components[i] is MonoBehaviour behaviour)
                {
                    CoroutineScheduler.OwnerDeactivated(behaviour);
                }
                else if (_components[i] is AudioSource audioSource)
                {
                    audioSource.Stop();
                }
            }

            for (int i = 0; i < transform.childCount; i++)
            {
                GameObject child = transform.GetChild(i).gameObject;
                if (child == null)
                {
                    continue;
                }

                child.StopCoroutinesInSubtree();
            }
        }
    }
}
