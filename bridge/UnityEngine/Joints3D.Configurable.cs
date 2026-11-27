namespace UnityEngine
{
    public sealed class ConfigurableJoint : Joint
    {
        private Vector3 _secondaryAxis = Vector3.up;
        private ConfigurableJointMotion _xMotion = ConfigurableJointMotion.Free;
        private ConfigurableJointMotion _yMotion = ConfigurableJointMotion.Free;
        private ConfigurableJointMotion _zMotion = ConfigurableJointMotion.Free;
        private ConfigurableJointMotion _angularXMotion = ConfigurableJointMotion.Free;
        private ConfigurableJointMotion _angularYMotion = ConfigurableJointMotion.Free;
        private ConfigurableJointMotion _angularZMotion = ConfigurableJointMotion.Free;
        private SoftJointLimit _linearLimit;
        private SoftJointLimit _lowAngularXLimit;
        private SoftJointLimit _highAngularXLimit;
        private SoftJointLimit _angularYLimit;
        private SoftJointLimit _angularZLimit;
        private JointDrive _xDrive;
        private JointDrive _yDrive;
        private JointDrive _zDrive;
        private Vector3 _targetPosition = Vector3.zero;
        private Vector3 _targetVelocity = Vector3.zero;

        internal override uint NativeJointKind => 4;

        public Vector3 secondaryAxis
        {
            get => _secondaryAxis;
            set
            {
                _secondaryAxis = value;
                SyncToNative();
            }
        }

        public ConfigurableJointMotion xMotion
        {
            get => _xMotion;
            set
            {
                _xMotion = value;
                SyncToNative();
            }
        }

        public ConfigurableJointMotion yMotion
        {
            get => _yMotion;
            set
            {
                _yMotion = value;
                SyncToNative();
            }
        }

        public ConfigurableJointMotion zMotion
        {
            get => _zMotion;
            set
            {
                _zMotion = value;
                SyncToNative();
            }
        }

        public ConfigurableJointMotion angularXMotion
        {
            get => _angularXMotion;
            set
            {
                _angularXMotion = value;
                SyncToNative();
            }
        }

        public ConfigurableJointMotion angularYMotion
        {
            get => _angularYMotion;
            set
            {
                _angularYMotion = value;
                SyncToNative();
            }
        }

        public ConfigurableJointMotion angularZMotion
        {
            get => _angularZMotion;
            set
            {
                _angularZMotion = value;
                SyncToNative();
            }
        }

        public SoftJointLimit linearLimit
        {
            get => _linearLimit;
            set
            {
                _linearLimit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit lowAngularXLimit
        {
            get => _lowAngularXLimit;
            set
            {
                _lowAngularXLimit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit highAngularXLimit
        {
            get => _highAngularXLimit;
            set
            {
                _highAngularXLimit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit angularYLimit
        {
            get => _angularYLimit;
            set
            {
                _angularYLimit = value;
                SyncToNative();
            }
        }

        public SoftJointLimit angularZLimit
        {
            get => _angularZLimit;
            set
            {
                _angularZLimit = value;
                SyncToNative();
            }
        }

        public JointDrive xDrive
        {
            get => _xDrive;
            set
            {
                _xDrive = value;
                SyncToNative();
            }
        }

        public JointDrive yDrive
        {
            get => _yDrive;
            set
            {
                _yDrive = value;
                SyncToNative();
            }
        }

        public JointDrive zDrive
        {
            get => _zDrive;
            set
            {
                _zDrive = value;
                SyncToNative();
            }
        }

        public Vector3 targetPosition
        {
            get => _targetPosition;
            set
            {
                _targetPosition = value;
                SyncToNative();
            }
        }

        public Vector3 targetVelocity
        {
            get => _targetVelocity;
            set
            {
                _targetVelocity = value;
                SyncToNative();
            }
        }
    }
}
