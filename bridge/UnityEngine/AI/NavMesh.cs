using System;

namespace UnityEngine.AI
{
    public struct NavMeshHit : IEquatable<NavMeshHit>
    {
        public Vector3 position { get; set; }

        public Vector3 normal { get; set; }

        public float distance { get; set; }

        public int mask { get; set; }

        public bool hit { get; set; }

        public bool Equals(NavMeshHit other)
        {
            return position == other.position
                && normal == other.normal
                && distance == other.distance
                && mask == other.mask
                && hit == other.hit;
        }

        public override bool Equals(object? obj)
        {
            return obj is NavMeshHit other && Equals(other);
        }

        public override int GetHashCode()
        {
            return HashCode.Combine(position, normal, distance, mask, hit);
        }

        public static bool operator ==(NavMeshHit left, NavMeshHit right)
        {
            return left.Equals(right);
        }

        public static bool operator !=(NavMeshHit left, NavMeshHit right)
        {
            return !left.Equals(right);
        }
    }

    public struct NavMeshQueryFilter : IEquatable<NavMeshQueryFilter>
    {
        public int areaMask { get; set; }

        public int agentTypeID { get; set; }

        public bool Equals(NavMeshQueryFilter other)
        {
            return areaMask == other.areaMask && agentTypeID == other.agentTypeID;
        }

        public override bool Equals(object? obj)
        {
            return obj is NavMeshQueryFilter other && Equals(other);
        }

        public override int GetHashCode()
        {
            return HashCode.Combine(areaMask, agentTypeID);
        }

        public static bool operator ==(NavMeshQueryFilter left, NavMeshQueryFilter right)
        {
            return left.Equals(right);
        }

        public static bool operator !=(NavMeshQueryFilter left, NavMeshQueryFilter right)
        {
            return !left.Equals(right);
        }
    }

    public interface INavigationProvider
    {
        bool EnsureAgent(ulong entity);

        bool HasAgent(ulong entity);

        bool SetDestination(ulong entity, Vector3 target, out NavMeshPathStatus status);

        bool TryGetDestination(ulong entity, out Vector3 destination);

        bool Warp(ulong entity, Vector3 newPosition);

        bool Move(ulong entity, Vector3 offset);

        bool ResetPath(ulong entity);

        bool SetStopped(ulong entity, bool stopped);

        bool TryGetStopped(ulong entity, out bool stopped);

        bool SetSpeed(ulong entity, float speed);

        bool TryGetSpeed(ulong entity, out float speed);

        bool SetAcceleration(ulong entity, float acceleration);

        bool TryGetAcceleration(ulong entity, out float acceleration);

        bool SetAngularSpeed(ulong entity, float angularSpeed);

        bool TryGetAngularSpeed(ulong entity, out float angularSpeed);

        bool SetStoppingDistance(ulong entity, float distance);

        bool TryGetStoppingDistance(ulong entity, out float distance);

        bool SetRadius(ulong entity, float radius);

        bool TryGetRadius(ulong entity, out float radius);

        bool SetHeight(ulong entity, float height);

        bool TryGetHeight(ulong entity, out float height);

        bool TryGetRemainingDistance(ulong entity, out float remainingDistance);

        bool TryGetPathStatus(ulong entity, out NavMeshPathStatus status);

        bool TryGetIsOnNavMesh(ulong entity, out bool isOnNavMesh);

        bool TryGetHasPath(ulong entity, out bool hasPath);

        bool TryGetPosition(ulong entity, out Vector3 position);

        bool TryGetVelocity(ulong entity, out Vector3 velocity);

        bool SamplePosition(Vector3 sourcePosition, out NavMeshHit hit, float maxDistance, int areaMask);

        bool Raycast(Vector3 sourcePosition, Vector3 targetPosition, out NavMeshHit hit, int areaMask);

        bool CalculatePath(Vector3 sourcePosition, Vector3 targetPosition, int areaMask, out NavMeshPathStatus status, out Vector3[] corners);

        bool SetObstacleCarving(ulong entity, bool carving, Vector3 center, Vector3 size);

        bool TryGetObstacleCarving(ulong entity, out bool carving);

        bool ConfigureObstacle(ulong entity, bool enabled, bool carving, bool carveOnlyStationary, NavMeshObstacleShape shape, Vector3 center, Vector3 size, float radius, float height);
    }

    public static class NavMesh
    {
        public const int AllAreas = -1;

        public static INavigationProvider? Provider { get; set; }

        public static bool SamplePosition(Vector3 sourcePosition, out NavMeshHit hit, float maxDistance, int areaMask)
        {
            hit = default;
            INavigationProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.SamplePosition(sourcePosition, out hit, maxDistance, areaMask);
        }

        public static bool SamplePosition(Vector3 sourcePosition, out NavMeshHit hit, float maxDistance, NavMeshQueryFilter filter)
        {
            return SamplePosition(sourcePosition, out hit, maxDistance, filter.areaMask == 0 ? AllAreas : filter.areaMask);
        }

        public static bool Raycast(Vector3 sourcePosition, Vector3 targetPosition, out NavMeshHit hit, int areaMask)
        {
            hit = default;
            INavigationProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Raycast(sourcePosition, targetPosition, out hit, areaMask);
        }

        public static bool Raycast(Vector3 sourcePosition, Vector3 targetPosition, out NavMeshHit hit, NavMeshQueryFilter filter)
        {
            return Raycast(sourcePosition, targetPosition, out hit, filter.areaMask == 0 ? AllAreas : filter.areaMask);
        }

        public static bool CalculatePath(Vector3 sourcePosition, Vector3 targetPosition, int areaMask, NavMeshPath path)
        {
            if (path == null)
            {
                throw new ArgumentNullException(nameof(path));
            }
            path.ClearCorners();
            INavigationProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            if (!provider.CalculatePath(sourcePosition, targetPosition, areaMask, out NavMeshPathStatus status, out Vector3[] corners))
            {
                path.SetPathResult(NavMeshPathStatus.PathInvalid, Array.Empty<Vector3>());
                return false;
            }
            path.SetPathResult(status, corners);
            return status != NavMeshPathStatus.PathInvalid;
        }

        public static bool CalculatePath(Vector3 sourcePosition, Vector3 targetPosition, NavMeshQueryFilter filter, NavMeshPath path)
        {
            return CalculatePath(sourcePosition, targetPosition, filter.areaMask == 0 ? AllAreas : filter.areaMask, path);
        }
    }
}
