using System;

namespace UnityEngine
{
    public static class QualitySettings
    {
        private static readonly string[] DefaultNames = new[]
        {
            "Very Low",
            "Low",
            "Medium",
            "High",
            "Very High",
            "Ultra",
        };

        private static string[] _names = (string[])DefaultNames.Clone();
        private static int _qualityLevel = 2;
        private static int _vSyncCount = 1;

        public static Action<int>? VSyncCountChangeHandler { get; set; }

        public static Action<int>? QualityLevelChangeHandler { get; set; }

        public static string[] names
        {
            get => (string[])_names.Clone();
            set
            {
                if (value != null && value.Length > 0)
                {
                    _names = (string[])value.Clone();
                    if (_qualityLevel >= _names.Length)
                    {
                        _qualityLevel = _names.Length - 1;
                    }
                }
            }
        }

        public static int vSyncCount
        {
            get => _vSyncCount;
            set
            {
                int clamped = value < 0 ? 0 : (value > 4 ? 4 : value);
                _vSyncCount = clamped;
                VSyncCountChangeHandler?.Invoke(clamped);
            }
        }

        public static int GetQualityLevel() => _qualityLevel;

        public static void SetQualityLevel(int index, bool applyExpensiveChanges = true)
        {
            _ = applyExpensiveChanges;
            if (index < 0)
            {
                return;
            }

            if (_names.Length > 0 && index >= _names.Length)
            {
                index = _names.Length - 1;
            }

            _qualityLevel = index;
            QualityLevelChangeHandler?.Invoke(index);
        }

        public static void IncreaseLevel(bool applyExpensiveChanges = false)
        {
            if (_qualityLevel + 1 < _names.Length)
            {
                SetQualityLevel(_qualityLevel + 1, applyExpensiveChanges);
            }
        }

        public static void DecreaseLevel(bool applyExpensiveChanges = false)
        {
            if (_qualityLevel > 0)
            {
                SetQualityLevel(_qualityLevel - 1, applyExpensiveChanges);
            }
        }

        public static void ResetToDefaults()
        {
            _names = (string[])DefaultNames.Clone();
            _qualityLevel = 2;
            _vSyncCount = 1;
        }
    }
}
