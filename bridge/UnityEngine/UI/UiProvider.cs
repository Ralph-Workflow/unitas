using System;

namespace UnityEngine.UI
{
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

        bool IsPointerOver();

        int PollClicks(ulong[] clicks);
    }

    public static class Ui
    {
        private const int ClickBufferSize = 64;

        private static readonly ulong[] ClickBuffer = new ulong[ClickBufferSize];

        public static IUiProvider? Provider { get; set; }

        public static int PumpClicks()
        {
            IUiProvider? provider = Provider;
            if (provider == null)
            {
                return 0;
            }

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
