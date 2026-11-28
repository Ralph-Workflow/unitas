using System;

namespace UnityEngine.AI
{
    public enum NavMeshPathStatus
    {
        PathComplete = 0,
        PathPartial = 1,
        PathInvalid = 2,
    }

    public sealed class NavMeshPath
    {
        private Vector3[] _corners = Array.Empty<Vector3>();
        private NavMeshPathStatus _status = NavMeshPathStatus.PathInvalid;

        public Vector3[] corners => _corners;

        public NavMeshPathStatus status => _status;

        public void ClearCorners()
        {
            _corners = Array.Empty<Vector3>();
            _status = NavMeshPathStatus.PathInvalid;
        }

        public int GetCornersNonAlloc(Vector3[] results)
        {
            if (results == null || results.Length == 0 || _corners.Length == 0)
            {
                return 0;
            }
            int count = Math.Min(results.Length, _corners.Length);
            Array.Copy(_corners, 0, results, 0, count);
            return count;
        }

        internal void SetPathResult(NavMeshPathStatus pathStatus, Vector3[] pathCorners)
        {
            _status = pathStatus;
            _corners = pathCorners ?? Array.Empty<Vector3>();
        }
    }
}
