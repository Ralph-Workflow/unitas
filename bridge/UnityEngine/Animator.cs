namespace UnityEngine
{
    public interface IAnimationProvider
    {
        ulong AttachAnimator(ulong entityId);

        ulong CreateController(AnimatorController controller);

        void DestroyController(ulong controllerHandle);

        bool SetController(ulong entityId, ulong controllerHandle);

        bool SetFloat(ulong entityId, string name, float value);

        bool GetFloat(ulong entityId, string name, out float value);

        bool SetInteger(ulong entityId, string name, int value);

        bool GetInteger(ulong entityId, string name, out int value);

        bool SetBool(ulong entityId, string name, bool value);

        bool GetBool(ulong entityId, string name, out bool value);

        bool SetTrigger(ulong entityId, string name);

        bool ResetTrigger(ulong entityId, string name);

        bool SetSpeed(ulong entityId, float speed);

        bool GetSpeed(ulong entityId, out float speed);

        bool GetStateInfo(ulong entityId, out AnimatorStateInfo stateInfo);
    }

    public sealed class Animator : Behaviour
    {
        private AnimatorController? _controller;
        private AnimatorController? _appliedController;
        private ulong _nativeControllerHandle;
        private ulong _nativeEntityId;
        private float _speed = 1f;

        public static IAnimationProvider? Provider { get; set; }

        public AnimatorController? runtimeAnimatorController
        {
            get => _controller;
            set
            {
                _controller = value;
                IAnimationProvider? provider = Provider;
                if (provider == null || value == null)
                {
                    return;
                }

                ulong entityId = EnsureNativeEntityId();
                if (entityId == 0UL)
                {
                    return;
                }

                ApplyController(provider, entityId);
            }
        }

        public float speed
        {
            get => _speed;
            set
            {
                _speed = value < 0f ? 0f : value;
                IAnimationProvider? provider = Provider;
                if (provider == null)
                {
                    return;
                }

                ulong entityId = EnsureNativeEntityId();
                if (entityId == 0UL)
                {
                    return;
                }

                provider.SetSpeed(entityId, _speed);
            }
        }

        public void SetFloat(string name, float value)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return;
            }

            provider.SetFloat(entityId, name, value);
        }

        public float GetFloat(string name)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return 0f;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return 0f;
            }

            return provider.GetFloat(entityId, name, out float value) ? value : 0f;
        }

        public void SetInteger(string name, int value)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return;
            }

            provider.SetInteger(entityId, name, value);
        }

        public int GetInteger(string name)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return 0;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return 0;
            }

            return provider.GetInteger(entityId, name, out int value) ? value : 0;
        }

        public void SetBool(string name, bool value)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return;
            }

            provider.SetBool(entityId, name, value);
        }

        public bool GetBool(string name)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return false;
            }

            return provider.GetBool(entityId, name, out bool value) && value;
        }

        public void SetTrigger(string name)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return;
            }

            provider.SetTrigger(entityId, name);
        }

        public void ResetTrigger(string name)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return;
            }

            provider.ResetTrigger(entityId, name);
        }

        public AnimatorStateInfo GetCurrentAnimatorStateInfo(int layer = 0)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null || layer != 0)
            {
                return AnimatorStateInfo.Empty;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return AnimatorStateInfo.Empty;
            }

            return provider.GetStateInfo(entityId, out AnimatorStateInfo stateInfo)
                ? stateInfo
                : AnimatorStateInfo.Empty;
        }

        private ulong EnsureNativeEntityId()
        {
            if (_nativeEntityId != 0UL)
            {
                return _nativeEntityId;
            }

            GameObject? owner = gameObject;
            ulong ownerEntityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return ownerEntityId;
            }

            _nativeEntityId = provider.AttachAnimator(ownerEntityId);
            if (owner != null && !owner.IsDestroyed && owner.NativeEntityId == 0UL && _nativeEntityId != 0UL)
            {
                owner.AssignNativeEntityId(_nativeEntityId);
            }

            if (_nativeEntityId != 0UL && _controller != null)
            {
                ApplyController(provider, _nativeEntityId);
            }

            return _nativeEntityId;
        }

        private void ApplyController(IAnimationProvider provider, ulong entityId)
        {
            if (_controller == null)
            {
                return;
            }

            if (ReferenceEquals(_appliedController, _controller) && _nativeControllerHandle != 0UL)
            {
                provider.SetController(entityId, _nativeControllerHandle);
                return;
            }

            ulong controllerHandle = provider.CreateController(_controller);
            if (controllerHandle == 0UL)
            {
                return;
            }

            if (_nativeControllerHandle != 0UL)
            {
                provider.DestroyController(_nativeControllerHandle);
            }

            _nativeControllerHandle = controllerHandle;
            _appliedController = _controller;
            provider.SetController(entityId, controllerHandle);
        }
    }
}
