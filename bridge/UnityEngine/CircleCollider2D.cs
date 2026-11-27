namespace UnityEngine
{
    public class CircleCollider2D : Collider2D
    {
        private float _radius = 0.5f;

        public float radius
        {
            get => _radius;
            set => _radius = value;
        }
    }
}
