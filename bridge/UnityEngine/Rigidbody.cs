using System;

namespace UnityEngine
{
    public enum RigidbodyInterpolation
    {
        None = 0,
        Interpolate = 1,
        Extrapolate = 2,
    }

    [Flags]
    public enum RigidbodyConstraints
    {
        None = 0,
        FreezePositionX = 2,
        FreezePositionY = 4,
        FreezePositionZ = 8,
        FreezeRotationX = 16,
        FreezeRotationY = 32,
        FreezeRotationZ = 64,
        FreezePosition = FreezePositionX | FreezePositionY | FreezePositionZ,
        FreezeRotation = FreezeRotationX | FreezeRotationY | FreezeRotationZ,
        FreezeAll = FreezePosition | FreezeRotation,
    }

    public interface IPhysicsRigidbodyDepthProvider
    {
        bool TryGetBodyAngularVelocity(ulong entity, out Vector3 angularVelocity);
        bool TrySetBodyAngularVelocity(ulong entity, Vector3 angularVelocity);
        bool TryAddBodyTorque(ulong entity, Vector3 torque, ForceMode mode);
        bool TryAddBodyRelativeForce(ulong entity, Vector3 force, ForceMode mode);
        bool TryAddBodyRelativeTorque(ulong entity, Vector3 torque, ForceMode mode);
        bool TryAddBodyForceAtPosition(ulong entity, Vector3 force, Vector3 position, ForceMode mode);
        bool TryAddBodyExplosionForce(
            ulong entity,
            float explosionForce,
            Vector3 explosionPosition,
            float explosionRadius,
            float upwardsModifier,
            ForceMode mode);
        bool TrySleepBody(ulong entity);
        bool TryWakeUpBody(ulong entity);
        bool TryIsBodySleeping(ulong entity, out bool sleeping);
        bool TrySetBodyInterpolation(ulong entity, RigidbodyInterpolation mode);
        bool TrySetBodyConstraints(ulong entity, RigidbodyConstraints constraints);
    }

    public sealed class Rigidbody : Component
    {
        private Vector3 _velocity = Vector3.zero;
        private Vector3 _angularVelocity = Vector3.zero;
        private float _mass = 1f;
        private float _drag;
        private float _angularDrag = 0.05f;
        private bool _useGravity = true;
        private bool _isKinematic;
        private bool _isSleeping;
        private CollisionDetectionMode _collisionDetectionMode = CollisionDetectionMode.Discrete;
        private RigidbodyInterpolation _interpolation = RigidbodyInterpolation.None;
        private RigidbodyConstraints _constraints = RigidbodyConstraints.None;

        public Vector3 velocity
        {
            get
            {
                if (_isKinematic)
                {
                    return Vector3.zero;
                }

                ulong entity = NativeEntityId;
                IPhysicsProvider? provider = Physics.Provider;
                if (entity != 0 && provider != null && provider.TryGetBodyVelocity(entity, out Vector3 nativeVelocity))
                {
                    _velocity = MaskLinear(nativeVelocity);
                }

                return _velocity;
            }
            set
            {
                if (_isKinematic)
                {
                    _velocity = Vector3.zero;
                    return;
                }

                _velocity = MaskLinear(value);
                _isSleeping = false;
                ulong entity = NativeEntityId;
                IPhysicsProvider? provider = Physics.Provider;
                if (entity != 0 && provider != null)
                {
                    provider.TrySetBodyVelocity(entity, _velocity);
                }
            }
        }

        public Vector3 angularVelocity
        {
            get
            {
                if (_isKinematic)
                {
                    return Vector3.zero;
                }

                ulong entity = NativeEntityId;
                if (entity != 0
                    && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                    && depthProvider.TryGetBodyAngularVelocity(entity, out Vector3 nativeAngularVelocity))
                {
                    _angularVelocity = MaskAngular(nativeAngularVelocity);
                }

                return _angularVelocity;
            }
            set
            {
                if (_isKinematic)
                {
                    _angularVelocity = Vector3.zero;
                    return;
                }

                _angularVelocity = MaskAngular(value);
                _isSleeping = false;
                ulong entity = NativeEntityId;
                if (entity != 0 && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider)
                {
                    depthProvider.TrySetBodyAngularVelocity(entity, _angularVelocity);
                }
            }
        }

        public float mass
        {
            get => _mass;
            set => _mass = value > 0f ? value : 0.0001f;
        }

        public float drag
        {
            get => _drag;
            set => _drag = value >= 0f ? value : 0f;
        }

        public float angularDrag
        {
            get => _angularDrag;
            set => _angularDrag = value >= 0f ? value : 0f;
        }

        public bool useGravity
        {
            get => _useGravity;
            set => _useGravity = value;
        }

