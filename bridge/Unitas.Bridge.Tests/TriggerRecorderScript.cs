using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class TriggerRecorderScript : MonoBehaviour
    {
        private readonly System.Collections.Generic.List<string> _events = new System.Collections.Generic.List<string>();
        private bool _otherColliderAlwaysNull = true;

        public string ObservedOrder => string.Join(", ", _events);

        public bool OtherColliderAlwaysNull => _otherColliderAlwaysNull;

        public override void OnTriggerEnter(Collider? other)
        {
            Record(other, "Enter");
        }

        public override void OnTriggerStay(Collider? other)
        {
            Record(other, "Stay");
        }

        public override void OnTriggerExit(Collider? other)
        {
            Record(other, "Exit");
        }

        private void Record(Collider? other, string name)
        {
            if (other != null)
            {
                _otherColliderAlwaysNull = false;
            }

            _events.Add(name);
        }
    }
}
