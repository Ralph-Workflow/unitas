using System;
using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public class ArenaController : MonoBehaviour
    {
        private const string ClockworkScene = "Arena2";
        private const string SkyGardenScene = "Arena3";
        private const int PickupCount = 12;
        private const float PickupHeight = 0.75f;
        private const float ClockworkRadius = 3.75f;
        private const float BlockSpeed = 2.5f;
        private const float BlockTravel = 2f;
        private const string MovingBlockPrefix = "MovingBlock_";

        private static readonly Vector3[] SkyGardenPath =
        {
            new Vector3(0f, PickupHeight, 2f),
            new Vector3(0f, PickupHeight, 3f),
            new Vector3(0f, PickupHeight, 4f),
            new Vector3(2f, PickupHeight, 4f),
            new Vector3(3f, PickupHeight, 4f),
            new Vector3(4f, PickupHeight, 4f),
            new Vector3(5f, PickupHeight, 4f),
            new Vector3(5f, PickupHeight, 3f),
            new Vector3(5f, PickupHeight, 2f),
            new Vector3(5f, PickupHeight, 1f),
            new Vector3(5f, PickupHeight, 0f),
            new Vector3(5f, PickupHeight, -1f),
        };

        private readonly GameObject?[] _pickups = new GameObject?[PickupCount];
        private readonly List<Rigidbody> _blocks = new List<Rigidbody>();
        private readonly List<float> _blockOrigins = new List<float>();
        private readonly List<float> _blockDirections = new List<float>();
        private string _arena = string.Empty;
        private bool _running;

        private new void Start()
        {
            _arena = gameObject.scene.name;
            _running = string.Equals(_arena, ClockworkScene, StringComparison.Ordinal)
                || string.Equals(_arena, SkyGardenScene, StringComparison.Ordinal);
            if (!_running)
            {
                return;
            }

            if (string.Equals(_arena, SkyGardenScene, StringComparison.Ordinal))
            {
                CollectMovingBlocks();
            }

            Debug.Log($"[roll-a-ball] arena {_arena} pickups {PickupCount}");
        }

        private new void FixedUpdate()
        {
            if (!_running)
            {
                return;
            }

            DriveMovingBlocks();
        }

        private new void LateUpdate()
        {
            if (!_running)
            {
                return;
            }

            PlacePickups();
        }

        private void PlacePickups()
        {
            for (int index = 0; index < PickupCount; index++)
            {
                GameObject? pickup = _pickups[index];
                if (pickup == null)
                {
                    pickup = GameObject.Find("Pickup_" + (index + 1).ToString(CultureInfo.InvariantCulture));
                    _pickups[index] = pickup;
                }

                if (pickup == null)
                {
                    continue;
                }

                pickup.transform.position = PickupPosition(index);
            }
        }

        private Vector3 PickupPosition(int index)
        {
            if (string.Equals(_arena, SkyGardenScene, StringComparison.Ordinal))
            {
                return SkyGardenPath[index];
            }

            float angle = index * Mathf.PI * 2f / PickupCount;
            return new Vector3(
                Mathf.Sin(angle) * ClockworkRadius,
                PickupHeight,
                Mathf.Cos(angle) * ClockworkRadius);
        }

        private void CollectMovingBlocks()
        {
            Rigidbody[] bodies = UnityEngine.Object.FindObjectsOfType<Rigidbody>();
            for (int index = 0; index < bodies.Length; index++)
            {
                Rigidbody body = bodies[index];
                if (body == null || !body.gameObject.name.StartsWith(MovingBlockPrefix, StringComparison.Ordinal))
                {
                    continue;
                }

                _blocks.Add(body);
                _blockOrigins.Add(body.transform.position.x);
                _blockDirections.Add(1f);
            }
        }

        private void DriveMovingBlocks()
        {
            for (int index = 0; index < _blocks.Count; index++)
            {
                Rigidbody block = _blocks[index];
                if (block == null)
                {
                    continue;
                }

                float origin = _blockOrigins[index];
                float direction = _blockDirections[index];
                Vector3 currentPosition = block.transform.position;
                float position = currentPosition.x;
                if (position >= origin + BlockTravel)
                {
                    direction = -1f;
                }
                else if (position <= origin - BlockTravel)
                {
                    direction = 1f;
                }

                _blockDirections[index] = direction;
                float nextX = position + (direction * BlockSpeed * Time.fixedDeltaTime);
                block.MovePosition(new Vector3(nextX, currentPosition.y, currentPosition.z));
            }
        }
    }
}
