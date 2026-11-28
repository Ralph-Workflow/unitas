using System;
using System.Collections.Generic;
using UnityEngine.Events;

namespace UnityEngine.UI
{
    public class Toggle : MonoBehaviour
    {
        public sealed class ToggleEvent : UnityEvent<bool>
        {
            public ToggleEvent() : base("Toggle", "m_OnValueChanged")
            {
            }
        }

        private ToggleEvent? _onValueChanged;
        private bool _isOn;
        private bool _interactable = true;
        private ToggleGroup? _group;
        private Component? _graphic;

        public Toggle()
        {
            UiToggles.Register(this);
        }

        public ToggleEvent onValueChanged
        {
            get => _onValueChanged ??= new ToggleEvent();
            set => _onValueChanged = value;
        }

        public bool isOn
        {
            get => _isOn;
            set => SetIsOn(value, sendCallback: true);
        }

        public bool interactable
        {
            get => _interactable;
            set => _interactable = value;
        }

        public ToggleGroup? group
        {
            get => _group;
            set
            {
                _group?.Unregister(this);
                _group = value;
                _group?.Register(this);
            }
        }

        public Component? graphic
        {
            get => _graphic;
            set => _graphic = value;
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

        public void SetIsOnWithoutNotify(bool value)
        {
            SetIsOn(value, sendCallback: false);
        }

        public bool HandleNativeValueChanged(bool value)
        {
            SetIsOn(value, sendCallback: true);
            return true;
        }

        private void SetIsOn(bool value, bool sendCallback)
        {
            if (_isOn == value)
            {
                return;
            }
            _isOn = value;
            if (_isOn && _group != null && _group.ToggleAllowedExclusive(this))
            {
                _group.NotifyToggleOn(this);
            }
            if (sendCallback)
            {
                onValueChanged.Invoke(_isOn);
            }
        }
    }

    public class ToggleGroup : MonoBehaviour
    {
        private readonly List<Toggle> _toggles = new List<Toggle>();
        private bool _allowSwitchOff;

        public bool allowSwitchOff
        {
            get => _allowSwitchOff;
            set => _allowSwitchOff = value;
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

        internal void Register(Toggle toggle)
        {
            if (!_toggles.Contains(toggle))
            {
                _toggles.Add(toggle);
            }
        }

        internal void Unregister(Toggle toggle)
        {
            _toggles.Remove(toggle);
        }

        internal bool ToggleAllowedExclusive(Toggle active)
        {
            for (int index = 0; index < _toggles.Count; index++)
            {
                Toggle toggle = _toggles[index];
                if (toggle != active && toggle.isOn && !toggle.interactable)
                {
                    return false;
                }
            }
            return true;
        }

        internal void NotifyToggleOn(Toggle active)
        {
            for (int index = 0; index < _toggles.Count; index++)
            {
                Toggle toggle = _toggles[index];
                if (toggle != active)
                {
                    toggle.SetIsOnWithoutNotify(false);
                }
            }
        }
    }

    internal static class UiToggles
    {
        private static readonly List<Toggle> Toggles = new List<Toggle>();

        internal static void Register(Toggle toggle)
        {
            Toggles.Add(toggle);
        }

        internal static Toggle? Find(ulong objectId)
        {
            if (objectId == 0UL)
            {
                return null;
            }
            for (int index = Toggles.Count - 1; index >= 0; index--)
            {
                Toggle toggle = Toggles[index];
                if (toggle == null || toggle.IsDestroyed)
                {
                    Toggles.RemoveAt(index);
                    continue;
                }
                if (toggle.NativeObjectId == objectId)
                {
                    return toggle;
                }
            }
            return null;
        }
    }
}
