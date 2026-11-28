using System;
using System.Collections.Generic;

namespace UnityEngine.UI
{
    public readonly struct UiInputFieldValueChangedEvent
    {
        public UiInputFieldValueChangedEvent(ulong objectId, string text)
        {
            ObjectId = objectId;
            Text = text ?? string.Empty;
        }

        public ulong ObjectId { get; }

        public string Text { get; }
    }

    public readonly struct UiInputFieldEndEditEvent
    {
        public UiInputFieldEndEditEvent(ulong objectId, string text, bool canceled)
        {
            ObjectId = objectId;
            Text = text ?? string.Empty;
            Canceled = canceled;
        }

        public ulong ObjectId { get; }

        public string Text { get; }

        public bool Canceled { get; }
    }

    public interface IUiProvider
    {
        int SetText(ulong objectId, string text);

        int SetTextStyle(ulong objectId, float fontSize, Color color);

        int SetTextLayout(
            ulong objectId,
            TextAnchor alignment,
            HorizontalWrapMode horizontalOverflow,
            VerticalWrapMode verticalOverflow,
            float lineSpacing);

        int SetLayoutGroup(
            ulong objectId,
            uint axis,
            ulong paddingPacked,
            float spacing,
            TextAnchor childAlignment,
            uint flags) => 0;

        int SetContentSizeFitter(
            ulong objectId,
            ContentSizeFitter.FitMode horizontalFit,
            ContentSizeFitter.FitMode verticalFit,
            bool enabled) => 0;

        int CreateInputField(
            ulong objectId,
            bool interactable,
            int characterLimit,
            float fontSize) => 0;

        int SetInputFieldText(
            ulong objectId,
            string text,
            bool notify) => 0;

        bool TryGetInputFieldText(
            ulong objectId,
            out string text)
        {
            text = string.Empty;
            return false;
        }

        int SetInputFieldPlaceholder(
            ulong objectId,
            string placeholder) => 0;

        int SetInputFieldInteractable(
            ulong objectId,
            bool interactable) => 0;

        int SetInputFieldCharacterLimit(
            ulong objectId,
            int characterLimit) => 0;

        int GetInputFieldCaretPosition(ulong objectId) => -1;

        int SetInputFieldCaretPosition(
            ulong objectId,
            int caretPosition) => 0;

        int FocusInputField(
            ulong objectId,
            bool focused) => 0;

        bool IsInputFieldFocused(ulong objectId) => false;

        bool WasInputFieldCanceled(ulong objectId) => false;

        int InjectCharEvent(char character) => 0;

        int PollInputFieldValueChanged(List<UiInputFieldValueChangedEvent> events) => 0;

        int PollInputFieldEndEdit(List<UiInputFieldEndEditEvent> events) => 0;

        bool IsPointerOver();

        int PollClicks(ulong[] clicks);
    }

    public static class Ui
    {
        private const int ClickBufferSize = 64;

        private static readonly ulong[] ClickBuffer = new ulong[ClickBufferSize];
        private static readonly List<UiInputFieldValueChangedEvent> ValueChangedBuffer =
            new List<UiInputFieldValueChangedEvent>();
        private static readonly List<UiInputFieldEndEditEvent> EndEditBuffer =
            new List<UiInputFieldEndEditEvent>();

        public static IUiProvider? Provider { get; set; }

        public static int PumpClicks()
        {
            IUiProvider? provider = Provider;
            if (provider == null)
            {
                return 0;
            }

            PumpInputFields(provider);

            int count = provider.PollClicks(ClickBuffer);
            if (count <= 0)
            {
                return 0;
            }

            if (count > ClickBuffer.Length)
            {
                count = ClickBuffer.Length;
            }

            for (int index = 0; index < count; index++)
            {
                DispatchClick(ClickBuffer[index]);
            }

            return count;
        }

        public static int PumpInputFields()
        {
            IUiProvider? provider = Provider;
            return provider != null ? PumpInputFields(provider) : 0;
        }

        private static int PumpInputFields(IUiProvider provider)
        {
            UiInputFields.SyncAll(provider);

            ValueChangedBuffer.Clear();
            int valueChangedCount = provider.PollInputFieldValueChanged(ValueChangedBuffer);
            for (int index = 0; index < ValueChangedBuffer.Count; index++)
            {
                UiInputFieldValueChangedEvent ev = ValueChangedBuffer[index];
                InputField? field = UiInputFields.Find(ev.ObjectId);
                field?.HandleNativeValueChanged(ev.Text);
            }

            EndEditBuffer.Clear();
            int endEditCount = provider.PollInputFieldEndEdit(EndEditBuffer);
            for (int index = 0; index < EndEditBuffer.Count; index++)
            {
                UiInputFieldEndEditEvent ev = EndEditBuffer[index];
                InputField? field = UiInputFields.Find(ev.ObjectId);
                field?.HandleNativeEndEdit(ev.Text, ev.Canceled);
            }

            return Math.Max(0, valueChangedCount) + Math.Max(0, endEditCount);
        }

        private static void DispatchClick(ulong objectId)
        {
            Button? button = UiButtons.Find(objectId);
            if (button == null)
            {
                return;
            }

            GameObject? owner = button.Owner;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }

            owner.SendMessage("OnClick", SendMessageOptions.DontRequireReceiver);
        }
    }
}
