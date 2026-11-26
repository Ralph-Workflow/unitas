namespace UnityEngine.UI
{
    /// <summary>Managed uGUI image: a thin wrapper over the native UI object.</summary>
    public class Image : MonoBehaviour
    {
        private Color _color = Color.white;
        private string _sprite = string.Empty;

        /// <summary>The colour the graphic is filled with.</summary>
        public Color color
        {
            get => _color;
            set => _color = value;
        }

        /// <summary>The guid of the sprite the graphic shows, empty when it shows none.</summary>
        public string sprite
        {
            get => _sprite;
            set => _sprite = value ?? string.Empty;
        }

        /// <summary>Native scene object id of the game object this graphic belongs to.</summary>
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
