namespace UnityEngine
{
    public class BoxCollider2D : Collider2D
    {
        private Vector2 _size = Vector2.one;

        public Vector2 size
        {
            get => _size;
            set => _size = value;
        }
    }
}
