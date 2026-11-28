using UnityEngine;

namespace Unitas.Samples.MarbleRush
{
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