        public bool isKinematic
        {
            get => _isKinematic;
            set
            {
                _isKinematic = value;
                if (!value)
                {
                    return;
                }

                _velocity = Vector3.zero;
                _angularVelocity = Vector3.zero;
                ulong entity = NativeEntityId;
                if (entity == 0)
                {
                    return;
                }

                Physics.Provider?.TrySetBodyVelocity(entity, Vector3.zero);
                if (Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider)
                {
                    depthProvider.TrySetBodyAngularVelocity(entity, Vector3.zero);
                }
            }
        }

        public CollisionDetectionMode collisionDetectionMode
        {
            get => _collisionDetectionMode;
            set
            {
                _collisionDetectionMode = value;
                ulong entity = NativeEntityId;
                if (entity != 0)
                {
                    Physics.Provider?.TrySetBodyCollisionDetectionMode(entity, value);
                }
            }
        }

        public RigidbodyInterpolation interpolation
        {
            get => _interpolation;
            set
            {
                _interpolation = value;
                ulong entity = NativeEntityId;
                if (entity != 0 && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider)
                {
                    depthProvider.TrySetBodyInterpolation(entity, value);
                }
            }
        }

        public RigidbodyConstraints constraints
        {
            get => _constraints;
            set
            {
                _constraints = value;
                _velocity = MaskLinear(_velocity);
                _angularVelocity = MaskAngular(_angularVelocity);
                ulong entity = NativeEntityId;
                if (entity != 0 && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider)
                {
                    depthProvider.TrySetBodyConstraints(entity, value);
                }
            }
        }

        public Vector3 position
        {
            get => transform.position;
            set => MovePosition(value);
        }

        public Quaternion rotation
        {
            get => transform.rotation;
            set => MoveRotation(value);
        }

        public void AddForce(Vector3 force) => AddForce(force, ForceMode.Force);

        public void AddForce(float x, float y, float z, ForceMode mode = ForceMode.Force) =>
            AddForce(new Vector3(x, y, z), mode);

        public void AddForce(Vector3 force, ForceMode mode)
        {
            if (_isKinematic)
            {
                return;
            }

            _isSleeping = false;
            Vector3 maskedForce = MaskLinear(force);
            ulong entity = NativeEntityId;
            IPhysicsProvider? provider = Physics.Provider;
            if (entity != 0 && provider != null && provider.TryAddForce(entity, maskedForce, mode))
            {
                SyncNativeLinearVelocity(entity);
                return;
            }

            _velocity = MaskLinear(_velocity + ComputeDelta(maskedForce, mode));
        }

        public void AddTorque(Vector3 torque, ForceMode mode = ForceMode.Force)
        {
            if (_isKinematic)
            {
                return;
            }

            _isSleeping = false;
            Vector3 maskedTorque = MaskAngular(torque);
            ulong entity = NativeEntityId;
            if (entity != 0
                && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                && depthProvider.TryAddBodyTorque(entity, maskedTorque, mode))
            {
                SyncNativeAngularVelocity(entity, depthProvider);
                return;
            }

            _angularVelocity = MaskAngular(_angularVelocity + ComputeDelta(maskedTorque, mode));
        }

        public void AddTorque(float x, float y, float z, ForceMode mode = ForceMode.Force) =>
            AddTorque(new Vector3(x, y, z), mode);

        public void AddRelativeForce(Vector3 force, ForceMode mode = ForceMode.Force)
        {
            if (_isKinematic)
            {
                return;
            }

            _isSleeping = false;
            ulong entity = NativeEntityId;
            if (entity != 0
                && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                && depthProvider.TryAddBodyRelativeForce(entity, force, mode))
            {
                SyncNativeLinearVelocity(entity);
                return;
            }

            AddForce(transform.rotation * force, mode);
        }

        public void AddRelativeForce(float x, float y, float z, ForceMode mode = ForceMode.Force) =>
            AddRelativeForce(new Vector3(x, y, z), mode);

        public void AddRelativeTorque(Vector3 torque, ForceMode mode = ForceMode.Force)
        {
            if (_isKinematic)
            {
                return;
            }

            _isSleeping = false;
            ulong entity = NativeEntityId;
            if (entity != 0
                && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                && depthProvider.TryAddBodyRelativeTorque(entity, torque, mode))
            {
                SyncNativeAngularVelocity(entity, depthProvider);
                return;
            }

            AddTorque(transform.rotation * torque, mode);
        }

        public void AddRelativeTorque(float x, float y, float z, ForceMode mode = ForceMode.Force) =>
            AddRelativeTorque(new Vector3(x, y, z), mode);

        public void AddForceAtPosition(Vector3 force, Vector3 position, ForceMode mode = ForceMode.Force)
        {
            if (_isKinematic)
            {
                return;
            }

            _isSleeping = false;
            ulong entity = NativeEntityId;
            if (entity != 0
                && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                && depthProvider.TryAddBodyForceAtPosition(entity, force, position, mode))
            {
                SyncNativeLinearVelocity(entity);
                SyncNativeAngularVelocity(entity, depthProvider);
                return;
            }

            AddForce(force, mode);
            AddTorque(Vector3.Cross(position - transform.position, force), mode);
        }

