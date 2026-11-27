namespace UnityEngine
{
    public struct JointMotor2D
    {
        private float _maxMotorTorque;
        private bool _hasMaxTorque;

        public float motorSpeed { get; set; }

        public float maxMotorTorque
        {
            get => _hasMaxTorque ? _maxMotorTorque : 10000f;
            set
            {
                _maxMotorTorque = value;
                _hasMaxTorque = true;
            }
        }
    }

    public struct JointAngleLimits2D
    {
        public float min { get; set; }
        public float max { get; set; }
    }

    public struct JointTranslationLimits2D
    {
        public float min { get; set; }
        public float max { get; set; }
    }

    public struct JointSuspension2D
    {
        private float _dampingRatio;
        private bool _hasDampingRatio;
        private float _frequency;
        private bool _hasFrequency;
        private float _angle;
        private bool _hasAngle;

        public float dampingRatio
        {
            get => _hasDampingRatio ? _dampingRatio : 0.7f;
            set
            {
                _dampingRatio = value;
                _hasDampingRatio = true;
            }
        }

        public float frequency
        {
            get => _hasFrequency ? _frequency : 2f;
            set
            {
                _frequency = value;
                _hasFrequency = true;
            }
        }

        public float angle
        {
            get => _hasAngle ? _angle : 90f;
            set
            {
                _angle = value;
                _hasAngle = true;
            }
        }
    }

    public interface IPhysicsJoint2DProvider
    {
        bool TrySyncJoint2D(Joint2D joint);
        bool TryRemoveJoint2D(ulong entity, uint kind);
        bool TryGetBodyPose2D(ulong entity, out Vector2 position, out float rotationRadians);
    }

    public class Joint2D : Behaviour
    {
        private Rigidbody2D? _connectedBody;
        private float _breakForce = float.PositiveInfinity;
        private float _breakTorque = float.PositiveInfinity;
        private bool _enableCollision;

        static Joint2D()
        {
            UnityEngine.SceneManagement.SceneManager.sceneLoaded += (_, _) =>
            {
                if (Physics.Provider is IPhysicsJointProvider provider)
                {
                    provider.EnsureLoadedSceneJointsMaterialized();
                }
            };
        }

        public Rigidbody2D? connectedBody
        {
            get => _connectedBody;
            set
            {
                _connectedBody = value;
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

        internal virtual uint NativeJointKind => 0;

        internal void SyncToNative()
        {
            GameObject? owner = OwnerOrNull;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }
            if (Physics2D.Provider is IPhysicsJoint2DProvider provider)
            {
                provider.TrySyncJoint2D(this);
            }
        }
    }

    public class AnchoredJoint2D : Joint2D
    {
        private Vector2 _anchor = Vector2.zero;
        private Vector2 _connectedAnchor = Vector2.zero;
        private bool _autoConfigureConnectedAnchor = true;

        public Vector2 anchor
        {
            get => _anchor;
            set
            {
                _anchor = value;
                SyncToNative();
            }
        }

        public Vector2 connectedAnchor
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
    }

    public sealed class DistanceJoint2D : AnchoredJoint2D
    {
        private bool _autoConfigureDistance = true;
        private float _distance = 1f;
        private bool _maxDistanceOnly;

        internal override uint NativeJointKind => 0;

        public bool autoConfigureDistance
        {
            get => _autoConfigureDistance;
            set
            {
                _autoConfigureDistance = value;
                SyncToNative();
            }
        }

        public float distance
        {
            get => _distance;
            set
            {
                _distance = value;
                SyncToNative();
            }
        }

        public bool maxDistanceOnly
        {
            get => _maxDistanceOnly;
            set
            {
                _maxDistanceOnly = value;
                SyncToNative();
            }
        }
    }

    public sealed class FixedJoint2D : AnchoredJoint2D
    {
        private float _dampingRatio;
        private float _frequency = 2f;
        private float _referenceAngle;

        internal override uint NativeJointKind => 1;

        public float dampingRatio
        {
            get => _dampingRatio;
            set
            {
                _dampingRatio = value;
                SyncToNative();
            }
        }

        public float frequency
        {
            get => _frequency;
            set
            {
                _frequency = value;
                SyncToNative();
            }
        }

        public float referenceAngle
        {
            get => _referenceAngle;
            set
            {
                _referenceAngle = value;
                SyncToNative();
            }
        }
    }

    public sealed class FrictionJoint2D : AnchoredJoint2D
    {
        private float _maxForce = 1f;
        private float _maxTorque = 1f;

        internal override uint NativeJointKind => 2;

        public float maxForce
        {
            get => _maxForce;
            set
            {
                _maxForce = value;
                SyncToNative();
            }
        }

        public float maxTorque
        {
            get => _maxTorque;
            set
            {
                _maxTorque = value;
                SyncToNative();
            }
        }
    }

    public sealed class HingeJoint2D : AnchoredJoint2D
    {
        private bool _useMotor;
        private JointMotor2D _motor;
        private bool _useLimits;
        private JointAngleLimits2D _limits;

        internal override uint NativeJointKind => 3;

        public bool useMotor
        {
            get => _useMotor;
            set
            {
                _useMotor = value;
                SyncToNative();
            }
        }

        public JointMotor2D motor
        {
            get => _motor;
            set
            {
                _motor = value;
                SyncToNative();
            }
        }

        public bool useLimits
        {
            get => _useLimits;
            set
            {
                _useLimits = value;
                SyncToNative();
            }
        }

        public JointAngleLimits2D limits
        {
            get => _limits;
            set
            {
                _limits = value;
                SyncToNative();
            }
        }
    }
}
