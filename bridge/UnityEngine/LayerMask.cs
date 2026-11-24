namespace UnityEngine
{
    public struct LayerMask
    {
        private int _value;

        public LayerMask(int value)
        {
            _value = value;
        }

        public int value
        {
            get => _value;
            set => _value = value;
        }

        public static implicit operator int(LayerMask mask) => mask.value;

        public static implicit operator LayerMask(int value) => new LayerMask(value);
    }
}
