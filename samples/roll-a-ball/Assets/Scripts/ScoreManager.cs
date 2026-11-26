using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class ScoreManager : MonoBehaviour
    {
        private const float PickupRadius = 0.45f;
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
        private const float SlowMotionRatio = 4f;
        private const float SlowMotionRatioTolerance = 0.05f;
        private const int FrozenProofFrames = 5;
        private const float FrozenToleranceSqr = 1e-8f;
        private const string GoldPickupName = "Pickup_Gold";

        private static readonly Vector3 GoldSpawnPosition = new Vector3(5f, 0.75f, 5f);

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
        private readonly Dictionary<string, int> _pickupTriggers = new Dictionary<string, int>();
        private readonly List<Vector3> _route = new List<Vector3>();

        private PlayerController? _player;
        private GameObject? _goldPickup;
        private Coroutine? _goldExpiry;
        private int _score;
        private int _pickupsCollected;
        private int _multiplier = 1;
        private float _lastPickupTime = float.NegativeInfinity;
        private float _runStartTime;
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
        private float _resumeScale = 1f;
        private float _scaleBeforeSlowMotion = 1f;
        private bool _runRecorded;

        public int Score => _score;

        public int Multiplier => _multiplier;

        public bool Won => _won;

        public float RunTime => _runStarted ? Time.time - _runStartTime : 0f;

        private new void Start()
        {
            _player = GetComponentInParent<PlayerController>();
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
            if (Input.GetKeyDown(KeyCode.Return))
            {
                StartRun();
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
            RecordFinishedRun();
        }

        private new void FixedUpdate()
        {
            // The ghost holds the ball's real position at every fixed step of the run.
            if (_runRecorded || !_runStarted)
            {
                return;
            }

            PlayerController? player = _player;
            if (player == null)
            {
                return;
            }

            _route.Add(player.transform.position);
        }

        private new void OnTriggerEnter(Collider? other)
        {
            if (_won || other == null)
            {
                return;
            }
            GameObject pickup = other.gameObject;
            if (pickup == null)
            {
                return;
            }
            if (_goldPickup != null && ReferenceEquals(pickup, _goldPickup))
            {
                CollectGold(pickup);
                return;
            }
            if (!_pickups.Contains(pickup))
            {
                return;
            }
            CountTrigger(pickup);
            DestroyPickup(pickup);
            AwardPickup();
        }

        public void StartRun()
        {
            if (_runStarted || _countdownRunning)
            {
                return;
            }
            _countdownRunning = true;
            StartCoroutine(CountdownRoutine());
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
                Debug.Log("Gold pickup removed");
            }
            _pickupTriggers.Clear();
            _route.Clear();
            _runRecorded = false;
            _score = 0;
            _pickupsCollected = 0;
            _multiplier = 1;
            _lastPickupTime = float.NegativeInfinity;
            _won = false;
            _runStartTime = Time.time;
            SpawnAllPickups();
            PlayerController? controller = GetComponentInParent<PlayerController>();
            if (controller != null)
            {
                controller.ResetPlayer();
            }
            Debug.Log($"Retry complete score {_score} pickups {_pickups.Count} multiplier x{_multiplier} timer {FormatSeconds(RunTime)}");
        }

        private void RecordFinishedRun()
        {
            if (_runRecorded || !_won)
            {
                return;
            }

            _runRecorded = true;
            SavedState.FinishRun(_route, _route.Count * Time.fixedDeltaTime, _score);
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
            GameObject gold = CreatePickup(GoldPickupName, GoldSpawnPosition);
            _goldPickup = gold;
            Debug.Log("Gold pickup spawned");
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
            Pickup? component = pickup.GetComponent<Pickup>();
            if (component != null)
            {
                component.ReportCollectBurst();
            }
            Destroy(pickup);
            bool destroyedReadsNull = pickup == null;
            Debug.Log($"Pickup destroyed null {destroyedReadsNull}");
        }

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
            _runStartTime = Time.time;
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
            StartCoroutine(MenuTimerRoutine());
        }

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
            pickup.AddComponent<Pickup>();
            pickup.AddComponent<Rotator>();
            return pickup;
        }

        private static string FormatSeconds(float seconds)
        {
            return seconds.ToString("F3", CultureInfo.InvariantCulture);
        }
    }
}
