using UnityEngine;

namespace Unitas.Samples.RollABall
{
    /// <summary>
    /// Owns the reduced-motion setting. The PlayerPrefs key is shared with the
    /// settings panel, which flips the same key; both sides read it as an int.
    /// </summary>
    public static class ReducedMotion
    {
        private const string PrefsKey = "reduced_motion";

        public static bool IsEnabled => PlayerPrefs.GetInt(PrefsKey, 0) != 0;

        public static void SetEnabled(bool enabled)
        {
            PlayerPrefs.SetInt(PrefsKey, enabled ? 1 : 0);
            PlayerPrefs.Save();
            Debug.Log(enabled ? "Reduced motion on" : "Reduced motion off");
        }

        public static void Toggle()
        {
            SetEnabled(!IsEnabled);
        }
    }
}
