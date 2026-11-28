using System;
using System.Collections.Generic;
using UnityEngine.Events;

namespace UnityEngine.UI
{
    public class Slider : MonoBehaviour
    {
        public enum Direction
        {
            LeftToRight = 0,
            RightToLeft = 1,
            BottomToTop = 2,
            TopToBottom = 3,
        }

        public sealed class SliderEvent : UnityEvent<float>
        {
            public SliderEvent() : base("Slider", "m_OnValueChanged")
            {
            }
        }

        private SliderEvent? _onValueChanged;
        private float _minValue;
        private float _maxValue = 1f;
        private float _value;
        private bool _wholeNumbers;
        private bool _interactable = true;
        private Direction _direction = Direction.LeftToRight;
        private Component? _fillRect;
        private Component? _handleRect;

        public Slider()
        {
            UiSliders.Register(this);
        }

        public SliderEvent onValueChanged
        {
            get => _onValueChanged ??= new SliderEvent();
            set => _onValueChanged = value;
        }

        public float minValue
        {
            get => _minValue;
            set
            {
                _minValue = value;
                SetValue(_value, sendCallback: false);
            }
        }

        public float maxValue
        {
            get => _maxValue;
            set
            {
                _maxValue = value;
                SetValue(_value, sendCallback: false);
            }
        }

        public bool wholeNumbers
        {
            get => _wholeNumbers;
            set
            {
                _wholeNumbers = value;
                SetValue(_value, sendCallback: false);
            }
        }

        public float value
        {
            get => _value;
            set => SetValue(value, sendCallback: true);
        }

        public float normalizedValue
        {
            get
            {
                float span = _maxValue - _minValue;
                return span > 0f ? Mathf.Clamp01((_value - _minValue) / span) : 0f;
            }
            set => SetValue(_minValue + Mathf.Clamp01(value) * (_maxValue - _minValue), true);
        }

        public bool interactable
        {
            get => _interactable;
            set => _interactable = value;
        }

        public Direction direction
        {
            get => _direction;
            set => _direction = value;
        }

        public Component? fillRect
        {
            get => _fillRect;
            set => _fillRect = value;
        }

        public Component? handleRect
        {
            get => _handleRect;
            set => _handleRect = value;
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

        public void SetValueWithoutNotify(float value)
        {
            SetValue(value, sendCallback: false);
        }

        public bool HandleNativeValueChanged(float nativeValue)
        {
            SetValue(nativeValue, sendCallback: true);
            return true;
        }

        private void SetValue(float value, bool sendCallback)
        {
            float clamped = Mathf.Clamp(value, Mathf.Min(_minValue, _maxValue), Mathf.Max(_minValue, _maxValue));
            if (_wholeNumbers)
            {
                clamped = Mathf.Round(clamped);
            }
            if (_value == clamped)
            {
                return;
            }
            _value = clamped;
            if (sendCallback)
            {
                onValueChanged.Invoke(_value);
            }
        }
    }

    internal static class UiSliders
    {
        private static readonly List<Slider> Sliders = new List<Slider>();

        internal static void Register(Slider slider)
        {
            Sliders.Add(slider);
        }

        internal static Slider? Find(ulong objectId)
        {
            if (objectId == 0UL)
            {
                return null;
            }
            for (int index = Sliders.Count - 1; index >= 0; index--)
            {
                Slider slider = Sliders[index];
                if (slider == null || slider.IsDestroyed)
                {
                    Sliders.RemoveAt(index);
                    continue;
                }
                if (slider.NativeObjectId == objectId)
                {
                    return slider;
                }
            }
            return null;
        }
    }
}
