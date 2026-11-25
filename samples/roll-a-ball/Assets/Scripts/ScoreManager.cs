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
        private PlayerController? _player;
        private GameObject? _goldPickup;
        private float _goldExpireTime;
        private int _score;
        private int _pickupsCollected;
        private int _multiplier = 1;
        private float _lastPickupTime = float.NegativeInfinity;
        private bool _won;

        public int Score => _score;

        public int Multiplier => _multiplier;

        public bool Won => _won;

        public override void Start()
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
        public override void Update()
        {
            if (Input.GetKeyDown(KeyCode.R))
            {
                Restart();
                return;
            }

            if (_multiplier > 1 && Time.time - _lastPickupTime > ComboWindowSeconds)
            {
                _multiplier = 1;
                Debug.Log("Combo expired");
            }

            if (_goldPickup != null && Time.time >= _goldExpireTime)
            {
                Destroy(_goldPickup);
                _goldPickup = null;
                Debug.Log("Gold pickup expired");
            }

            CollectNearbyPickups();
        }

        public void Restart()
        {
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
                if (Vector3.Distance(pickup.transform.position, playerPosition) > CollectRadius)
                {
                    continue;
                }

                _pickups.RemoveAt(i);
                Destroy(pickup);
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
            }
        }

        private void CollectGold(GameObject goldObject)
        {
            _goldPickup = null;
            Destroy(goldObject);
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
            _goldExpireTime = Time.time + GoldLifetimeSeconds;
            Debug.Log("Gold pickup spawned");
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
