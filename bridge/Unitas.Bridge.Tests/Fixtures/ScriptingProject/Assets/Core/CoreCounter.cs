using UnityEngine;

namespace MyGame.Core
{
    public class CoreCounter : MonoBehaviour
    {
        public int Count;
        public string LastMessage = string.Empty;
        private int _secretSteps;

        public void Increment()
        {
            Count += 1;
        }

        public void AddAmount(int amount)
        {
            Count += amount;
        }

        public void SetLabel(string label)
        {
            LastMessage = label;
        }

        private void OnSecretPing()
        {
            _secretSteps += 5;
            Count += 5;
        }

        private void OnSecretBonus(int bonus)
        {
            _secretSteps += bonus;
            Count += bonus;
        }

        public static string ActiveProfileSummary()
        {
            string summary = "BASE";
#if UNITY_2022_3_OR_NEWER
            summary += "|U2022_3";
#endif
#if UNITY_EDITOR
            summary += "|EDITOR";
#else
            summary += "|PLAYER";
#endif
#if UNITY_SERVER
            summary += "|SERVER";
#endif
#if DEVELOPMENT_BUILD
            summary += "|DEV";
#endif
#if CSC_CUSTOM_SYMBOL
            summary += "|CSC_RSP";
#endif
#if HAS_ANALYTICS_1_2
            summary += "|ANALYTICS_1_2";
#endif
            return summary;
        }
    }
}
