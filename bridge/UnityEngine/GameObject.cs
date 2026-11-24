using System.Collections.Generic;

namespace UnityEngine
{
    public class GameObject : Object
    {
        private readonly List<Component> _components = new List<Component>();

        public Transform transform { get; }

        public bool activeSelf { get; set; }

        internal IReadOnlyList<Component> Components => _components;

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