        public void AddExplosionForce(
            float explosionForce,
            Vector3 explosionPosition,
            float explosionRadius,
            float upwardsModifier = 0.0f,
            ForceMode mode = ForceMode.Force)
        {
            if (_isKinematic)
            {
                return;
            }

            _isSleeping = false;
            ulong entity = NativeEntityId;
            if (entity != 0
                && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                && depthProvider.TryAddBodyExplosionForce(
                    entity,
                    explosionForce,
                    explosionPosition,
                    explosionRadius,
                    upwardsModifier,
                    mode))
            {
                SyncNativeLinearVelocity(entity);
                return;
            }

            Vector3 center = upwardsModifier == 0f
                ? explosionPosition
                : new Vector3(explosionPosition.x, explosionPosition.y - upwardsModifier, explosionPosition.z);
            Vector3 dir = transform.position - center;
            float distance = dir.magnitude;
            if (explosionRadius > 0f && distance > explosionRadius)
            {
                return;
            }

            float attenuation = explosionRadius <= 0f ? 1f : Mathf.Clamp01(1f - (distance / explosionRadius));
            Vector3 unitDir = distance > 1e-6f ? (dir / distance) : Vector3.up;
            AddForce(unitDir * (explosionForce * attenuation), mode);
        }

        public void Sleep()
        {
            _isSleeping = true;
            _velocity = Vector3.zero;
            _angularVelocity = Vector3.zero;
            ulong entity = NativeEntityId;
            if (entity != 0 && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider)
            {
                depthProvider.TrySleepBody(entity);
            }
        }

        public void WakeUp()
        {
            _isSleeping = false;
            ulong entity = NativeEntityId;
            if (entity != 0 && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider)
            {
                depthProvider.TryWakeUpBody(entity);
            }
        }

        public bool IsSleeping()
        {
            ulong entity = NativeEntityId;
            if (entity != 0
                && Physics.Provider is IPhysicsRigidbodyDepthProvider depthProvider
                && depthProvider.TryIsBodySleeping(entity, out bool nativeSleeping))
            {
                _isSleeping = nativeSleeping;
            }

            return _isSleeping;
        }

        public void MovePosition(Vector3 position)
        {
            ulong entity = NativeEntityId;
            if (entity != 0)
            {
                Physics.Provider?.TrySetBodyPosition(entity, position);
            }

            transform.SeedPositionFromNative(position);
        }

        public void MoveRotation(Quaternion rot)
        {
            ulong entity = NativeEntityId;
            if (entity != 0)
            {
                Physics.Provider?.TrySetBodyRotation(entity, rot);
            }

            transform.rotation = rot;
        }

        private void SyncNativeLinearVelocity(ulong entity)
        {
            if (Physics.Provider != null && Physics.Provider.TryGetBodyVelocity(entity, out Vector3 nativeVelocity))
            {
                _velocity = MaskLinear(nativeVelocity);
            }
        }

        private void SyncNativeAngularVelocity(ulong entity, IPhysicsRigidbodyDepthProvider depthProvider)
        {
            if (depthProvider.TryGetBodyAngularVelocity(entity, out Vector3 nativeAngularVelocity))
            {
                _angularVelocity = MaskAngular(nativeAngularVelocity);
            }
        }

        private Vector3 ComputeDelta(Vector3 input, ForceMode mode)
        {
            float dt = Time.fixedDeltaTime > 0f ? Time.fixedDeltaTime : 0.02f;
            float invMass = _mass > 0f ? 1f / _mass : 0f;
            switch (mode)
            {
                case ForceMode.Force:
                    return input * (invMass * dt);
                case ForceMode.Impulse:
                    return input * invMass;
                case ForceMode.Acceleration:
                    return input * dt;
                case ForceMode.VelocityChange:
                    return input;
                default:
                    return Vector3.zero;
            }
        }

        private Vector3 MaskLinear(Vector3 v) =>
            new Vector3(
                (_constraints & RigidbodyConstraints.FreezePositionX) != 0 ? 0f : v.x,
                (_constraints & RigidbodyConstraints.FreezePositionY) != 0 ? 0f : v.y,
                (_constraints & RigidbodyConstraints.FreezePositionZ) != 0 ? 0f : v.z);

        private Vector3 MaskAngular(Vector3 v) =>
            new Vector3(
                (_constraints & RigidbodyConstraints.FreezeRotationX) != 0 ? 0f : v.x,
                (_constraints & RigidbodyConstraints.FreezeRotationY) != 0 ? 0f : v.y,
                (_constraints & RigidbodyConstraints.FreezeRotationZ) != 0 ? 0f : v.z);

        private ulong NativeEntityId => gameObject != null ? gameObject.NativeEntityId : 0;
    }
}
