using System;

namespace UnityEngine
{
    public class RectOffset
    {
        private int _left;
        private int _right;
        private int _top;
        private int _bottom;
        private Action? _onChanged;

        public RectOffset()
        {
        }

        public RectOffset(int left, int right, int top, int bottom)
        {
            _left = left;
            _right = right;
            _top = top;
            _bottom = bottom;
        }

        public int left
        {
            get => _left;
            set
            {
                _left = value;
                _onChanged?.Invoke();
            }
        }

        public int right
        {
            get => _right;
            set
            {
                _right = value;
                _onChanged?.Invoke();
            }
        }

        public int top
        {
            get => _top;
            set
            {
                _top = value;
                _onChanged?.Invoke();
            }
        }

        public int bottom
        {
            get => _bottom;
            set
            {
                _bottom = value;
                _onChanged?.Invoke();
            }
        }

        public int horizontal => _left + _right;

        public int vertical => _top + _bottom;

        internal void AttachCallback(Action? onChanged)
        {
            _onChanged = onChanged;
        }

        internal ulong Pack16()
        {
            ulong l = (ushort)(short)Math.Clamp(_left, short.MinValue, short.MaxValue);
            ulong r = (ushort)(short)Math.Clamp(_right, short.MinValue, short.MaxValue);
            ulong t = (ushort)(short)Math.Clamp(_top, short.MinValue, short.MaxValue);
            ulong b = (ushort)(short)Math.Clamp(_bottom, short.MinValue, short.MaxValue);
            return l | (r << 16) | (t << 32) | (b << 48);
        }
    }
}

namespace UnityEngine.UI
{
    public abstract class LayoutGroup : MonoBehaviour
    {
        private RectOffset _padding;
        private TextAnchor _childAlignment = TextAnchor.UpperLeft;

        protected LayoutGroup()
        {
            _padding = new RectOffset();
            _padding.AttachCallback(ApplyLayoutGroup);
        }

        public RectOffset padding
        {
            get => _padding;
            set
            {
                _padding.AttachCallback(null);
                _padding = value ?? new RectOffset();
                _padding.AttachCallback(ApplyLayoutGroup);
                ApplyLayoutGroup();
            }
        }

        public TextAnchor childAlignment
        {
            get => _childAlignment;
            set
            {
                _childAlignment = value;
                ApplyLayoutGroup();
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

        public virtual void CalculateLayoutInputHorizontal()
        {
            ApplyLayoutGroup();
        }

        public virtual void CalculateLayoutInputVertical()
        {
            ApplyLayoutGroup();
        }

        public virtual void SetLayoutHorizontal()
        {
            ApplyLayoutGroup();
        }

        public virtual void SetLayoutVertical()
        {
            ApplyLayoutGroup();
        }

        public override void OnEnable()
        {
            ApplyLayoutGroup();
        }

        public override void OnDisable()
        {
            ApplyLayoutGroup();
        }

        protected abstract void ApplyLayoutGroup();
    }

    public abstract class HorizontalOrVerticalLayoutGroup : LayoutGroup
    {
        private float _spacing;
        private bool _childForceExpandWidth = true;
        private bool _childForceExpandHeight = true;
        private bool _childControlWidth = true;
        private bool _childControlHeight = true;
        private bool _childScaleWidth;
        private bool _childScaleHeight;
        private bool _reverseArrangement;

        public float spacing
        {
            get => _spacing;
            set
            {
                _spacing = value;
                ApplyLayoutGroup();
            }
        }

        public bool childForceExpandWidth
        {
            get => _childForceExpandWidth;
            set
            {
                _childForceExpandWidth = value;
                ApplyLayoutGroup();
            }
        }

        public bool childForceExpandHeight
        {
            get => _childForceExpandHeight;
            set
            {
                _childForceExpandHeight = value;
                ApplyLayoutGroup();
            }
        }

        public bool childControlWidth
        {
            get => _childControlWidth;
            set
            {
                _childControlWidth = value;
                ApplyLayoutGroup();
            }
        }

        public bool childControlHeight
        {
            get => _childControlHeight;
            set
            {
                _childControlHeight = value;
                ApplyLayoutGroup();
            }
        }

        public bool childScaleWidth
        {
            get => _childScaleWidth;
            set
            {
                _childScaleWidth = value;
                ApplyLayoutGroup();
            }
        }

        public bool childScaleHeight
        {
            get => _childScaleHeight;
            set
            {
                _childScaleHeight = value;
                ApplyLayoutGroup();
            }
        }

        public bool reverseArrangement
        {
            get => _reverseArrangement;
            set
            {
                _reverseArrangement = value;
                ApplyLayoutGroup();
            }
        }

        protected abstract uint AxisCode { get; }

        protected override void ApplyLayoutGroup()
        {
            IUiProvider? provider = Ui.Provider;
            ulong objectId = NativeObjectId;
            if (provider == null || objectId == 0UL)
            {
                return;
            }

            uint flags = 0U;
            if (_childControlWidth)
            {
                flags |= 1U;
            }

            if (_childControlHeight)
            {
                flags |= 2U;
            }

            if (_childForceExpandWidth)
            {
                flags |= 4U;
            }

            if (_childForceExpandHeight)
            {
                flags |= 8U;
            }

            if (isActiveAndEnabled)
            {
                flags |= 16U;
            }

            provider.SetLayoutGroup(
                objectId,
                AxisCode,
                padding.Pack16(),
                _spacing,
                childAlignment,
                flags);
        }
    }

    public class HorizontalLayoutGroup : HorizontalOrVerticalLayoutGroup
    {
        protected override uint AxisCode => 0U;
    }

    public class VerticalLayoutGroup : HorizontalOrVerticalLayoutGroup
    {
        protected override uint AxisCode => 1U;
    }
}
