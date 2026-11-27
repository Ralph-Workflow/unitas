namespace UnityEngine
{
    public struct ContactPoint2D
    {
        public Vector2 point;
        public Vector2 normal;
        public float impulse;
    }

    public struct CollisionSummary2D
    {
        public float impulse;
        public Vector2 relativeVelocity;
        public int contactCount;
        public Vector2 point;
        public Vector2 normal;
    }
}
