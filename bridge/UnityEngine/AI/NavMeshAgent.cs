using System;

namespace UnityEngine.AI
{
    public sealed class NavMeshAgent : Behaviour
    {
        private Vector3 _destination;
        private bool _hasDestination;
        private bool _isStopped;
        private float _speed = 3.5f;
        private float _acceleration = 8f;
        private float _angularSpeed = 120f;
        private float _stoppingDistance;
        private float _radius = 0.5f;
        private float _height = 2f;
        private NavMeshPathStatus _pathStatus = NavMeshPathStatus.PathComplete;
        private Vector3 _velocity = Vector3.zero;
        private Vector3 _nextPosition;
        private bool _hasNextPosition;

        public Vector3 destination
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetDestination(entity, out Vector3 nativeDest))
                {
                    SyncNativePositionToTransform(entity);
                    return nativeDest;
                }
                return _hasDestination ? _destination : transform.position;
            }
            set => SetDestination(value);
        }

        public bool isStopped
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetStopped(entity, out bool nativeStopped))
                {
                    return nativeStopped;
                }
                return _isStopped;
            }
            set
            {
                _isStopped = value;
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetStopped(entity, value);
                }
            }
        }

        public float speed
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetSpeed(entity, out float nativeSpeed))
                {
                    return nativeSpeed;
                }
                return _speed;
            }
            set
            {
                _speed = Math.Max(0f, value);
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetSpeed(entity, _speed);
                }
            }
        }

        public float acceleration
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetAcceleration(entity, out float nativeAcc))
                {
                    return nativeAcc;
                }
                return _acceleration;
            }
            set
            {
                _acceleration = Math.Max(0f, value);
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetAcceleration(entity, _acceleration);
                }
            }
        }

        public float angularSpeed
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetAngularSpeed(entity, out float nativeAngular))
                {
                    return nativeAngular;
                }
                return _angularSpeed;
            }
            set
            {
                _angularSpeed = Math.Max(0f, value);
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetAngularSpeed(entity, _angularSpeed);
                }
            }
        }

        public float stoppingDistance
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetStoppingDistance(entity, out float nativeDist))
                {
                    return nativeDist;
                }
                return _stoppingDistance;
            }
            set
            {
                _stoppingDistance = Math.Max(0f, value);
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetStoppingDistance(entity, _stoppingDistance);
                }
            }
        }

        public float radius
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetRadius(entity, out float nativeRadius))
                {
                    return nativeRadius;
                }
                return _radius;
            }
            set
            {
                _radius = Math.Max(0.001f, value);
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetRadius(entity, _radius);
                }
            }
        }

        public float height
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetHeight(entity, out float nativeHeight))
                {
                    return nativeHeight;
                }
                return _height;
            }
            set
            {
                _height = Math.Max(0.001f, value);
                if (TryEnsureNativeEntity(out ulong entity))
                {
                    NavMesh.Provider!.SetHeight(entity, _height);
                }
            }
        }

        public float remainingDistance
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetRemainingDistance(entity, out float nativeRem))
                {
                    SyncNativePositionToTransform(entity);
                    return nativeRem;
                }
                return 0f;
            }
        }

        public NavMeshPathStatus pathStatus
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetPathStatus(entity, out NavMeshPathStatus nativeStatus))
                {
                    SyncNativePositionToTransform(entity);
                    return nativeStatus;
                }
                return _pathStatus;
            }
        }

        public bool isOnNavMesh
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetIsOnNavMesh(entity, out bool onMesh))
                {
                    SyncNativePositionToTransform(entity);
                    return onMesh;
                }
                return false;
            }
        }

        public bool hasPath
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetHasPath(entity, out bool nativeHasPath))
                {
                    SyncNativePositionToTransform(entity);
                    return nativeHasPath;
                }
                return false;
            }
        }

        public bool pathPending => false;

        public Vector3 velocity
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetVelocity(entity, out Vector3 nativeVel))
                {
                    SyncNativePositionToTransform(entity);
                    return nativeVel;
                }
                return _velocity;
            }
            set => _velocity = value;
        }

        public Vector3 nextPosition
        {
            get
            {
                if (TryEnsureNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetPosition(entity, out Vector3 nativePos))
                {
                    SyncNativePositionToTransform(entity);
                    return nativePos;
                }
                return _hasNextPosition ? _nextPosition : transform.position;
            }
            set
            {
                _nextPosition = value;
                _hasNextPosition = true;
                Warp(value);
            }
        }

        public bool SetDestination(Vector3 target)
        {
            _destination = target;
            _hasDestination = true;
            if (!TryEnsureNativeEntity(out ulong entity))
            {
                _pathStatus = NavMeshPathStatus.PathInvalid;
                return false;
            }
            bool ok = NavMesh.Provider!.SetDestination(entity, target, out NavMeshPathStatus status);
            _pathStatus = status;
            SyncNativePositionToTransform(entity);
            return ok;
        }

        public bool Warp(Vector3 newPosition)
        {
            _nextPosition = newPosition;
            _hasNextPosition = true;
            transform.SeedPositionFromNative(newPosition);
            if (!TryEnsureNativeEntity(out ulong entity))
            {
                return false;
            }
            return NavMesh.Provider!.Warp(entity, newPosition);
        }

        public void ResetPath()
        {
            _hasDestination = false;
            _pathStatus = NavMeshPathStatus.PathComplete;
            _velocity = Vector3.zero;
            if (TryEnsureNativeEntity(out ulong entity))
            {
                NavMesh.Provider!.ResetPath(entity);
            }
        }

        public void Move(Vector3 offset)
        {
            Vector3 next = transform.position + offset;
            _nextPosition = next;
            _hasNextPosition = true;
            if (TryEnsureNativeEntity(out ulong entity) && NavMesh.Provider!.Move(entity, offset))
            {
                SyncNativePositionToTransform(entity);
                return;
            }
            transform.SeedPositionFromNative(next);
        }

        private void SyncNativePositionToTransform(ulong entity)
        {
            INavigationProvider? provider = NavMesh.Provider;
            if (provider != null && provider.TryGetPosition(entity, out Vector3 pos))
            {
                GameObject? owner = gameObject;
                if (owner != null && !owner.IsDestroyed)
                {
                    owner.transform.SeedPositionFromNative(pos);
                }
            }
        }

        private bool TryEnsureNativeEntity(out ulong entity)
        {
            entity = 0UL;
            INavigationProvider? provider = NavMesh.Provider;
            if (provider == null)
            {
                return false;
            }
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return false;
            }
            entity = owner.NativeEntityId;
            if (entity == 0UL)
            {
                return false;
            }
            if (!provider.HasAgent(entity))
            {
                provider.EnsureAgent(entity);
                provider.Warp(entity, owner.transform.position);
            }
            return true;
        }
    }
}
