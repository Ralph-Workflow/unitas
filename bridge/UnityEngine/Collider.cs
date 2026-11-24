namespace UnityEngine
{
    public abstract class Collider : Component
    {
        private bool _isTrigger;

        public bool isTrigger
        {
            get => _isTrigger;
            set => _isTrigger = value;
        }

        public Rigidbody? attachedRigidbody
        {
            get
            {
                GameObject? owner = gameObject;
                return owner == null ? null : owner.GetComponent<Rigidbody>();
            }
        }
    }
}
