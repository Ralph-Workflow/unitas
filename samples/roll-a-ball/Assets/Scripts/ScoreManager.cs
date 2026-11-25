using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class ScoreManager : MonoBehaviour
    {
        private const float PickupRadius = 0.45f;
        private const int DefaultTotalPickups = 4;

        [SerializeField]
        private int totalPickups = DefaultTotalPickups;

        private readonly List<PickupSpawn> _spawns = new List<PickupSpawn>();
        private int _score;
        private bool _won;

        public int Score => _score;

        public bool Won => _won;

        public override void OnTriggerEnter(Collider? other)
        {
            if (_won || other == null)
            {
                return;
            }

            Rotator? pickup = other.GetComponent<Rotator>();
            if (pickup == null)
            {
                return;
            }

            GameObject pickupObject = pickup.gameObject;
            _spawns.Add(new PickupSpawn(pickupObject.name, pickupObject.transform.position));
            Destroy(pickupObject);
            _score++;
            if (_score >= totalPickups)
            {
                _won = true;
                Debug.Log($"All {totalPickups} pickups collected, press R to play again.");
            }
        }

        public override void Update()
        {
            if (_won && Input.GetKeyDown(KeyCode.R))
            {
                Restart();
            }
        }

        public void Restart()
        {
            _score = 0;
            _won = false;
            foreach (PickupSpawn spawn in _spawns)
            {
                CreatePickup(spawn.Name, spawn.Position);
            }

            _spawns.Clear();
            PlayerController? controller = GetComponentInParent<PlayerController>();
            if (controller != null)
            {
                controller.ResetPlayer();
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

        private sealed class PickupSpawn
        {
            public PickupSpawn(string name, Vector3 position)
            {
                Name = name;
                Position = position;
            }

            public string Name { get; }

            public Vector3 Position { get; }
        }
    }
}
