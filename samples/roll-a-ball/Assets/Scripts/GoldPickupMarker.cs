using UnityEngine;

namespace Unitas.Samples.RollABall
{
    // Carried by the child that the gold variant adds to the pickup. The
    // script runs only when the variant's structural override was applied, so
    // its log line is the in-game proof of that override.
    public class GoldPickupMarker : MonoBehaviour
    {
        // The names the variant authors: the added child and the child whose
        // name the variant overrides.
        public const string AddedChildName = "GoldAura";
        public const string OverriddenValue = "PickupVisualGold";

        private new void Start()
        {
            Debug.Log($"[roll-a-ball] gold variant applied added={AddedChildName} value={OverriddenValue}");
        }
    }
}
