using System.Collections.Generic;
using UnityEngine;

namespace Cinemachine
{
    public readonly struct CinemachineBrainState
    {
        public CinemachineBrainState(
            bool hasLiveCamera,
            string liveCameraName,
            Vector3 liveCameraPosition,
            Quaternion liveCameraRotation,
            bool hasBlend,
            string blendFrom,
            string blendTo,
            float blendWeight)
        {
            HasLiveCamera = hasLiveCamera;
            LiveCameraName = liveCameraName;
            LiveCameraPosition = liveCameraPosition;
            LiveCameraRotation = liveCameraRotation;
            HasBlend = hasBlend;
            BlendFrom = blendFrom;
            BlendTo = blendTo;
            BlendWeight = blendWeight;
        }

        public bool HasLiveCamera { get; }

        public string LiveCameraName { get; }

        public Vector3 LiveCameraPosition { get; }

        public Quaternion LiveCameraRotation { get; }

        public bool HasBlend { get; }

        public string BlendFrom { get; }

        public string BlendTo { get; }

        public float BlendWeight { get; }
    }

    public sealed class CinemachineBrain : MonoBehaviour
    {
        private readonly List<CinemachineVirtualCamera> _cameras = new List<CinemachineVirtualCamera>();
        private readonly ulong _world;

        public CinemachineBrain()
        {
            ICinemachineProvider? provider = CinemachineProvider.Current;
            _world = provider == null ? 0UL : provider.CreateWorld();
        }

        public CinemachineBlendDefinition DefaultBlend
        {
            get => _defaultBlend;
            set
            {
                _defaultBlend = value;
                ICinemachineProvider? provider = CinemachineProvider.Current;
                if (provider != null && _world != 0UL)
                {
                    provider.SetDefaultBlend(_world, value.Style, value.Duration);
                }
            }
        }

        public IReadOnlyList<CinemachineVirtualCamera> Cameras => _cameras;

        public CinemachineVirtualCamera? ActiveCamera { get; private set; }

        public Vector3 ActiveCameraPosition { get; private set; }

        public Quaternion ActiveCameraRotation { get; private set; }

        public string? ActiveBlendFrom { get; private set; }

        public string? ActiveBlendTo { get; private set; }

        public float ActiveBlendWeight { get; private set; }

        public CinemachineVirtualCamera AddCamera(string name, int priority)
        {
            CinemachineVirtualCamera camera = new CinemachineVirtualCamera(name, priority);
            AddCamera(camera);
            return camera;
        }

        public void AddCamera(CinemachineVirtualCamera camera)
        {
            ICinemachineProvider? provider = CinemachineProvider.Current;
            if (provider == null || _world == 0UL)
            {
                return;
            }
            ulong handle = provider.AddCamera(_world, camera.Name, camera.Priority);
            if (handle == 0UL)
            {
                return;
            }
            _cameras.Add(camera);
            camera.Attach(this, handle);
        }

        public void SetCustomBlend(string from, string to, CinemachineBlendDefinition blend)
        {
            ICinemachineProvider? provider = CinemachineProvider.Current;
            if (provider == null || _world == 0UL)
            {
                return;
            }
            provider.SetCustomBlend(_world, from, to, blend.Style, blend.Duration);
        }

        public void UpdateBrain(double deltaSeconds)
        {
            ICinemachineProvider? provider = CinemachineProvider.Current;
            if (provider == null || _world == 0UL)
            {
                return;
            }
            PushTargets(provider);
            provider.UpdateWorld(_world, deltaSeconds);
            ReadState(provider);
        }

        public override void LateUpdate()
        {
            UpdateBrain(Time.deltaTime);
        }

        public override void OnDestroy()
        {
            ICinemachineProvider? provider = CinemachineProvider.Current;
            if (provider != null && _world != 0UL)
            {
                provider.DestroyWorld(_world);
            }
        }

        internal void Push(CinemachineVirtualCamera camera)
        {
            ICinemachineProvider? provider = CinemachineProvider.Current;
            if (provider == null || _world == 0UL || camera.NativeHandle == 0UL)
            {
                return;
            }
            provider.SetCameraPriority(_world, camera.NativeHandle, camera.Priority);
            provider.SetCameraActive(_world, camera.NativeHandle, camera.Active);
            provider.SetCameraFollow(
                _world,
                camera.NativeHandle,
                camera.Follow != null,
                TargetId(camera.Follow),
                camera.FollowOffset,
                camera.Damping);
            provider.SetCameraLookAt(
                _world,
                camera.NativeHandle,
                camera.LookAt != null,
                TargetId(camera.LookAt),
                camera.LookAtDamping);
        }

        private static ulong TargetId(Transform? target)
        {
            if (target == null)
            {
                return 0UL;
            }
            GameObject? owner = target.OwnerOrNull;
            if (owner == null)
            {
                return 0UL;
            }
            return owner.NativeEntityId;
        }

        private void PushTargets(ICinemachineProvider provider)
        {
            for (int index = 0; index < _cameras.Count; index++)
            {
                CinemachineVirtualCamera camera = _cameras[index];
                PushTarget(provider, camera.Follow);
                PushTarget(provider, camera.LookAt);
            }
        }

        private void PushTarget(ICinemachineProvider provider, Transform? target)
        {
            if (target == null)
            {
                return;
            }
            provider.SetTargetPosition(_world, TargetId(target), target.position);
        }

        private void ReadState(ICinemachineProvider provider)
        {
            if (!provider.TryReadState(_world, out CinemachineBrainState state))
            {
                return;
            }
            ActiveCamera = state.HasLiveCamera ? FindCamera(state.LiveCameraName) : null;
            if (state.HasLiveCamera)
            {
                ActiveCameraPosition = state.LiveCameraPosition;
                ActiveCameraRotation = state.LiveCameraRotation;
            }
            if (state.HasBlend)
            {
                ActiveBlendFrom = state.BlendFrom;
                ActiveBlendTo = state.BlendTo;
                ActiveBlendWeight = state.BlendWeight;
            }
            else
            {
                ActiveBlendFrom = null;
                ActiveBlendTo = null;
                ActiveBlendWeight = 0f;
            }
        }

        private CinemachineVirtualCamera? FindCamera(string name)
        {
            for (int index = 0; index < _cameras.Count; index++)
            {
                if (_cameras[index].Name == name)
                {
                    return _cameras[index];
                }
            }
            return null;
        }

        private CinemachineBlendDefinition _defaultBlend =
            CinemachineBlendDefinition.EaseInOut(2f);
    }
}
