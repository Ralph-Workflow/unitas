namespace UnityEngine
{
    public struct ContactFilter2D
    {
        // the native filter keeps the normals inside [minNormalAngle, maxNormalAngle] once this flag is on.
        public bool useTriggers;

        public bool useOutsideNormalAngle;

        public float minNormalAngle;

        public float maxNormalAngle;

        public bool useDepth;

        public float minDepth;

        public float maxDepth;

        public static ContactFilter2D noFilter => default;
    }
}
