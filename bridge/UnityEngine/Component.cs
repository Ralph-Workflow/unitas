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

        public T? AddComponent<T>() where T : Component, new() => gameObject != null ? gameObject.AddComponent<T>() : null;
    }
}
