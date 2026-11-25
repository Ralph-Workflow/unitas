using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class CollisionRecorderScript : MonoBehaviour
    {
        private readonly System.Collections.Generic.List<string> _events = new System.Collections.Generic.List<string>();
        private readonly System.Collections.Generic.List<string> _otherNames = new System.Collections.Generic.List<string>();
        private bool _otherGameObjectAlwaysNull = true;

        public string ObservedOrder => string.Join(", ", _events);

        public string ObservedOtherNames => string.Join(", ", _otherNames);

        public bool OtherGameObjectAlwaysNull => _otherGameObjectAlwaysNull;

        public override void OnCollisionEnter(Collision collision)
        {
            Record(collision, "Enter");
        }

        public override void OnCollisionStay(Collision collision)
        {
            Record(collision, "Stay");
        }

        public override void OnCollisionExit(Collision collision)
        {
            Record(collision, "Exit");
        }

        private void Record(Collision? collision, string name)
        {
            GameObject? other = collision?.gameObject;
            if (other != null)
            {
                _otherGameObjectAlwaysNull = false;
                if (!_otherNames.Contains(other.name))
                {
                    _otherNames.Add(other.name);
                }
            }

            _events.Add(name);
        }
    }
}
