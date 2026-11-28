using System.Text;

namespace UnityEngine
{
    public enum AvatarIKGoal
    {
        LeftFoot = 0,
        RightFoot = 1,
        LeftHand = 2,
        RightHand = 3,
    }

    public enum AvatarIKHint
    {
        LeftKnee = 0,
        RightKnee = 1,
        LeftElbow = 2,
        RightElbow = 3,
    }

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

        bool Play(ulong entityId, string stateName);

        bool PlayHash(ulong entityId, int stateNameHash);

        bool PlayQueued(ulong entityId, string stateName);

        bool CrossFade(ulong entityId, string stateName, float normalizedTransitionDuration);

        bool CrossFadeHash(ulong entityId, int stateNameHash, float normalizedTransitionDuration);

        bool SetApplyRootMotion(ulong entityId, bool apply);

        bool GetApplyRootMotion(ulong entityId, out bool apply);

        bool GetRootMotion(ulong entityId, out Vector3 deltaPosition, out Quaternion deltaRotation);

        bool SetIKPosition(ulong entityId, AvatarIKGoal goal, Vector3 goalPosition);

        bool GetIKPosition(ulong entityId, AvatarIKGoal goal, out Vector3 goalPosition);

        bool SetIKPositionWeight(ulong entityId, AvatarIKGoal goal, float value);

        bool GetIKPositionWeight(ulong entityId, AvatarIKGoal goal, out float value);

        bool SetIKRotation(ulong entityId, AvatarIKGoal goal, Quaternion goalRotation);

        bool GetIKRotation(ulong entityId, AvatarIKGoal goal, out Quaternion goalRotation);

        bool SetIKRotationWeight(ulong entityId, AvatarIKGoal goal, float value);

        bool GetIKRotationWeight(ulong entityId, AvatarIKGoal goal, out float value);

        bool SetIKHintPosition(ulong entityId, AvatarIKHint hint, Vector3 hintPosition);

        bool GetIKHintPosition(ulong entityId, AvatarIKHint hint, out Vector3 hintPosition);

        bool SetIKHintPositionWeight(ulong entityId, AvatarIKHint hint, float value);

        bool GetIKHintPositionWeight(ulong entityId, AvatarIKHint hint, out float value);

        bool SetLookAtPosition(ulong entityId, Vector3 lookAtPosition);

