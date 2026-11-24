using System;

namespace UnityEngine
{
    public class Component : Object
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

        public T? GetComponent<T>() where T : Component => gameObject != null ? gameObject.GetComponent<T>() : null;

        public Component? GetComponent(Type type)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponent(type);
            }

            GameObject.ValidateComponentType(type);
            return null;
        }

        public T[] GetComponents<T>() where T : Component => gameObject != null ? gameObject.GetComponents<T>() : Array.Empty<T>();

        public Component[] GetComponents(Type type)
        {
            if (gameObject != null)
            {
                return gameObject.GetComponents(type);
            }

            GameObject.ValidateComponentType(type);
            return Array.Empty<Component>();
        }

        public T? GetComponentInChildren<T>(bool includeInactive = false) where T : Component => gameObject != null ? gameObject.GetComponentInChildren<T>(includeInactive) : null;

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

        public T? GetComponentInParent<T>(bool includeInactive = false) where T : Component => gameObject != null ? gameObject.GetComponentInParent<T>(includeInactive) : null;

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

        public T? AddComponent<T>() where T : Component, new() => gameObject != null ? gameObject.AddComponent<T>() : null;

        public Component? AddComponent(Type componentType) => gameObject != null ? gameObject.AddComponent(componentType) : null;
    }
}
