namespace UnityEngine
{
    public class PhysicsMaterial2D : Object
    {
        private float _friction = 0.4f;
        private float _bounciness;

        public PhysicsMaterial2D()
        {
        }

        public PhysicsMaterial2D(string name)
        {
            this.name = name ?? string.Empty;
        }

        public float friction
        {
            get => _friction;
            set => _friction = value;
        }

        public float bounciness
        {
            get => _bounciness;
            set => _bounciness = value;
        }
    }
}
