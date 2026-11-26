using System;
using System.Collections.Generic;
using UnityEngine.Events;

namespace UnityEngine.UI
{
    /// <summary>Managed uGUI button: a thin wrapper over the native UI object.</summary>
    public class Button : MonoBehaviour, IUnitasMessageHandler
    {
        private UnityEvent? _onClick;
        private bool _interactable = true;

        public Button()
        {
            UiButtons.Register(this);
        }

        /// <summary>Invoked when the native side dispatches a click for this button.</summary>
        public UnityEvent onClick => _onClick ??= new UnityEvent("Button", "m_OnClick");

        public bool interactable
        {
            get => _interactable;
            set => _interactable = value;
        }

        /// <summary>Native scene object id of the game object this button belongs to.</summary>
        public ulong NativeObjectId
        {
            get
            {
                GameObject? owner = Owner;
                return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            }
        }

        internal GameObject? Owner => OwnerOrNull;

        public bool Dispatch(string message) =>
            string.Equals(message, "OnClick", StringComparison.Ordinal) ? InvokeOnClick() : false;

        public bool Dispatch<T>(string message, T argument) => Dispatch(message);

        private bool InvokeOnClick()
        {
            if (!interactable)
            {
                return false;
            }

            onClick.Invoke();
            return true;
        }
    }

    /// <summary>Managed buttons keyed by the native scene object id they belong to.</summary>
    internal static class UiButtons
    {
        private static readonly List<Button> Buttons = new List<Button>();

        internal static void Register(Button button)
        {
            Buttons.Add(button);
        }

        internal static Button? Find(ulong objectId)
        {
            if (objectId == 0UL)
            {
                return null;
            }

            for (int index = Buttons.Count - 1; index >= 0; index--)
            {
                Button button = Buttons[index];
                if (button == null || button.IsDestroyed)
                {
                    Buttons.RemoveAt(index);
                    continue;
                }

                if (button.NativeObjectId == objectId)
                {
                    return button;
                }
            }

            return null;
        }
    }
}
