namespace UnityEngine.UI
{
    public class Mask : MonoBehaviour
    {
        private bool _showMaskGraphic = true;

        public bool showMaskGraphic
        {
            get => _showMaskGraphic;
            set => _showMaskGraphic = value;
        }

        public ulong NativeObjectId
        {
            get
            {
                GameObject? owner = Owner;
                return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            }
        }

        internal GameObject? Owner => OwnerOrNull;
    }
}