        bool SetLookAtWeight(ulong entityId, float weight, float bodyWeight, float headWeight, float eyesWeight, float clampWeight);
    }

    public sealed class Animator : Behaviour
    {
        private AnimatorController? _controller;
        private AnimatorController? _appliedController;
        private ulong _nativeControllerHandle;
        private ulong _nativeEntityId;
        private float _speed = 1f;
        private bool _applyRootMotion;
        private bool _hasExplicitApplyRootMotion;

        public static IAnimationProvider? Provider { get; set; }

        public static int StringToHash(string name)
        {
            byte[] bytes = Encoding.UTF8.GetBytes(name ?? string.Empty);
            ulong hash = 0x811c9dc5UL;
            for (int i = 0; i < bytes.Length; i++)
            {
                hash ^= bytes[i];
                hash = (hash * 0x01000193UL) & 0xffffffffUL;
            }

            return hash <= 0x7fffffffUL ? (int)hash : (int)((long)hash - 0x100000000L);
        }

        internal static void BindNativeAnimator(GameObject owner)
        {
            IAnimationProvider? provider = Provider;
            ulong entity = owner.NativeEntityId;
            if (provider == null || entity == 0UL || owner.GetComponent<Animator>() != null)
            {
                return;
            }

            if (!provider.GetSpeed(entity, out float nativeSpeed))
            {
                return;
            }

            Animator animator = new Animator
            {
                _nativeEntityId = entity,
                _speed = nativeSpeed,
            };
            owner.AttachComponent(animator, typeof(Animator));
        }

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

        public bool applyRootMotion
        {
            get
            {
                IAnimationProvider? provider = Provider;
                if (provider == null)
                {
                    return _applyRootMotion;
                }

                ulong entityId = EnsureNativeEntityId();
                if (entityId == 0UL)
                {
                    return _applyRootMotion;
                }

                return provider.GetApplyRootMotion(entityId, out bool apply) ? apply : _applyRootMotion;
            }
            set
            {
                _applyRootMotion = value;
                _hasExplicitApplyRootMotion = true;
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

                provider.SetApplyRootMotion(entityId, value);
            }
        }

        public Vector3 deltaPosition
        {
            get
            {
                IAnimationProvider? provider = Provider;
                if (provider == null)
                {
                    return Vector3.zero;
                }

                ulong entityId = EnsureNativeEntityId();
                if (entityId == 0UL)
                {
                    return Vector3.zero;
                }

                return provider.GetRootMotion(entityId, out Vector3 deltaPos, out _) ? deltaPos : Vector3.zero;
            }
        }

        public Quaternion deltaRotation
        {
            get
            {
                IAnimationProvider? provider = Provider;
                if (provider == null)
                {
                    return Quaternion.identity;
                }

                ulong entityId = EnsureNativeEntityId();
                if (entityId == 0UL)
                {
                    return Quaternion.identity;
                }

                return provider.GetRootMotion(entityId, out _, out Quaternion deltaRot) ? deltaRot : Quaternion.identity;
            }
        }

        public void Play(string stateName, int layer = -1, float normalizedTime = float.NegativeInfinity)
        {
            _ = layer;
            _ = normalizedTime;
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

            provider.Play(entityId, stateName);
        }

        public void Play(int stateNameHash, int layer = -1, float normalizedTime = float.NegativeInfinity)
        {
            _ = layer;
            _ = normalizedTime;
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

            provider.PlayHash(entityId, stateNameHash);
        }

        public void PlayQueued(string stateName)
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

            provider.PlayQueued(entityId, stateName);
        }

        public void CrossFade(string stateName, float normalizedTransitionDuration, int layer = -1, float normalizedTimeOffset = float.NegativeInfinity)
        {
            _ = layer;
            _ = normalizedTimeOffset;
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

            provider.CrossFade(entityId, stateName, normalizedTransitionDuration);
        }

        public void CrossFade(int stateNameHash, float normalizedTransitionDuration, int layer = -1, float normalizedTimeOffset = float.NegativeInfinity)
        {
            _ = layer;
            _ = normalizedTimeOffset;
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

            provider.CrossFadeHash(entityId, stateNameHash, normalizedTransitionDuration);
        }

        public void SetIKPosition(AvatarIKGoal goal, Vector3 goalPosition)
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

            provider.SetIKPosition(entityId, goal, goalPosition);
        }

        public Vector3 GetIKPosition(AvatarIKGoal goal)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return Vector3.zero;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return Vector3.zero;
            }

            return provider.GetIKPosition(entityId, goal, out Vector3 position) ? position : Vector3.zero;
        }

        public void SetIKPositionWeight(AvatarIKGoal goal, float value)
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

            provider.SetIKPositionWeight(entityId, goal, value);
        }

        public float GetIKPositionWeight(AvatarIKGoal goal)
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

            return provider.GetIKPositionWeight(entityId, goal, out float weight) ? weight : 0f;
        }

        public void SetIKRotation(AvatarIKGoal goal, Quaternion goalRotation)
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

            provider.SetIKRotation(entityId, goal, goalRotation);
        }

        public Quaternion GetIKRotation(AvatarIKGoal goal)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return Quaternion.identity;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return Quaternion.identity;
            }

            return provider.GetIKRotation(entityId, goal, out Quaternion rotation) ? rotation : Quaternion.identity;
        }

        public void SetIKRotationWeight(AvatarIKGoal goal, float value)
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

            provider.SetIKRotationWeight(entityId, goal, value);
        }

        public float GetIKRotationWeight(AvatarIKGoal goal)
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

            return provider.GetIKRotationWeight(entityId, goal, out float weight) ? weight : 0f;
        }

        public void SetIKHintPosition(AvatarIKHint hint, Vector3 hintPosition)
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

            provider.SetIKHintPosition(entityId, hint, hintPosition);
        }

        public Vector3 GetIKHintPosition(AvatarIKHint hint)
        {
            IAnimationProvider? provider = Provider;
            if (provider == null)
            {
                return Vector3.zero;
            }

            ulong entityId = EnsureNativeEntityId();
            if (entityId == 0UL)
            {
                return Vector3.zero;
            }

            return provider.GetIKHintPosition(entityId, hint, out Vector3 position) ? position : Vector3.zero;
        }

        public void SetIKHintPositionWeight(AvatarIKHint hint, float value)
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

            provider.SetIKHintPositionWeight(entityId, hint, value);
        }

        public float GetIKHintPositionWeight(AvatarIKHint hint)
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

            return provider.GetIKHintPositionWeight(entityId, hint, out float weight) ? weight : 0f;
        }

        public void SetLookAtPosition(Vector3 lookAtPosition)
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

            provider.SetLookAtPosition(entityId, lookAtPosition);
        }

        public void SetLookAtWeight(
            float weight,
            float bodyWeight = 0f,
            float headWeight = 1f,
            float eyesWeight = 0f,
            float clampWeight = 0.5f)
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

            provider.SetLookAtWeight(entityId, weight, bodyWeight, headWeight, eyesWeight, clampWeight);
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

            if (_nativeEntityId != 0UL && _hasExplicitApplyRootMotion)
            {
                provider.SetApplyRootMotion(_nativeEntityId, _applyRootMotion);
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
