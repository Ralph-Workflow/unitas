using System;
using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class TransformPositionSetterTests
    {
        public static void Run()
        {
            PresentRigidbodyStillNotifiedOnEverySet();
            RigidbodyAddedLaterIsPickedUp();
            DestroyedRigidbodyStopsNotifications();
        }

        private static void PresentRigidbodyStillNotifiedOnEverySet()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            Physics.Provider = provider;
            try
            {
                GameObject owner = new GameObject("setter-hot");
                owner.AssignNativeEntityId(7);
                owner.AddComponent<Rigidbody>();
                Transform transform = owner.transform;

                transform.position = new Vector3(1f, 2f, 3f);
                transform.position = new Vector3(4f, 5f, 6f);
                transform.position = new Vector3(7f, 8f, 9f);

                Check.Equal("every position set reaches the Rigidbody body", 3, provider.SetBodyPositionCalls.Count);
                Check.Equal("body notified with the entity id", "7", provider.SetBodyPositionCalls[0].Entity);
                Check.Equal("last notification carries the latest position", "7:7,8,9", provider.SetBodyPositionCalls[2].Call);
            }
            finally
            {
                Physics.Provider = previous;
            }
        }

        private static void RigidbodyAddedLaterIsPickedUp()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            Physics.Provider = provider;
            try
            {
                GameObject owner = new GameObject("setter-late-body");
                owner.AssignNativeEntityId(8);
                Transform transform = owner.transform;

                transform.position = new Vector3(1f, 1f, 1f);
                Check.Equal("no body notification while no Rigidbody exists", 0, provider.SetBodyPositionCalls.Count);

                owner.AddComponent<Rigidbody>();
                transform.position = new Vector3(2f, 2f, 2f);
                Check.Equal("Rigidbody added after earlier sets is picked up", 1, provider.SetBodyPositionCalls.Count);
                Check.Equal("late Rigidbody receives the set", "8:2,2,2", provider.SetBodyPositionCalls[0].Call);
            }
            finally
            {
                Physics.Provider = previous;
            }
        }

        private static void DestroyedRigidbodyStopsNotifications()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            Physics.Provider = provider;
            try
            {
                GameObject owner = new GameObject("setter-removed-body");
                owner.AssignNativeEntityId(9);
                Rigidbody body = owner.AddComponent<Rigidbody>();
                Transform transform = owner.transform;

                transform.position = new Vector3(1f, 1f, 1f);
                Check.Equal("Rigidbody notified before removal", 1, provider.SetBodyPositionCalls.Count);

                UnityEngine.Object.Destroy(body);
                transform.position = new Vector3(3f, 3f, 3f);
                Check.Equal("destroyed Rigidbody stops notifications", 1, provider.SetBodyPositionCalls.Count);
            }
            finally
            {
                Physics.Provider = previous;
            }
        }

        private sealed class RecordingPhysicsProvider : IPhysicsProvider
        {
            public readonly List<(string Entity, string Call)> SetBodyPositionCalls = new List<(string Entity, string Call)>();

            public bool TrySetBodyPosition(ulong entity, Vector3 position)
            {
                SetBodyPositionCalls.Add((entity.ToString(CultureInfo.InvariantCulture), entity + ":" + position.x + "," + position.y + "," + position.z));
                return true;
            }

            public bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit)
            {
                hit = default;
                return false;
            }

            public bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask, QueryTriggerInteraction queryTriggerInteraction)
            {
                hitInfo = default;
                return false;
            }

            public bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                hit = default;
                return false;
            }

            public bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                hit = default;
                return false;
            }

            public Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
            {
                return Array.Empty<Collider>();
            }

            public bool TryGetBodyPosition(ulong entity, out Vector3 position)
            {
                position = default;
                return false;
            }

            public bool TryGetBodyVelocity(ulong entity, out Vector3 velocity)
            {
                velocity = default;
                return false;
            }

            public bool TrySetBodyVelocity(ulong entity, Vector3 velocity)
            {
                return false;
            }

            public bool TryAddForce(ulong entity, Vector3 force, ForceMode mode)
            {
                return false;
            }
        }
    }
}
