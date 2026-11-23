using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class CollisionRecorderScript : MonoBehaviour
    {
        private readonly System.Collections.Generic.List<string> _events = new System.Collections.Generic.List<string>();
        private bool _otherGameObjectAlwaysNull = true;

        public string ObservedOrder => string.Join(", ", _events);

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
            if (collision?.gameObject != null)
            {
                _otherGameObjectAlwaysNull = false;
            }

            _events.Add(name);
        }
    }
}
