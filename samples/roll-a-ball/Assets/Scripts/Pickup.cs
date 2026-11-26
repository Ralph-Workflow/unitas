using UnityEngine;

namespace Unitas.Samples.RollABall
{
    // Pickup on a prefab instance: it reports the collect burst and the gold
    // variant overrides that the scene loader applied to the instance.
    public class Pickup : MonoBehaviour
    {
        private const string BurstObjectName = "CollectBurst";
        private const string GoldVariantName = "PickupGold";
        // The added child is the gold variant's structural override; the
        // renamed sibling carries its non-transform property override.
        private const string GoldAddedChildName = "GoldAura";
        private const string GoldOverriddenChildName = "PickupVisualGold";
        // The bridge names the scene objects it parses. A prefab instance has
        // no scene document, so its managed name falls back to this default.
        private const string AnonymousName = "GameObject";
        private const int BaseScoreValue = 1;

        [SerializeField]
        private int scoreValue = BaseScoreValue;

        [SerializeField]
        private string burstObjectName = BurstObjectName;

        private static int _instanceCount;

        private bool _variantChecked;
        private int _ordinal;

        public int ScoreValue => scoreValue;

        public override void Start()
        {
            _ordinal = ++_instanceCount;
            ReportVariantOverrides();
        }

        // ScoreManager owns collection so the burst report fires exactly once
        // per pickup in a deterministic order at every frame rate.
        public void ReportCollectBurst()
        {
            TriggerBurst();
            Debug.Log($"[roll-a-ball] collect burst triggered pickup={PickupName()}");
        }

        // The burst is a scene particle system that emits when it becomes
        // active, so re-arming it is a deactivate/activate pair.
        private void TriggerBurst()
        {
            GameObject? burst = GameObject.Find(burstObjectName);
            if (burst == null)
            {
                return;
            }
            burst.SetActive(false);
            burst.SetActive(true);
        }

        private void ReportVariantOverrides()
        {
            if (_variantChecked)
            {
                return;
            }
            _variantChecked = true;
            Transform? added = FindChild(GoldAddedChildName);
            if (added == null && !IsGoldVariant())
            {
                return;
            }
            Transform? overridden = FindChild(GoldOverriddenChildName);
            string addedName = added != null ? added.name : GoldAddedChildName;
            string overriddenName = overridden != null ? overridden.name : GoldOverriddenChildName;
            Debug.Log($"[roll-a-ball] gold variant applied added={addedName} value={overriddenName}");
        }

        private bool IsGoldVariant()
        {
            string name = gameObject.name;
            return name == GoldVariantName || name.StartsWith(GoldVariantName);
        }

        private string PickupName()
        {
            if (IsGoldVariant() || FindChild(GoldAddedChildName) != null)
            {
                return GoldVariantName;
            }
            string name = gameObject.name;
            if (!string.IsNullOrEmpty(name) && name != AnonymousName)
            {
                return name;
            }
            return $"Pickup#{_ordinal}";
        }

        private Transform? FindChild(string childName)
        {
            for (int i = 0; i < transform.childCount; i++)
            {
                Transform child = transform.GetChild(i);
                if (child.name == childName)
                {
                    return child;
                }
            }
            return null;
        }
    }
}
