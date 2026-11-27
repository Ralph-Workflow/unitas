namespace UnityEngine
{
    public sealed class RelativeJoint2D : Joint2D
    {
        private float _maxForce = 10000f;
        private float _maxTorque = 10000f;
        private float _correctionScale = 0.3f;
        private bool _autoConfigureOffset = true;
        private Vector2 _linearOffset = Vector2.zero;
        private float _angularOffset;

        internal override uint NativeJointKind => 4;

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

        public float correctionScale
        {
            get => _correctionScale;
            set
            {
                _correctionScale = value;
                SyncToNative();
            }
        }

        public bool autoConfigureOffset
        {
            get => _autoConfigureOffset;
            set
            {
                _autoConfigureOffset = value;
                SyncToNative();
            }
        }

        public Vector2 linearOffset
        {
            get => _linearOffset;
            set
            {
                _linearOffset = value;
                SyncToNative();
            }
        }

        public float angularOffset
        {
            get => _angularOffset;
            set
            {
                _angularOffset = value;
                SyncToNative();
            }
        }
    }

    public sealed class SliderJoint2D : AnchoredJoint2D
    {
        private bool _autoConfigureAngle = true;
        private float _angle;
        private bool _useMotor;
        private JointMotor2D _motor;
        private bool _useLimits;
        private JointTranslationLimits2D _limits;

        internal override uint NativeJointKind => 5;

        public bool autoConfigureAngle
        {
            get => _autoConfigureAngle;
            set
            {
                _autoConfigureAngle = value;
                SyncToNative();
            }
        }

        public float angle
        {
            get => _angle;
            set
            {
                _angle = value;
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

        public JointTranslationLimits2D limits
        {
            get => _limits;
            set
            {
                _limits = value;
                SyncToNative();
            }
        }
    }

    public sealed class SpringJoint2D : AnchoredJoint2D
    {
        private bool _autoConfigureDistance = true;
        private float _distance = 1f;
        private float _dampingRatio;
        private float _frequency = 2f;

        internal override uint NativeJointKind => 6;

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
    }

    public sealed class TargetJoint2D : Joint2D
    {
        private Vector2 _anchor = Vector2.zero;
        private Vector2 _target = Vector2.zero;
        private bool _autoConfigureTarget = true;
        private float _maxForce = 10000f;
        private float _dampingRatio = 1f;
        private float _frequency = 5f;

        internal override uint NativeJointKind => 7;

        public Vector2 anchor
        {
            get => _anchor;
            set
            {
                _anchor = value;
                SyncToNative();
            }
        }

        public Vector2 target
        {
            get => _target;
            set
            {
                _target = value;
                SyncToNative();
            }
        }

        public bool autoConfigureTarget
        {
            get => _autoConfigureTarget;
            set
            {
                _autoConfigureTarget = value;
                SyncToNative();
            }
        }

        public float maxForce
        {
            get => _maxForce;
            set
            {
                _maxForce = value;
                SyncToNative();
            }
        }

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
    }

    public sealed class WheelJoint2D : AnchoredJoint2D
    {
        private JointSuspension2D _suspension;
        private bool _useMotor;
        private JointMotor2D _motor;

        internal override uint NativeJointKind => 8;

        public JointSuspension2D suspension
        {
            get => _suspension;
            set
            {
                _suspension = value;
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

        public JointMotor2D motor
        {
            get => _motor;
            set
            {
                _motor = value;
                SyncToNative();
            }
        }
    }
}
