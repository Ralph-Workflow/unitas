namespace UnityEngine.UI
{
    /// <summary>Managed uGUI text: a thin wrapper over the native UI object.</summary>
    public class Text : MonoBehaviour
    {
        private string _text = string.Empty;
        private int _fontSize = 14;
        private Color _color = Color.white;

        /// <summary>The text the label shows. Writing it reaches the native canvas.</summary>
        public string text
        {
            get => _text;
            set
            {
                _text = value ?? string.Empty;
                IUiProvider? provider = Ui.Provider;
                ulong objectId = NativeObjectId;
                if (provider != null && objectId != 0UL)
                {
                    provider.SetText(objectId, _text);
                }
            }
        }

        /// <summary>The font size in points. Writing it reaches the native canvas.</summary>
        public int fontSize
        {
            get => _fontSize;
            set
            {
                _fontSize = value;
                ApplyStyle();
            }
        }

        /// <summary>The label colour. Writing it reaches the native canvas.</summary>
        public Color color
        {
            get => _color;
            set
            {
                _color = value;
                ApplyStyle();
            }
        }

        /// <summary>Native scene object id of the game object this label belongs to.</summary>
        public ulong NativeObjectId
        {
            get
            {
                GameObject? owner = Owner;
                return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            }
        }

        internal GameObject? Owner => OwnerOrNull;

        private void ApplyStyle()
        {
            IUiProvider? provider = Ui.Provider;
            ulong objectId = NativeObjectId;
            if (provider == null || objectId == 0UL)
            {
                return;
            }

            provider.SetTextStyle(objectId, _fontSize, _color);
        }
    }
}
