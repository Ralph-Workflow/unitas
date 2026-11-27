using UnityEngine;

namespace Cinemachine
{
    public sealed class CinemachineVirtualCamera
    {
        private CinemachineBrain? _brain;
        private Transform? _follow;
        private Transform? _lookAt;
        private Vector3 _followOffset;
        private float _damping;
        private float _lookAtDamping;
        private int _priority;
        private bool _active = true;

        public CinemachineVirtualCamera(string name, int priority)
        {
            Name = name ?? string.Empty;
            _priority = priority;
        }

        public string Name { get; }

        public int Priority
        {
            get => _priority;
            set
            {
                _priority = value;
                Push();
            }
        }

        public Transform? Follow
        {
            get => _follow;
            set
            {
                _follow = value;
                Push();
            }
        }

        public Transform? LookAt
        {
            get => _lookAt;
            set
            {
                _lookAt = value;
                Push();
            }
        }

        public Vector3 FollowOffset
        {
            get => _followOffset;
            set
            {
                _followOffset = value;
                Push();
            }
        }

        public float Damping
        {
            get => _damping;
            set
            {
                _damping = value;
                Push();
            }
        }

        public float LookAtDamping
        {
            get => _lookAtDamping;
            set
            {
                _lookAtDamping = value;
                Push();
            }
        }

        public bool Active
        {
            get => _active;
            set
            {
                _active = value;
                Push();
            }
        }

        internal ulong NativeHandle { get; private set; }

        internal void Attach(CinemachineBrain brain, ulong handle)
        {
            _brain = brain;
            NativeHandle = handle;
            Push();
        }

        internal void Push()
        {
            _brain?.Push(this);
        }
    }
}
