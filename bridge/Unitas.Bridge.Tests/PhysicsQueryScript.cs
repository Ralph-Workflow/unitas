using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class PhysicsQueryScript : MonoBehaviour
    {
        private int _awakeCount;

        public int AwakeCount => _awakeCount;

        public Collider? OwnCollider => GetComponent<Collider>();

        public override void Awake()
        {
            _awakeCount++;
        }
    }
}
