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
    }
}
