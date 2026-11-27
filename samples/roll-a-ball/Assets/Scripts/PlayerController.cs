using System;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class PlayerController : MonoBehaviour
    {
        private const float DefaultMoveSpeed = 7.5f;
        private const float DefaultBoostMultiplier = 1.65f;
        private const float BoostPadSpeed = 16f;
        private const float BoostCoastSeconds = 0.5f;
        private const float CoarseStepThreshold = 0.015f;
        private const float ArenaContainmentBoundary = 6f;
        private const float DashImpulse = 8f;
        private const float DashCooldownSeconds = 2f;
        private const float DashCoastSeconds = 0.25f;
        private const float CameraYawPerPixel = 0.005f;
        private const float MinimumCameraDistance = 1f;
        private const float TrailEmissionSpeed = 10f;
        private const float CameraShakeSeconds = 0.25f;
        private const float CameraShakeMetres = 0.15f;
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
        private float _lastBoostTime = float.NegativeInfinity;
        private bool _boostContainmentPending;
        private bool _sweeperContactLogged;
        private string _stepLabel = "coarse";
        private int _dashCount;
        private bool _reducedMotionTrailOff;
        private float _shakeTime;
        private int _shakeCount;

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
            if (Time.timeScale <= 0f)
            {
                return;
            }

            _stepLabel = Time.deltaTime >= CoarseStepThreshold ? "coarse" : "fine";

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

            if (Time.time - _lastDashTime >= DashCoastSeconds && Time.time - _lastBoostTime >= BoostCoastSeconds)
            {
                if (_boostContainmentPending)
                {
                    _boostContainmentPending = false;
                    ReportBoostContainment();
                }

                body.velocity = new Vector3(directionX * speed, body.velocity.y, directionZ * speed);
            }
            TryDash(body, directionX, directionZ);
            UpdateTrail(body);
        }

        private void ReportBoostContainment()
        {
            Vector3 position = transform.position;
            if (Mathf.Abs(position.x) <= ArenaContainmentBoundary && Mathf.Abs(position.z) <= ArenaContainmentBoundary)
            {
                Debug.Log($"PlayerBall boost containment held at {_stepLabel} step");
            }
        }

        private void UpdateTrail(Rigidbody body)
        {
            TrailRenderer? trail = _trail;
            if (trail == null)
            {
                return;
            }

            if (ReducedMotion.IsEnabled)
            {
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
            RequestCameraShake("dash");
        }

        private new void OnTriggerEnter(Collider other)
        {
            if (other == null || other.gameObject == null)
            {
                return;
            }

            if (!other.gameObject.name.StartsWith("BoostPad", StringComparison.Ordinal))
            {
                return;
            }

            Rigidbody? body = _body;
            if (body == null)
            {
                return;
            }

            Vector3 forward = other.transform.forward;
            Vector3 planarDirection = new Vector3(forward.x, 0f, forward.z);
            if (planarDirection.sqrMagnitude < Vector3.kEpsilon)
            {
                planarDirection = Vector3.forward;
            }
            else
            {
                planarDirection = planarDirection.normalized;
            }

            _lastBoostTime = Time.time;
            _boostContainmentPending = true;
            body.velocity = new Vector3(planarDirection.x * BoostPadSpeed, body.velocity.y, planarDirection.z * BoostPadSpeed);
            Debug.Log($"Boost pad triggered speed {BoostPadSpeed:F1} m/s");
            UpdateTrail(body);
        }

        private new void OnCollisionEnter(Collision collision)
        {
            string otherName = collision.gameObject.name;
            if (otherName.StartsWith("Bumper", StringComparison.Ordinal)
                || otherName.StartsWith("Wall", StringComparison.Ordinal)
                || otherName.StartsWith("Sweeper", StringComparison.Ordinal)
                || otherName.StartsWith("MovingBlock", StringComparison.Ordinal))
            {
                RequestCameraShake("impact");
            }

            ReportKinematicSweeperContact(collision, otherName);
        }

        private new void OnCollisionStay(Collision collision)
        {
            string otherName = collision.gameObject.name;
            ReportKinematicSweeperContact(collision, otherName);
        }

        private void ReportKinematicSweeperContact(Collision collision, string otherName)
        {
            if (_sweeperContactLogged)
            {
                return;
            }

            if (!otherName.StartsWith("Sweeper", StringComparison.Ordinal)
                && !otherName.StartsWith("MovingBlock", StringComparison.Ordinal))
            {
                return;
            }

            Rigidbody? body = _body;
            float speed = body != null ? body.velocity.magnitude : 0f;
            if (collision.impulse > 0f || speed > 0.1f)
            {
                _sweeperContactLogged = true;
                Debug.Log($"PlayerBall moved by kinematic sweeper {otherName}");
            }
        }

        private void RequestCameraShake(string source)
        {
            if (SettingsController.IsCameraShakeActive)
            {
                _shakeTime = CameraShakeSeconds;
                _shakeCount++;
                Debug.Log($"Camera shake active {source} {_shakeCount}");
                return;
            }

            _shakeTime = 0f;
            string reason = ReducedMotion.IsEnabled ? "reduced_motion" : "camera_shake_off";
            Debug.Log($"Camera shake suppressed {source} {reason}");
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
            if (_shakeTime > 0f)
            {
                if (!SettingsController.IsCameraShakeActive)
                {
                    _shakeTime = 0f;
                }
                else
                {
                    _shakeTime = Mathf.Max(0f, _shakeTime - Time.deltaTime);
                    if (_shakeTime > 0f)
                    {
                        eye += new Vector3(CameraShakeMetres, CameraShakeMetres * 0.5f, 0f);
                    }
                }
            }

            camera.transform.position = eye;
            camera.transform.LookAt(target);
        }

        public void ResetPlayer()
        {
            _cameraYaw = 0f;
            _lastDashTime = float.NegativeInfinity;
            _lastBoostTime = float.NegativeInfinity;
            _boostContainmentPending = false;
            _sweeperContactLogged = false;
            _dashCount = 0;
            _shakeTime = 0f;
            _shakeCount = 0;
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
