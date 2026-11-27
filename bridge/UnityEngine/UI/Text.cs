namespace UnityEngine
{
    public enum TextAnchor
    {
        UpperLeft = 0,
        UpperCenter = 1,
        UpperRight = 2,
        MiddleLeft = 3,
        MiddleCenter = 4,
        MiddleRight = 5,
        LowerLeft = 6,
        LowerCenter = 7,
        LowerRight = 8,
    }

    public enum HorizontalWrapMode
    {
        Wrap = 0,
        Overflow = 1,
    }

    public enum VerticalWrapMode
    {
        Truncate = 0,
        Overflow = 1,
    }
}

namespace UnityEngine.UI
{
    public class Text : MonoBehaviour
    {
        private string _text = string.Empty;
        private int _fontSize = 14;
        private Color _color = Color.white;
        private TextAnchor _alignment = TextAnchor.UpperLeft;
        private HorizontalWrapMode _horizontalOverflow = HorizontalWrapMode.Overflow;
        private VerticalWrapMode _verticalOverflow = VerticalWrapMode.Overflow;
        private float _lineSpacing = 1f;

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

        public int fontSize
        {
            get => _fontSize;
            set
            {
                _fontSize = value;
                ApplyStyle();
            }
        }

        public Color color
        {
            get => _color;
            set
            {
                _color = value;
                ApplyStyle();
            }
        }

        public TextAnchor alignment
        {
            get => _alignment;
            set
            {
                _alignment = value;
                ApplyLayout();
            }
        }

        public HorizontalWrapMode horizontalOverflow
        {
            get => _horizontalOverflow;
            set
            {
                _horizontalOverflow = value;
                ApplyLayout();
            }
        }

        public VerticalWrapMode verticalOverflow
        {
            get => _verticalOverflow;
            set
            {
                _verticalOverflow = value;
                ApplyLayout();
            }
        }

        public float lineSpacing
        {
            get => _lineSpacing;
            set
            {
                _lineSpacing = value;
                ApplyLayout();
            }
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

        private void ApplyLayout()
        {
            IUiProvider? provider = Ui.Provider;
            ulong objectId = NativeObjectId;
            if (provider == null || objectId == 0UL)
            {
                return;
            }

            provider.SetTextLayout(
                objectId,
                _alignment,
                _horizontalOverflow,
                _verticalOverflow,
                _lineSpacing);
        }
    }
}
