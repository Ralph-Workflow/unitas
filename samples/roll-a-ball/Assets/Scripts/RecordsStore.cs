using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public static class RecordsStore
    {
        private const string MedalsKey = "records.medals";
        private const string QualityKey = "settings.quality";

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        public static void RestoreAndSave()
        {
            // Active only when the lane arms a data path; otherwise medal logs would
            // differ run-to-run and break the frame-rate sweep's log-order comparison.
            if (string.IsNullOrEmpty(System.Environment.GetEnvironmentVariable("UNITAS_PERSISTENT_DATA_PATH")))
            {
                return;
            }
            int medals = PlayerPrefs.GetInt(MedalsKey, 0);
            Debug.Log($"Records loaded medals {medals}");
            PlayerPrefs.SetInt(MedalsKey, medals + 1);
            PlayerPrefs.SetString(QualityKey, "high");
            PlayerPrefs.Save();
            Debug.Log($"Records saved medals {medals + 1}");
        }
    }
}
