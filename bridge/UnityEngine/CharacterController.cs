using System;

namespace UnityEngine
{
    [Flags]
    public enum CollisionFlags
    {
        None = 0,
        Sides = 1,
        Above = 2,
        Below = 4,
        CollidedSides = Sides,
        CollidedAbove = Above,
        CollidedBelow = Below,
    }

    public readonly struct CharacterMoveInput
    {
        public CharacterMoveInput(
            ulong entity,
            Vector3 position,
            Vector3 motion,
            float radius,
            float height,
            Vector3 center,
            float slopeLimit,
            float stepOffset,
            float skinWidth,
            float minMoveDistance,
            float deltaSeconds,
            uint layerMask,
            bool enabled)
        {
            Entity = entity;
            Position = position;
            Motion = motion;
            Radius = radius;
            Height = height;
            Center = center;
            SlopeLimit = slopeLimit;
            StepOffset = stepOffset;
            SkinWidth = skinWidth;
            MinMoveDistance = minMoveDistance;
            DeltaSeconds = deltaSeconds;
            LayerMask = layerMask;
            Enabled = enabled;
        }

        public ulong Entity { get; }

        public Vector3 Position { get; }

        public Vector3 Motion { get; }

        public float Radius { get; }

        public float Height { get; }

        public Vector3 Center { get; }

        public float SlopeLimit { get; }

        public float StepOffset { get; }

        public float SkinWidth { get; }

        public float MinMoveDistance { get; }

        public float DeltaSeconds { get; }

        public uint LayerMask { get; }

        public bool Enabled { get; }
    }

    public readonly struct CharacterMoveOutput
    {
        public CharacterMoveOutput(Vector3 position, ulong groundEntity, CollisionFlags collisionFlags, bool isGrounded)
        {
            Position = position;
            GroundEntity = groundEntity;
            CollisionFlags = collisionFlags;
            IsGrounded = isGrounded;
        }

        public Vector3 Position { get; }

        public ulong GroundEntity { get; }

        public CollisionFlags CollisionFlags { get; }

        public bool IsGrounded { get; }
    }

    public readonly struct CharacterControllerConfig
    {
        public CharacterControllerConfig(
            float radius,
            float height,
            Vector3 center,
            float slopeLimit,
            float stepOffset,
            float skinWidth,
            float minMoveDistance,
            bool enabled)
        {
            Radius = radius;
            Height = height;
            Center = center;
            SlopeLimit = slopeLimit;
            StepOffset = stepOffset;
            SkinWidth = skinWidth;
            MinMoveDistance = minMoveDistance;
            Enabled = enabled;
        }

        public float Radius { get; }

        public float Height { get; }

        public Vector3 Center { get; }

        public float SlopeLimit { get; }

        public float StepOffset { get; }

        public float SkinWidth { get; }

        public float MinMoveDistance { get; }

        public bool Enabled { get; }
    }

    public interface ICharacterControllerProvider
    {
        bool TryMove(CharacterMoveInput input, out CharacterMoveOutput output);

        bool TryTeleport(ulong entity, Vector3 position);

        bool TryGetConfig(ulong entity, out CharacterControllerConfig config);
    }

    public sealed class CharacterController : Collider
    {
        private const float DefaultGravityY = -9.81f;
        private const float DefaultStepDt = 0.02f;

        private float _radius = 0.5f;
        private float _height = 2.0f;
        private Vector3 _center = Vector3.zero;
        private float _slopeLimit = 45.0f;
        private float _stepOffset = 0.3f;
        private float _skinWidth = 0.08f;
        private float _minMoveDistance = 0.001f;
        private bool _isGrounded;
        private CollisionFlags _collisionFlags = CollisionFlags.None;
        private Vector3 _velocity = Vector3.zero;
        private bool _updatingTransformFromMove;

        public static ICharacterControllerProvider? Provider { get; set; }

        public float radius
        {
            get => _radius;
            set => _radius = value;
        }

        public float height
        {
            get => _height;
            set => _height = value;
        }

        public Vector3 center
        {
            get => _center;
            set => _center = value;
        }

        public float slopeLimit
        {
            get => _slopeLimit;
            set => _slopeLimit = value;
        }

        public float stepOffset
        {
            get => _stepOffset;
            set => _stepOffset = value;
        }

        public float skinWidth
        {
            get => _skinWidth;
            set => _skinWidth = value;
        }

        public float minMoveDistance
        {
            get => _minMoveDistance;
            set => _minMoveDistance = value;
        }

        public bool isGrounded => _isGrounded;

        public CollisionFlags collisionFlags => _collisionFlags;

        public Vector3 velocity => _velocity;

        internal bool IsUpdatingTransformFromMove => _updatingTransformFromMove;

        public CollisionFlags Move(Vector3 motion)
        {
            GameObject? owner = gameObject;
            if (!enabled || owner == null || owner.IsDestroyed || !owner.activeInHierarchy)
            {
                _isGrounded = false;
                _collisionFlags = CollisionFlags.None;
                _velocity = Vector3.zero;
                return CollisionFlags.None;
            }

            ICharacterControllerProvider? provider = Provider;
            if (provider == null)
            {
                _isGrounded = false;
                _collisionFlags = CollisionFlags.None;
                _velocity = Vector3.zero;
                return CollisionFlags.None;
            }

            Vector3 startPosition = transform.position;
            float dt = Time.deltaTime > 0f ? Time.deltaTime : (Time.fixedDeltaTime > 0f ? Time.fixedDeltaTime : DefaultStepDt);
            CharacterMoveInput input = new CharacterMoveInput(
                owner.NativeEntityId,
                startPosition,
                motion,
                _radius,
                _height,
                _center,
                _slopeLimit,
                _stepOffset,
                _skinWidth,
                _minMoveDistance,
                dt,
                uint.MaxValue,
                enabled);

            if (!provider.TryMove(input, out CharacterMoveOutput output))
            {
                _isGrounded = false;
                _collisionFlags = CollisionFlags.None;
                _velocity = Vector3.zero;
                return CollisionFlags.None;
            }

            _updatingTransformFromMove = true;
            try
            {
                transform.position = output.Position;
            }
            finally
            {
                _updatingTransformFromMove = false;
            }

            _isGrounded = output.IsGrounded;
            _collisionFlags = output.CollisionFlags;
            _velocity = dt > 0f ? (output.Position - startPosition) * (1f / dt) : Vector3.zero;
            return _collisionFlags;
        }

        public bool SimpleMove(Vector3 speed)
        {
            float dt = Time.deltaTime > 0f ? Time.deltaTime : (Time.fixedDeltaTime > 0f ? Time.fixedDeltaTime : DefaultStepDt);
            Vector3 horizontal = new Vector3(speed.x, 0f, speed.z);
            Vector3 motion = (horizontal + new Vector3(0f, DefaultGravityY, 0f)) * dt;
            Move(motion);
            return _isGrounded;
        }

        internal void OnTransformTeleported(Vector3 newPosition)
        {
            if (_updatingTransformFromMove)
            {
                return;
            }

            _isGrounded = false;
            _collisionFlags = CollisionFlags.None;
            _velocity = Vector3.zero;
            GameObject? owner = gameObject;
            ulong entityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            Provider?.TryTeleport(entityId, newPosition);
        }
    }
}
