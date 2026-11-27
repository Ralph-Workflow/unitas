using System;

namespace UnityEngine
{
    public enum ConfigurableJointMotion
    {
        Locked = 0,
        Limited = 1,
        Free = 2,
    }

    public struct JointLimits
    {
        public float min { get; set; }
        public float max { get; set; }
        public float bounciness { get; set; }
        public float bounceMinVelocity { get; set; }
        public float contactDistance { get; set; }
    }

    public struct JointSpring
    {
        public float spring { get; set; }
        public float damper { get; set; }
        public float targetPosition { get; set; }
    }

    public struct JointMotor
    {
        public float targetVelocity { get; set; }
        public float force { get; set; }
        public bool freeSpin { get; set; }
    }

    public struct JointDrive
    {
        private float _maximumForce;
        private bool _hasMaximumForce;

        public float positionSpring { get; set; }
        public float positionDamper { get; set; }

        public float maximumForce
        {
            get => _hasMaximumForce ? _maximumForce : float.MaxValue;
            set
            {
                _maximumForce = value;
                _hasMaximumForce = true;
            }
        }
    }

    public struct SoftJointLimit
    {
        public float limit { get; set; }
        public float bounciness { get; set; }
        public float contactDistance { get; set; }
    }

    public interface IPhysicsJointProvider
    {
        bool TrySyncJoint3D(Joint joint);
        bool TryRemoveJoint3D(ulong entity, uint kind);
        void EnsureLoadedSceneJointsMaterialized();
        void SyncJointTransformsAfterStep();
        bool TryGetBodyRotation3D(ulong entity, out Quaternion rotation);
    }

    public class Joint : Component
    {
        private Rigidbody? _connectedBody;
        private Vector3 _axis = Vector3.right;
        private Vector3 _anchor = Vector3.zero;
        private Vector3 _connectedAnchor = Vector3.zero;
        private bool _autoConfigureConnectedAnchor = true;
        private float _breakForce = float.PositiveInfinity;
        private float _breakTorque = float.PositiveInfinity;
        private bool _enableCollision;
        private bool _enablePreprocessing = true;
        private float _massScale = 1f;
        private float _connectedMassScale = 1f;

        static Joint()
        {
            UnityEngine.SceneManagement.SceneManager.sceneLoaded += (_, _) =>
            {
                if (Physics.Provider is IPhysicsJointProvider provider)
                {
                    provider.EnsureLoadedSceneJointsMaterialized();
                }
            };
        }

        public Rigidbody? connectedBody
        {
            get => _connectedBody;
            set
            {
                _connectedBody = value;
                SyncToNative();
            }
        }

        public Vector3 axis
        {
            get => _axis;
            set
            {
                _axis = value;
                SyncToNative();
            }
        }

        public Vector3 anchor
        {
            get => _anchor;
            set
            {
                _anchor = value;
                SyncToNative();
            }
        }

        public Vector3 connectedAnchor
        {
            get => _connectedAnchor;
            set
            {
                _connectedAnchor = value;
                SyncToNative();
            }
        }

        public bool autoConfigureConnectedAnchor
        {
            get => _autoConfigureConnectedAnchor;
            set
            {
                _autoConfigureConnectedAnchor = value;
                SyncToNative();
            }
        }

        public float breakForce
        {
            get => _breakForce;
            set
            {
                _breakForce = value;
                SyncToNative();
            }
        }

        public float breakTorque
        {
            get => _breakTorque;
            set
            {
                _breakTorque = value;
                SyncToNative();
            }
        }

        public bool enableCollision
        {
            get => _enableCollision;
            set
            {
                _enableCollision = value;
                SyncToNative();
            }
        }

        public bool enablePreprocessing
        {
            get => _enablePreprocessing;
            set
            {
                _enablePreprocessing = value;
                SyncToNative();
            }
        }

        public float massScale
        {
            get => _massScale;
            set
            {
                _massScale = value;
                SyncToNative();
            }
        }

        public float connectedMassScale
        {
            get => _connectedMassScale;
            set
            {
                _connectedMassScale = value;
                SyncToNative();
            }
        }

        internal virtual uint NativeJointKind => 0;

        internal void SyncToNative()
        {
            GameObject? owner = OwnerOrNull;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }
            if (Physics.Provider is IPhysicsJointProvider provider)
            {
                provider.TrySyncJoint3D(this);
            }
        }
    }

    public sealed class FixedJoint : Joint
    {
        internal override uint NativeJointKind => 0;
    }

    public sealed class HingeJoint : Joint
    {
        private bool _useLimits;
        private JointLimits _limits;
        private bool _useMotor;
        private JointMotor _motor;
        private bool _useSpring;
        private JointSpring _spring;

        internal override uint NativeJointKind => 1;

        public bool useLimits
        {
            get => _useLimits;
            set
            {
                _useLimits = value;
                SyncToNative();
            }
        }

        public JointLimits limits
        {
            get => _limits;
            set
            {
                _limits = value;
                SyncToNative();
            }
        }

        public bool useMotor
        {
            get => _useMotor;
            set
            {
                _useMotor = value;
                SyncToNative();
            }
        }

        public JointMotor motor
        {
            get => _motor;
            set
            {
                _motor = value;
                SyncToNative();
            }
        }

        public bool useSpring
        {
            get => _useSpring;
            set
            {
                _useSpring = value;
                SyncToNative();
            }
        }

        public JointSpring spring
        {
            get => _spring;
            set
            {
                _spring = value;
                SyncToNative();
            }
        }

        public float angle => 0f;

        public float velocity => 0f;
    }

    public sealed class SpringJoint : Joint
    {
        private float _spring = 10f;
        private float _damper = 0.2f;
        private float _minDistance;
        private float _maxDistance;
        private float _tolerance = 0.025f;

        internal override uint NativeJointKind => 2;

        public float spring
        {
            get => _spring;
            set
            {
                _spring = value;
                SyncToNative();
            }
        }

        public float damper
        {
            get => _damper;
            set
            {
                _damper = value;
                SyncToNative();
            }
        }

        public float minDistance
        {
            get => _minDistance;
            set
            {
                _minDistance = value;
                SyncToNative();
            }
        }

        public float maxDistance
        {
            get => _maxDistance;
            set
            {
                _maxDistance = value;
                SyncToNative();
            }
        }

        public float tolerance
        {
            get => _tolerance;
            set
            {
                _tolerance = value;
                SyncToNative();
            }
        }
    }

    public sealed class CharacterJoint : Joint
    {
        private Vector3 _swingAxis = Vector3.up;
        private SoftJointLimit _lowTwistLimit = new SoftJointLimit { limit = -20f };
        private SoftJointLimit _highTwistLimit = new SoftJointLimit { limit = 70f };
        private SoftJointLimit _swing1Limit = new SoftJointLimit { limit = 40f };
        private SoftJointLimit _swing2Limit = new SoftJointLimit { limit = 40f };

        internal override uint NativeJointKind => 3;

        public Vector3 swingAxis
        {
            get => _swingAxis;
            set
            {
                _swingAxis = value;
                SyncToNative();
            }
        }

        public SoftJointLimit lowTwistLimit
        {
            get => _lowTwistLimit;
            set
            {
                _lowTwistLimit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit highTwistLimit
        {
            get => _highTwistLimit;
            set
            {
                _highTwistLimit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit swing1Limit
        {
            get => _swing1Limit;
            set
            {
                _swing1Limit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit swing2Limit
        {
            get => _swing2Limit;
            set
            {
                _swing2Limit = value;
                SyncToNative();
            }
        }
    }
}
