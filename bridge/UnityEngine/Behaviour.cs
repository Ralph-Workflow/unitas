namespace UnityEngine
{
    public class Behaviour : Component
    {
        private bool _enabled = true;

        public bool enabled
        {
            get
            {
                ThrowIfUnassignedReference();
                return _enabled;
            }
            set
            {
                ThrowIfUnassignedReference();
                if (_enabled == value)
                {
                    return;
                }

                _enabled = value;
                OnEnabledChanged(value);
            }
        }

        public bool isActiveAndEnabled => !IsDestroyed && !IsUnassignedReference && _enabled && OwnerOrNull != null && OwnerOrNull.activeInHierarchy;

        internal virtual void OnEnabledChanged(bool isEnabled)
        {
        }
    }
}
