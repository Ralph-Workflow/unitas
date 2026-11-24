using System.Collections.Generic;

namespace UnityEngine
{
    public class GameObject : Object
    {
        private readonly List<Component> _components = new List<Component>();

        public Transform transform { get; }

        public bool activeSelf { get; set; }

        public GameObject() : this("New Game Object")
        {
        }

        public GameObject(string name)
        {
            this.name = name;
            activeSelf = true;
            Transform t = new Transform();
            t.gameObject = this;
            transform = t;
            _components.Add(t);
        }

        public void SetActive(bool value)
        {
            activeSelf = value;
            if (!value)
            {
                for (int i = 0; i < _components.Count; i++)
                {
                    if (_components[i] is MonoBehaviour behaviour)
                    {
                        CoroutineScheduler.OwnerDeactivated(behaviour);
                    }
                }
            }
        }

        public T AddComponent<T>() where T : Component, new()
        {
            T component = new T();
            AttachComponent(component);
            return component;
        }

        internal void AttachComponent(Component component)
        {
            component.gameObject = this;
            _components.Add(component);
        }

        public T? GetComponent<T>() where T : Component
        {
            for (int i = 0; i < _components.Count; i++)
            {
                if (_components[i] is T match)
                {
                    return match;
                }
            }
            return null;
        }
    }
}
