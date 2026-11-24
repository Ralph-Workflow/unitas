namespace UnityEngine
{
    public class Behaviour : Component
    {
        private bool _enabled = true;

        public bool enabled
        {
            get => _enabled;
            set
            {
                if (_enabled == value)
                {
                    return;
                }

                _enabled = value;
                OnEnabledChanged(value);
            }
        }

        public bool isActiveAndEnabled => _enabled && gameObject != null && gameObject.activeInHierarchy;

        internal virtual void OnEnabledChanged(bool isEnabled)
        {
        }
    }
}
