using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public partial class Component : Object
    {
        private GameObject? _gameObject;

        public GameObject gameObject
        {
            get => _gameObject!;
            internal set => _gameObject = value;
        }

        public Transform transform => gameObject.transform;

        public override string name
        {
            get => gameObject != null ? gameObject.name : base.name;
            set
            {
                if (gameObject != null)
                {
                    gameObject.name = value;
                }
                else
                {
                    base.name = value;
                }
            }
        }

        public string tag
        {
            get => gameObject != null ? gameObject.tag : "Untagged";
            set
            {
                if (gameObject != null)
                {
                    gameObject.tag = value;
                }
            }
        }

        public bool CompareTag(string tag) => gameObject != null && gameObject.CompareTag(tag);

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
