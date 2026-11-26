using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class PhysicsLabProbe : MonoBehaviour
    {
        private const float CoarseStepThreshold = 0.015f;
        private const float ContainmentReportSeconds = 1.5f;
        private const float CarryDistanceThreshold = 0.5f;
        private const float PusherSpeed = 2f;

        [SerializeField]
        private float launchSpeed;

        [SerializeField]
        private float containmentMaxZ;

        [SerializeField]
        private float carryReportSeconds;

        [SerializeField]
        private string bumperName = "Bumper";

        private Rigidbody? _body;
        private Vector3 _startPosition;
        private Vector3 _pusherTarget;
        private string _stepLabel = "coarse";
        private float _elapsed;
        private float _lastSpeed;
        private float _bumperIncomingSpeed;
        private bool _tunneled;
        private bool _containmentReported;
        private bool _carryReported;
        private bool _bumperHit;
        private bool _isPusher;

        private new void Awake()
        {
            _body = GetComponent<Rigidbody>();
            switch (gameObject.name)
            {
                case "CorridorMarble":
                    launchSpeed = 30f;
                    containmentMaxZ = 20f;
                    break;
                case "BumperMarble":
                    launchSpeed = 12f;
                    break;
                case "PlatformMarble":
                    carryReportSeconds = 2f;
                    break;
                case "EdgeMarble":
                    launchSpeed = 30f;
                    bumperName = "ArenaBumper";
                    break;
                case "Pusher":
                    _isPusher = true;
                    _pusherTarget = transform.position;
                    break;
            }
        }

        private new void Start()
        {
            _startPosition = transform.position;
            Rigidbody? body = _body;
            if (body != null && launchSpeed > 0f)
            {
                body.velocity = new Vector3(0f, 0f, launchSpeed);
                Debug.Log($"{gameObject.name} launched at {launchSpeed} m/s");
            }
        }

        private new void Update()
        {
            _stepLabel = Time.deltaTime >= CoarseStepThreshold ? "coarse" : "fine";
        }

        private new void FixedUpdate()
        {
            _elapsed += Time.fixedDeltaTime;
            Rigidbody? body = _body;
            if (body == null)
            {
                return;
            }

            if (_isPusher)
            {
                _pusherTarget = new Vector3(
                    _pusherTarget.x,
                    _pusherTarget.y,
                    _pusherTarget.z + (PusherSpeed * Time.fixedDeltaTime));
                body.MovePosition(_pusherTarget);
            }

            if (containmentMaxZ > 0f)
            {
                ReportContainment();
            }

            if (carryReportSeconds > 0f && !_carryReported && _elapsed >= carryReportSeconds)
            {
                _carryReported = true;
                Vector3 offset = transform.position - _startPosition;
                float carried = Mathf.Sqrt((offset.x * offset.x) + (offset.z * offset.z));
                string verdict = carried >= CarryDistanceThreshold ? "carried" : "carry stalled";
                Debug.Log($"{gameObject.name} platform {verdict} marble {carried} m");
            }

            if (_bumperHit)
            {
                _bumperHit = false;
                float outgoing = body.velocity.magnitude;
                float ratio = _bumperIncomingSpeed > 0f ? outgoing / _bumperIncomingSpeed : 0f;
                string verdict = outgoing > _bumperIncomingSpeed ? "restitution gained" : "restitution lost";
                Debug.Log($"{gameObject.name} bumper outgoing {outgoing} incoming {_bumperIncomingSpeed} ratio {ratio:F3}: {verdict}");
            }

            _lastSpeed = body.velocity.magnitude;
        }

        private new void OnCollisionEnter(Collision collision)
        {
            GameObject? other = collision.gameObject;
            string otherName = other != null ? other.name : "unknown";
            if (other != null && other.name == bumperName)
            {
                _bumperHit = true;
                _bumperIncomingSpeed = _lastSpeed;
            }

            int contacts = collision.contactCount;
            if (contacts <= 0)
            {
                LogContact(otherName, collision.impulse, collision.relativeVelocity, 0, collision.point, collision.normal);
                return;
            }

            for (int index = 0; index < contacts; index++)
            {
                ContactPoint contact = collision.GetContact(index);
                LogContact(otherName, collision.impulse, collision.relativeVelocity, contacts, contact.point, contact.normal);
            }
        }

        private void LogContact(string otherName, float impulse, Vector3 relative, int contacts, Vector3 point, Vector3 normal)
        {
            string verdict = impulse > 0f ? "non-zero impulse" : "zero impulse";
            Debug.Log($"{gameObject.name} contact {otherName} {verdict} impulse {impulse} relative {relative.magnitude} contacts {contacts} point {point.x} {point.y} {point.z} normal {normal.x} {normal.y} {normal.z}");
        }

        private void ReportContainment()
        {
            if (!_tunneled && transform.position.z > containmentMaxZ)
            {
                _tunneled = true;
                Debug.LogWarning($"{gameObject.name} tunneled past containment at z {transform.position.z}");
            }

            if (!_tunneled && !_containmentReported && _elapsed >= ContainmentReportSeconds)
            {
                _containmentReported = true;
                Debug.Log($"{gameObject.name} containment held at {_stepLabel} step");
            }
        }
    }
}
