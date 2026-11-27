using System;
using UnityEngine;

namespace Unitas.Samples.Platformer2D
{
    public class PlayerController2D : MonoBehaviour
    {
        private const float RunSpeed = 6f;
        private const float RunAcceleration = 60f;
        private const float JumpVelocity = 14.353f;
        private const float MaxFallSpeed = 18f;
        private const float EarlyReleaseCutFactor = 0.4f;
        private const float CoyoteWindowSeconds = 0.1f;
        private const float JumpBufferWindowSeconds = 0.1f;
        private const float ChainWindowSeconds = 3f;
        private const float RespawnFadeSeconds = 0.25f;
        private const int BaseEmberPoints = 100;
        private const int MaxChainMultiplier = 4;
        private const int StartingHearts = 3;
        private const int GroundLayerMask = (1 << 0) | (1 << 6) | (1 << 8);

        [SerializeField]
        private float moveSpeed = RunSpeed;

        [SerializeField]
        private float jumpVelocity = JumpVelocity;

        private Rigidbody2D? _body;
        private Vector2 _respawnPosition;
        private float _coyoteTimer;
        private float _jumpBufferTimer;
        private float _chainTimer;
        private float _respawnTimer;
        private int _embersCollected;
        private int _chainMultiplier;
        private int _glowScore;
        private int _heartsRemaining = StartingHearts;
        private bool _jumpHeldLastStep;
        private bool _jumpCutApplied;
        private bool _goalReached;

        private new void Awake()
        {
            _body = GetComponent<Rigidbody2D>();
            Vector3 initial = transform.position;
            _respawnPosition = new Vector2(initial.x, initial.y);
            Debug.Log("Wick lantern ready");
        }

        private new void Update()
        {
            if (Time.timeScale <= 0f || _goalReached)
            {
                return;
            }

            float dt = Time.deltaTime;
            if (_chainTimer > 0f)
            {
                _chainTimer = Mathf.Max(0f, _chainTimer - dt);
                if (_chainTimer <= 0f)
                {
                    _chainMultiplier = 0;
                }
            }

            if (IsJumpPressed())
            {
                if (!_jumpHeldLastStep)
                {
                    _jumpBufferTimer = JumpBufferWindowSeconds;
                }
                _jumpHeldLastStep = true;
            }
            else
            {
                _jumpHeldLastStep = false;
            }
        }

        private new void FixedUpdate()
        {
            Rigidbody2D? body = _body;
            if (body == null || _goalReached)
            {
                return;
            }

            float dt = Time.fixedDeltaTime;
            if (_respawnTimer > 0f)
            {
                UpdateRespawnPause(body, dt);
                return;
            }

            UpdateTimersAndGround(dt);
            Vector2 velocity = body.velocity;
            float moveX = ReadHorizontalInput();
            float targetVx = moveX * moveSpeed;
            float nextVx = MoveTowards(velocity.x, targetVx, RunAcceleration * dt);
            float nextVy = ComputeVerticalVelocity(velocity.y);
            body.velocity = new Vector2(nextVx, nextVy);
        }

        private void UpdateRespawnPause(Rigidbody2D body, float dt)
        {
            _respawnTimer = Mathf.Max(0f, _respawnTimer - dt);
            if (_respawnTimer <= 0f)
            {
                body.MovePosition(_respawnPosition);
                body.velocity = Vector2.zero;
                body.simulated = true;
                Debug.Log($"Wick respawned at ({_respawnPosition.x:F1}, {_respawnPosition.y:F1})");
            }
        }

        private void UpdateTimersAndGround(float dt)
        {
            if (CheckGrounded())
            {
                _coyoteTimer = CoyoteWindowSeconds;
                _jumpCutApplied = false;
            }
            else
            {
                _coyoteTimer = Mathf.Max(0f, _coyoteTimer - dt);
            }

            if (_jumpBufferTimer > 0f)
            {
                _jumpBufferTimer = Mathf.Max(0f, _jumpBufferTimer - dt);
            }
        }

        private float ComputeVerticalVelocity(float currentVy)
        {
            float nextVy = currentVy;
            if (_jumpBufferTimer > 0f && _coyoteTimer > 0f)
            {
                nextVy = jumpVelocity;
                _jumpBufferTimer = 0f;
                _coyoteTimer = 0f;
                _jumpCutApplied = false;
                Debug.Log("Wick jumped");
            }
            else if (!IsJumpPressed() && nextVy > 0f && !_jumpCutApplied)
            {
                nextVy *= EarlyReleaseCutFactor;
                _jumpCutApplied = true;
            }

            if (nextVy < -MaxFallSpeed)
            {
                nextVy = -MaxFallSpeed;
            }

            return nextVy;
        }

