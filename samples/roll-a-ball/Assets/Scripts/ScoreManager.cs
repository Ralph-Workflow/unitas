using System.Collections;
using System.Collections.Generic;
using UnityEngine;
namespace Unitas.Samples.RollABall
{
    public class ScoreManager : MonoBehaviour
    {
        private const float PickupRadius = 0.45f;
        private const float PickupSensorRadius = 1.15f;
        private const float CollectRadius = PickupRadius + PickupSensorRadius;
        private const int TotalPickups = 12;
        private const float ComboWindowSeconds = 1.5f;
        private const int MaxMultiplier = 5;
        private const int GoldMultiplierThreshold = 3;
        private const int GoldBasePoints = 5;
        private const float GoldLifetimeSeconds = 4f;
        private const float CountdownStepSeconds = 1f;
        private const float MenuTimerSeconds = 0.5f;
        private const float SlowMotionScale = 0.25f;
        private const float SlowMotionRealtimeSeconds = 0.5f;
        // 1 / SlowMotionScale: the scaled-to-realtime delta ratio while the
        // slow-motion finish is on.
        private const float SlowMotionRatio = 4f;
        private const float SlowMotionRatioTolerance = 0.05f;
        // Frames a pause must last before the marble counts as frozen.
        private const int FrozenProofFrames = 5;
        private const float FrozenToleranceSqr = 1e-8f;
        private static readonly Vector3 GoldSpawnPosition = new Vector3(5f, 0.75f, 5f);
        // Laid out along the sample route (up the middle, counterclockwise
        // around the ring) so a single lap chains the full combo.
        private static readonly Vector3[] PickupPositions =
        {
            new Vector3(0f, 0.75f, 2.25f),
            new Vector3(-2.25f, 0.75f, 4.5f),
            new Vector3(-4.5f, 0.75f, 1.5f),
            new Vector3(-4.5f, 0.75f, -1.5f),
            new Vector3(-4.5f, 0.75f, -4.5f),
            new Vector3(-1.5f, 0.75f, -4.5f),
            new Vector3(1.5f, 0.75f, -4.5f),
            new Vector3(4.5f, 0.75f, -4.5f),
            new Vector3(4.5f, 0.75f, -1.5f),
            new Vector3(4.5f, 0.75f, 1.5f),
            new Vector3(4.5f, 0.75f, 4.5f),
            new Vector3(1.5f, 0.75f, 4.5f),
        };
        private readonly List<GameObject> _pickups = new List<GameObject>();
        // Trigger count per pickup name: a pickup that fires twice is a bug,
        // and the counter is what proves it did not.
        private readonly Dictionary<string, int> _pickupTriggers = new Dictionary<string, int>();
        private PlayerController? _player;
        private GameObject? _goldPickup;
        private Coroutine? _goldExpiry;
        private int _score;
        private int _pickupsCollected;
        private int _multiplier = 1;
        private float _lastPickupTime = float.NegativeInfinity;
        private bool _won;
        private bool _runStarted;
        private bool _countdownRunning;
        private bool _paused;
        private bool _slowMotionActive;
        private bool _slowMotionScaledLogged;
        private bool _frozenLogged;
        private bool _movedWhilePausedLogged;
        private int _pausedFrames;
        private Vector3 _pausedPosition;
        // The scale to restore when the game is unpaused.
        private float _resumeScale = 1f;
        // The scale to restore when the slow-motion finish ends.
        private float _scaleBeforeSlowMotion = 1f;
        public int Score => _score;
        public int Multiplier => _multiplier;
        public bool Won => _won;
        private new void Start()
        {
            _player = GetComponentInParent<PlayerController>();
            // Pickups live in code from here on; drop the scene-authored copies.
            for (int i = 1; i <= 4; i++)
            {
                GameObject? scenePickup = GameObject.Find($"Pickup_{i}");
                if (scenePickup != null)
                {
                    Destroy(scenePickup);
                }
            }
            SpawnAllPickups();
        }
        // Collection is a managed distance check: runtime-spawned colliders
        // never reach the native physics world, so OnTriggerEnter cannot fire
        // for code-spawned pickups. Same overlap semantics, fully deterministic.
        private new void Update()
        {
            if (Input.GetKeyDown(KeyCode.R))
            {
                Restart();
                return;
            }
            if (Input.GetKeyDown(KeyCode.Escape) || Input.GetKeyDown(KeyCode.P))
            {
                TogglePause();
            }
            if (Input.GetKeyDown(KeyCode.Return) && !_runStarted && !_countdownRunning)
            {
                _countdownRunning = true;
                StartCoroutine(CountdownRoutine());
            }
            if (_multiplier > 1 && Time.time - _lastPickupTime > ComboWindowSeconds)
            {
                _multiplier = 1;
                Debug.Log("Combo expired");
            }
            if (_paused)
            {
                CheckFrozenWhilePaused();
            }
            ReportSlowMotionScale();
            CollectNearbyPickups();
        }
        public void Restart()
        {
            StopGoldExpiry();
            foreach (GameObject pickup in _pickups)
            {
                Destroy(pickup);
            }
            _pickups.Clear();
            if (_goldPickup != null)
            {
                Destroy(_goldPickup);
                _goldPickup = null;
            }
            _pickupTriggers.Clear();
            _score = 0;
            _pickupsCollected = 0;
            _multiplier = 1;
            _lastPickupTime = float.NegativeInfinity;
            _won = false;
            SpawnAllPickups();
            PlayerController? controller = GetComponentInParent<PlayerController>();
            if (controller != null)
            {
                controller.ResetPlayer();
            }
            Debug.Log("Retry complete score 0 pickups 12 multiplier x1");
        }
        private void CollectNearbyPickups()
        {
            PlayerController? player = _player;
            if (_won || player == null)
            {
                return;
            }
            Vector3 playerPosition = player.transform.position;
            for (int i = _pickups.Count - 1; i >= 0; i--)
            {
                GameObject pickup = _pickups[i];
                // A destroyed pickup compares equal to null, so the guard below is
                // what keeps it from being collected or triggered a second time.
                if (pickup == null)
                {
                    continue;
                }
                if (Vector3.Distance(pickup.transform.position, playerPosition) > CollectRadius)
                {
                    continue;
                }
                CountTrigger(pickup);
                DestroyPickup(pickup);
                AwardPickup();
                if (_won)
                {
                    return;
                }
            }
            if (_goldPickup != null
                && Vector3.Distance(_goldPickup.transform.position, playerPosition) <= CollectRadius)
            {
                CollectGold(_goldPickup);
            }
        }
        private void AwardPickup()
        {
            if (Time.time - _lastPickupTime <= ComboWindowSeconds)
            {
                if (_multiplier < MaxMultiplier)
                {
                    _multiplier++;
                }
            }
            else
            {
                _multiplier = 1;
            }
            _lastPickupTime = Time.time;
            _score += _multiplier;
            _pickupsCollected++;
            Debug.Log($"Pickup collected score {_pickupsCollected}");
            Debug.Log($"Combo multiplier x{_multiplier}");
            Debug.Log($"Score +{_multiplier} total {_score}");
            if (_multiplier == GoldMultiplierThreshold && _goldPickup == null)
            {
                SpawnGoldPickup();
            }
            if (_pickupsCollected >= TotalPickups)
            {
                _won = true;
                Debug.Log($"All {TotalPickups} pickups collected, press R to play again.");
                if (AllPickupsTriggeredOnce())
                {
                    Debug.Log("Pickup triggers once each");
                }
                if (_runStarted)
                {
                    EngageSlowMotion();
                }
            }
        }
        private void CollectGold(GameObject goldObject)
        {
            StopGoldExpiry();
            _goldPickup = null;
            CountTrigger(goldObject);
            DestroyPickup(goldObject);
            int award = GoldBasePoints * _multiplier;
            _score += award;
            _lastPickupTime = Time.time;
            Debug.Log("Gold pickup collected");
            Debug.Log($"Score +{award} total {_score}");
        }
        private void SpawnGoldPickup()
        {
            GameObject gold = CreatePickup("Pickup_Gold", GoldSpawnPosition);
            _goldPickup = gold;
            Debug.Log("Gold pickup spawned");
            // The window is 4 seconds of scaled time, so the coroutine waits on
            // scaled time (WaitForSeconds) and freezes with the game.
            _goldExpiry = StartCoroutine(GoldExpiryRoutine(gold));
        }
        private IEnumerator GoldExpiryRoutine(GameObject gold)
        {
            yield return new WaitForSeconds(GoldLifetimeSeconds);
            if (gold == null || _goldPickup != gold)
            {
                yield break;
            }
            _goldPickup = null;
            Destroy(gold);
            Debug.Log("Gold pickup expired");
        }
        private void StopGoldExpiry()
        {
            if (_goldExpiry != null)
            {
                StopCoroutine(_goldExpiry);
                _goldExpiry = null;
            }
        }
        private void CountTrigger(GameObject pickup)
        {
            string pickupName = pickup.name;
            int triggers = _pickupTriggers.TryGetValue(pickupName, out int previous) ? previous + 1 : 1;
            _pickupTriggers[pickupName] = triggers;
            if (triggers > 1)
            {
                Debug.Log($"{pickupName} triggered twice");
            }
        }
        private bool AllPickupsTriggeredOnce()
        {
            foreach (KeyValuePair<string, int> entry in _pickupTriggers)
            {
                if (entry.Value != 1)
                {
                    return false;
                }
            }
            return true;
        }
        private void DestroyPickup(GameObject pickup)
        {
            // The pickup reports its own collect burst so the log fires once,
            // in collection order, at every frame rate.
            Pickup? component = pickup.GetComponent<Pickup>();
            if (component != null)
            {
                component.ReportCollectBurst();
            }
            Destroy(pickup);
            // Unity's fake null: a destroyed object compares equal to null.
            bool destroyedReadsNull = pickup == null;
            Debug.Log($"Pickup destroyed null {destroyedReadsNull}");
        }
        // The countdown runs on scaled time, so it freezes with the game.
        private IEnumerator CountdownRoutine()
        {
            yield return new WaitForSeconds(CountdownStepSeconds);
            Debug.Log("Countdown 3");
            yield return new WaitForSeconds(CountdownStepSeconds);
            Debug.Log("Countdown 2");
            yield return new WaitForSeconds(CountdownStepSeconds);
            Debug.Log("Countdown 1");
            _countdownRunning = false;
            _runStarted = true;
            Debug.Log("Run started");
        }
        private void TogglePause()
        {
            if (_paused)
            {
                _paused = false;
                Time.timeScale = _resumeScale;
                Debug.Log("Game resumed");
                return;
            }
            _resumeScale = Time.timeScale;
            _paused = true;
            _pausedFrames = 0;
            _frozenLogged = false;
            _movedWhilePausedLogged = false;
            PlayerController? player = _player;
            _pausedPosition = player != null ? player.transform.position : Vector3.zero;
            Time.timeScale = 0f;
            Debug.Log("Game paused");
            // The menu timer runs on real time: it elapses while the scaled
            // clock is frozen.
            StartCoroutine(MenuTimerRoutine());
        }
        // While the game is paused the physics does not step, so the marble
        // must not move: the position is compared against the paused one.
        private void CheckFrozenWhilePaused()
        {
            _pausedFrames++;
            PlayerController? player = _player;
            if (player == null)
            {
                return;
            }
            Vector3 current = player.transform.position;
            if ((current - _pausedPosition).sqrMagnitude > FrozenToleranceSqr)
            {
                if (!_movedWhilePausedLogged)
                {
                    _movedWhilePausedLogged = true;
                    Debug.Log("Marble moved while paused");
                }
                return;
            }
            if (_frozenLogged || _pausedFrames < FrozenProofFrames)
            {
                return;
            }
            _frozenLogged = true;
            Debug.Log("Marble frozen while paused");
        }
        private IEnumerator MenuTimerRoutine()
        {
            yield return new WaitForSecondsRealtime(MenuTimerSeconds);
            Debug.Log("Menu timer elapsed");
        }
        private void EngageSlowMotion()
        {
            if (_paused)
            {
                return;
            }
            _scaleBeforeSlowMotion = Time.timeScale;
            _slowMotionActive = true;
            _slowMotionScaledLogged = false;
            Time.timeScale = SlowMotionScale;
            Debug.Log("Slow motion engaged");
            StartCoroutine(SlowMotionRoutine());
        }
        private IEnumerator SlowMotionRoutine()
        {
            yield return new WaitForSecondsRealtime(SlowMotionRealtimeSeconds);
            _slowMotionActive = false;
            if (_paused)
            {
                _resumeScale = _scaleBeforeSlowMotion;
            }
            else
            {
                Time.timeScale = _scaleBeforeSlowMotion;
            }
            Debug.Log("Time restored");
        }
        private void ReportSlowMotionScale()
        {
            if (!_slowMotionActive || _slowMotionScaledLogged)
            {
                return;
            }
            float scaledDelta = Time.deltaTime;
            if (scaledDelta <= 0f)
            {
                return;
            }
            float ratio = Time.unscaledDeltaTime / scaledDelta;
            if (Mathf.Abs(ratio - SlowMotionRatio) > SlowMotionRatioTolerance)
            {
                return;
            }
            _slowMotionScaledLogged = true;
            Debug.Log("Slow motion scaled");
        }
        private void SpawnAllPickups()
        {
            for (int i = 0; i < PickupPositions.Length; i++)
            {
                _pickups.Add(CreatePickup($"Pickup_{i + 1}", PickupPositions[i]));
            }
        }
        private static GameObject CreatePickup(string name, Vector3 position)
        {
            GameObject pickup = new GameObject(name);
            pickup.transform.position = position;
            SphereCollider trigger = pickup.AddComponent<SphereCollider>();
            trigger.isTrigger = true;
            trigger.radius = PickupRadius;
            pickup.AddComponent<Rotator>();
            return pickup;
        }
    }
}
