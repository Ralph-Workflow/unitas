using System;

namespace UnityEngine.UI
{
    /// <summary>
    /// Native uGUI surface the managed slice reads and writes. The bridge implements it
    /// against the runtime ABI; tests implement it with a fake.
    /// </summary>
    public interface IUiProvider
    {
        /// <summary>Sets the text of the UI text object with the given native object id.</summary>
        int SetText(ulong objectId, string text);

        /// <summary>Sets font size and colour of the UI text object with the given native object id.</summary>
        int SetTextStyle(ulong objectId, float fontSize, Color color);

        /// <summary>True when the pointer is over a UI graphic.</summary>
        bool IsPointerOver();

        /// <summary>
        /// Drains the pending native clicks into <paramref name="clicks"/> and returns the
        /// number of native object ids written.
        /// </summary>
        int PollClicks(ulong[] clicks);
    }

    /// <summary>Holds the provider the managed uGUI slice talks to.</summary>
    public static class Ui
    {
        private const int ClickBufferSize = 64;

        private static readonly ulong[] ClickBuffer = new ulong[ClickBufferSize];

        public static IUiProvider? Provider { get; set; }

        /// <summary>
        /// Drains the native click queue and hands every clicked object id to the managed
        /// button that owns it. Called once per frame after the runtime ticked.
        /// </summary>
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

            // The click travels the same message path a Unity button click travels.
            owner.SendMessage("OnClick", SendMessageOptions.DontRequireReceiver);
        }
    }
}