        private bool CheckGrounded()
        {
            ContactFilter2D filter = new ContactFilter2D
            {
                useTriggers = false,
                useOutsideNormalAngle = true,
                minNormalAngle = 45f,
                maxNormalAngle = 135f,
            };
            Vector3 pos = transform.position;
            RaycastHit2D[] hits = Physics2D.BoxCast(
                new Vector2(pos.x, pos.y + 0.25f),
                new Vector2(0.5f, 0.2f),
                0f,
                new Vector2(0f, -1f),
                1.1f,
                filter,
                GroundLayerMask);
            return hits.Length > 0;
        }

        private new void OnTriggerEnter2D(Collider2D other)
        {
            if (other == null || other.gameObject == null || _goalReached)
            {
                return;
            }

            string otherName = other.gameObject.name;
            if (otherName.StartsWith("Ember", StringComparison.Ordinal))
            {
                CollectEmber(other.gameObject);
            }
            else if (otherName.StartsWith("Checkpoint", StringComparison.Ordinal))
            {
                LightCheckpoint(other.gameObject);
            }
            else if (otherName.StartsWith("Thorn", StringComparison.Ordinal)
                || otherName.StartsWith("Hazard", StringComparison.Ordinal))
            {
                LoseHeart(otherName);
            }
            else if (otherName.StartsWith("GoalBell", StringComparison.Ordinal))
            {
                RingGoalBell();
            }
        }

        private new void OnCollisionEnter2D(Collision2D collision)
        {
            if (collision == null || collision.gameObject == null || _goalReached)
            {
                return;
            }

            string otherName = collision.gameObject.name;
            if (otherName.StartsWith("Thorn", StringComparison.Ordinal)
                || otherName.StartsWith("Hazard", StringComparison.Ordinal))
            {
                LoseHeart(otherName);
            }
        }

        private void CollectEmber(GameObject ember)
        {
            if (!ember.activeSelf)
            {
                return;
            }

            ember.SetActive(false);
            _embersCollected++;
            _chainMultiplier = _chainTimer > 0f
                ? Math.Min(MaxChainMultiplier, _chainMultiplier + 1)
                : 1;
            _chainTimer = ChainWindowSeconds;
            _glowScore += BaseEmberPoints * _chainMultiplier;
            Debug.Log($"Ember collected: {_embersCollected} glow {_glowScore} chain x{_chainMultiplier}");
        }

        private void LightCheckpoint(GameObject checkpoint)
        {
            Vector3 pos = checkpoint.transform.position;
            _respawnPosition = new Vector2(pos.x, pos.y);
            Debug.Log($"Checkpoint lit: {checkpoint.name}");
        }

        private void LoseHeart(string hazardName)
        {
            Rigidbody2D? body = _body;
            if (body == null || _respawnTimer > 0f)
            {
                return;
            }

            _heartsRemaining = Math.Max(0, _heartsRemaining - 1);
            _chainMultiplier = 0;
            _chainTimer = 0f;
            body.velocity = Vector2.zero;
            body.simulated = false;
            _respawnTimer = RespawnFadeSeconds;
            Debug.Log($"Heart lost to {hazardName}: {_heartsRemaining} remaining");
        }

        private void RingGoalBell()
        {
            _goalReached = true;
            Rigidbody2D? body = _body;
            if (body != null)
            {
                body.velocity = Vector2.zero;
            }

            Debug.Log($"Goal bell rung: embers {_embersCollected} glow {_glowScore}");
        }

        private static float ReadHorizontalInput()
        {
            if (Input.GetKey(KeyCode.D) || Input.GetKey(KeyCode.RightArrow))
            {
                return 1f;
            }

            if (Input.GetKey(KeyCode.A) || Input.GetKey(KeyCode.LeftArrow))
            {
                return -1f;
            }

            return Input.GetInjectedAxis("Horizontal");
        }

        private static bool IsJumpPressed()
        {
            return Input.GetKey(KeyCode.Space)
                || Input.GetKey(KeyCode.W)
                || Input.GetKey(KeyCode.UpArrow);
        }

        private static float MoveTowards(float current, float target, float maxDelta)
        {
            if (Mathf.Abs(target - current) <= maxDelta)
            {
                return target;
            }

            return current + (Mathf.Sign(target - current) * maxDelta);
        }
    }
}
