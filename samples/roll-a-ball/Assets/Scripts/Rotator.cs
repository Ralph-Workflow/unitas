using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class Rotator : MonoBehaviour
    {
        private const float DefaultBobHeight = 0.2f;
        private const float DefaultBobSpeed = 2f;
        private const float DefaultRotateSpeed = 60f;

        [SerializeField]
        private float bobHeight = DefaultBobHeight;

        [SerializeField]
        private float bobSpeed = DefaultBobSpeed;

        [SerializeField]
        private float rotateSpeed = DefaultRotateSpeed;

        private Vector3 _origin;

        public override void Start()
        {
            _origin = transform.position;
        }

        public override void Update()
        {
            transform.Rotate(Vector3.up * (rotateSpeed * Time.deltaTime), Space.World);
            transform.position = _origin + (Vector3.up * (Mathf.Sin(Time.time * bobSpeed) * bobHeight));
        }
    }
}
