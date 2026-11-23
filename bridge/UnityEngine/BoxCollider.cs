namespace UnityEngine
{
    public class BoxCollider : Collider
    {
        private Vector3 _size = new Vector3(1f, 1f, 1f);
        private Vector3 _center;

        public Vector3 size
        {
            get => _size;
            set => _size = value;
        }

        public Vector3 center
        {
            get => _center;
            set => _center = value;
        }
    }
}
