using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class PlayerController : MonoBehaviour
    {
        private const float DefaultMoveSpeed = 7.5f;
        private const float DefaultBoostMultiplier = 1.65f;
        private const float DefaultJumpImpulse = 4.2f;
        private const float CameraYawPerPixel = 0.005f;
        private const float GroundCheckDistance = 0.6f;
        private const float GroundCheckOffset = 0.5f;
        private const float MinimumCameraDistance = 1f;
        private const int AllLayers = -1;
        private static readonly Vector3 StartPosition = new Vector3(0f, 0.5f, 0f);
        private static readonly Vector3 CameraOffset = new Vector3(0f, 9.5f, -11.5f);

        [SerializeField]
        private float moveSpeed = DefaultMoveSpeed;

        [SerializeField]
        private float boostMultiplier = DefaultBoostMultiplier;

        [SerializeField]
        private float jumpImpulse = DefaultJumpImpulse;

        private Rigidbody? _body;
        private Camera? _camera;
        private float _cameraYaw;

        public override void Awake()
        {
            _body = GetComponent<Rigidbody>();
            GameObject? cameraObject = GameObject.Find("Main Camera");
            if (cameraObject != null)
            {
                _camera = cameraObject.GetComponent<Camera>();
            }
        }

        public override void Update()
        {
            float moveX = ReadAxis("Horizontal", KeyCode.D, KeyCode.RightArrow, KeyCode.A, KeyCode.LeftArrow);
            float moveZ = ReadAxis("Vertical", KeyCode.W, KeyCode.UpArrow, KeyCode.S, KeyCode.DownArrow);

            _cameraYaw += Input.mousePositionDelta.x * CameraYawPerPixel;

            float speed = moveSpeed * (Input.GetMouseButton(0) ? boostMultiplier : 1f);
            float yaw = _cameraYaw;
            float directionX = (moveX * Mathf.Cos(yaw)) + (moveZ * Mathf.Sin(yaw));
            float directionZ = (moveZ * Mathf.Cos(yaw)) - (moveX * Mathf.Sin(yaw));

            Rigidbody? body = _body;
            if (body != null)
            {
                body.velocity = new Vector3(directionX * speed, body.velocity.y, directionZ * speed);
            }
        }

        public override void FixedUpdate()
        {
            Rigidbody? body = _body;
            if (body == null || !Input.GetKeyDown(KeyCode.Space) || !IsGrounded())
            {
                return;
            }

            body.velocity = new Vector3(body.velocity.x, jumpImpulse, body.velocity.z);
        }

        public override void LateUpdate()
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
            transform.position = StartPosition;
            Rigidbody? body = _body;
            if (body != null)
            {
                body.velocity = Vector3.zero;
            }
        }

        private static float ReadAxis(
            string axisName,
            KeyCode positiveKey,
            KeyCode positiveAlternate,
            KeyCode negativeKey,
            KeyCode negativeAlternate)
        {
            float value = Input.GetAxis(axisName);
            if (Input.GetKey(positiveKey) || Input.GetKey(positiveAlternate))
            {
                value = Mathf.Max(value, 1f);
            }

            if (Input.GetKey(negativeKey) || Input.GetKey(negativeAlternate))
            {
                value = Mathf.Min(value, -1f);
            }

            return value;
        }

        private bool IsGrounded()
        {
            Vector3 origin = transform.position + (Vector3.down * GroundCheckOffset);
            return Physics.Raycast(
                origin,
                Vector3.down,
                out _,
                GroundCheckDistance,
                AllLayers,
                QueryTriggerInteraction.Ignore);
        }
    }
}
