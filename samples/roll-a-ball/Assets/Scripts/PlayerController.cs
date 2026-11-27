using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class PlayerController : MonoBehaviour
    {
        private const float DefaultMoveSpeed = 7.5f;
        private const float DefaultBoostMultiplier = 1.65f;
        private const float DashImpulse = 8f;
        private const float DashCooldownSeconds = 2f;
        private const float DashCoastSeconds = 0.25f;
        private const float CameraYawPerPixel = 0.005f;
        private const float MinimumCameraDistance = 1f;
        private const float TrailEmissionSpeed = 10f;
        private static readonly Vector3 StartPosition = new Vector3(0f, 0.5f, 0f);
        private static readonly Vector3 CameraOffset = new Vector3(0f, 9.5f, -11.5f);

        [SerializeField]
        private float moveSpeed = DefaultMoveSpeed;

        [SerializeField]
        private float boostMultiplier = DefaultBoostMultiplier;

        private Rigidbody? _body;
        private Camera? _camera;
        private TrailRenderer? _trail;
        private float _cameraYaw;
        private float _lastDashTime = float.NegativeInfinity;
        private int _dashCount;
        private bool _reducedMotionTrailOff;

        private new void Awake()
        {
            _body = GetComponent<Rigidbody>();
            _trail = GetComponent<TrailRenderer>();
            GameObject? cameraObject = GameObject.Find("Main Camera");
            if (cameraObject != null)
            {
                _camera = cameraObject.GetComponent<Camera>();
            }
        }

        private new void Update()
        {
            float moveX = ReadAxis(KeyCode.D, KeyCode.RightArrow, KeyCode.A, KeyCode.LeftArrow);
            float moveZ = ReadAxis(KeyCode.W, KeyCode.UpArrow, KeyCode.S, KeyCode.DownArrow);

            if (moveX == 0f)
            {
                moveX = Input.GetInjectedAxis("Horizontal");
            }
            if (moveZ == 0f)
            {
                moveZ = Input.GetInjectedAxis("Vertical");
            }

            _cameraYaw += Input.mousePositionDelta.x * CameraYawPerPixel;

            float speed = moveSpeed * (Input.GetMouseButton(0) ? boostMultiplier : 1f);
            float yaw = _cameraYaw;
            float directionX = (moveX * Mathf.Cos(yaw)) + (moveZ * Mathf.Sin(yaw));
            float directionZ = (moveZ * Mathf.Cos(yaw)) - (moveX * Mathf.Sin(yaw));

            Rigidbody? body = _body;
            if (body == null)
            {
                return;
            }

            if (Time.time - _lastDashTime >= DashCoastSeconds)
            {
                body.velocity = new Vector3(directionX * speed, body.velocity.y, directionZ * speed);
            }
            TryDash(body, directionX, directionZ);

            TrailRenderer? trail = _trail;
            if (trail != null)
            {
                if (ReducedMotion.IsEnabled)
                {
                    // The speed trail is the fast, sweeping motion the setting removes.
                    if (!_reducedMotionTrailOff)
                    {
                        _reducedMotionTrailOff = true;
                        Debug.Log("Reduced motion active trail off");
                    }

                    trail.emitting = false;
                }
                else
                {
                    trail.emitting = body.velocity.magnitude >= TrailEmissionSpeed;
                }
            }
        }

        private void TryDash(Rigidbody body, float directionX, float directionZ)
        {
            if (!Input.GetKeyDown(KeyCode.Space))
            {
                return;
            }

            if (Time.time - _lastDashTime < DashCooldownSeconds)
            {
                Debug.Log("Dash blocked cooldown");
                return;
            }

            Vector3 dashDirection = new Vector3(directionX, 0f, directionZ);
            if (dashDirection.sqrMagnitude < Vector3.kEpsilon)
            {
                Vector3 velocity = body.velocity;
                dashDirection = new Vector3(velocity.x, 0f, velocity.z);
                if (dashDirection.sqrMagnitude < Vector3.kEpsilon)
                {
                    dashDirection = new Vector3(Mathf.Sin(_cameraYaw), 0f, Mathf.Cos(_cameraYaw));
                }
            }

            _lastDashTime = Time.time;
            _dashCount++;
            body.AddForce(dashDirection.normalized * DashImpulse, ForceMode.Impulse);
            Debug.Log($"Dash fired {_dashCount}");
        }

        private new void LateUpdate()
        {
            Camera? camera = _camera;
            if (camera == null)
            {
                return;
            }

            Vector3 target = transform.position;
            target.y = Mathf.Max(target.y, StartPosition.y);
            float distance = Mathf.Max(-CameraOffset.z, MinimumCameraDistance);
            Vector3 eye = new Vector3(
                target.x - (Mathf.Sin(_cameraYaw) * distance),
                target.y + CameraOffset.y,
                target.z - (Mathf.Cos(_cameraYaw) * distance));
            camera.transform.position = eye;
            camera.transform.LookAt(target);
        }

        public void ResetPlayer()
        {
            _cameraYaw = 0f;
            _lastDashTime = float.NegativeInfinity;
            _dashCount = 0;
            transform.position = StartPosition;
            Rigidbody? body = _body;
            if (body != null)
            {
                body.velocity = Vector3.zero;
            }
            TrailRenderer? trail = _trail;
            if (trail != null)
            {
                trail.emitting = false;
            }
        }

        private static float ReadAxis(
            KeyCode positiveKey,
            KeyCode positiveAlternate,
            KeyCode negativeKey,
            KeyCode negativeAlternate)
        {
            if (Input.GetKey(positiveKey) || Input.GetKey(positiveAlternate))
            {
                return 1f;
            }

            if (Input.GetKey(negativeKey) || Input.GetKey(negativeAlternate))
            {
                return -1f;
            }

            return 0f;
        }
    }
}
