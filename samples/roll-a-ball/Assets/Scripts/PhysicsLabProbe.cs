using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class PhysicsLabProbe : MonoBehaviour
    {
        private const float CoarseStepThreshold = 0.015f;
        private const float ContainmentReportSeconds = 1.5f;
        private const float CarryDistanceThreshold = 0.5f;

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
        private string _stepLabel = "coarse";
        private float _elapsed;
        private float _lastSpeed;
        private float _bumperIncomingSpeed;
        private bool _tunneled;
        private bool _containmentReported;
        private bool _carryReported;
        private bool _bumperHit;

        public override void Awake()
        {
            _body = GetComponent<Rigidbody>();
            // The play pipeline does not apply scene field overrides yet, so each
            // lab marble is configured by object name; the scene values mirror these.
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
            }
        }

        public override void Start()
        {
            _startPosition = transform.position;
            Rigidbody? body = _body;
            if (body != null && launchSpeed > 0f)
            {
                body.velocity = new Vector3(0f, 0f, launchSpeed);
                Debug.Log($"{gameObject.name} launched at {launchSpeed} m/s");
            }
        }

        public override void Update()
        {
            _stepLabel = Time.deltaTime >= CoarseStepThreshold ? "coarse" : "fine";
        }

        public override void FixedUpdate()
        {
            _elapsed += Time.fixedDeltaTime;
            Rigidbody? body = _body;
            if (body == null)
            {
                return;
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
                string verdict = outgoing > _bumperIncomingSpeed ? "restitution gained" : "restitution lost";
                Debug.Log($"{gameObject.name} bumper outgoing {outgoing} incoming {_bumperIncomingSpeed}: {verdict}");
            }

            _lastSpeed = body.velocity.magnitude;
        }

        public override void OnCollisionEnter(Collision collision)
        {
            GameObject? other = collision.gameObject;
            if (other != null && other.name == bumperName)
            {
                _bumperHit = true;
                _bumperIncomingSpeed = _lastSpeed;
            }
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
