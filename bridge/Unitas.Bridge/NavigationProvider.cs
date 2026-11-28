using System;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.AI;

namespace Unitas.Bridge
{
    public sealed class RuntimeNavigationProvider : INavigationProvider
    {
        private const int HitBlobBytes = 64;
        private const int PathInitialBytes = 4096;

        private readonly ulong _runtimeHandle;

        public RuntimeNavigationProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public bool EnsureAgent(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_ensure(_runtimeHandle, entity) == 0;
        }

        public bool HasAgent(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_has(_runtimeHandle, entity) > 0;
        }

        public bool SetDestination(ulong entity, Vector3 target, out NavMeshPathStatus status)
        {
            status = NavMeshPathStatus.PathInvalid;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_agent_set_destination(
                _runtimeHandle,
                entity,
                target.x,
                target.y,
                target.z);
            if (code < 0)
            {
                return false;
            }
            status = (NavMeshPathStatus)code;
            return status != NavMeshPathStatus.PathInvalid;
        }

        public bool TryGetDestination(ulong entity, out Vector3 destination)
        {
            return TryReadVec3(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_destination, out destination);
        }

        public bool Warp(ulong entity, Vector3 newPosition)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_warp(
                _runtimeHandle,
                entity,
                newPosition.x,
                newPosition.y,
                newPosition.z) >= 0;
        }

        public bool Move(ulong entity, Vector3 offset)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_move(
                _runtimeHandle,
                entity,
                offset.x,
                offset.y,
                offset.z) == 0;
        }

        public bool ResetPath(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_reset_path(_runtimeHandle, entity) == 0;
        }

        public bool SetStopped(ulong entity, bool stopped)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_stopped(
                _runtimeHandle,
                entity,
                stopped ? (byte)1 : (byte)0) == 0;
        }

        public bool TryGetStopped(ulong entity, out bool stopped)
        {
            stopped = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_agent_get_stopped(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            stopped = code != 0;
            return true;
        }

        public bool SetSpeed(ulong entity, float speed)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_speed(_runtimeHandle, entity, speed) == 0;
        }

        public bool TryGetSpeed(ulong entity, out float speed)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_speed, out speed);
        }

        public bool SetAcceleration(ulong entity, float acceleration)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_acceleration(_runtimeHandle, entity, acceleration) == 0;
        }

        public bool TryGetAcceleration(ulong entity, out float acceleration)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_acceleration, out acceleration);
        }

        public bool SetAngularSpeed(ulong entity, float angularSpeed)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_angular_speed(_runtimeHandle, entity, angularSpeed) == 0;
        }

        public bool TryGetAngularSpeed(ulong entity, out float angularSpeed)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_angular_speed, out angularSpeed);
        }

        public bool SetStoppingDistance(ulong entity, float distance)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_stopping_distance(_runtimeHandle, entity, distance) == 0;
        }

        public bool TryGetStoppingDistance(ulong entity, out float distance)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_stopping_distance, out distance);
        }

        public bool SetRadius(ulong entity, float radius)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_radius(_runtimeHandle, entity, radius) == 0;
        }

        public bool TryGetRadius(ulong entity, out float radius)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_radius, out radius);
        }

        public bool SetHeight(ulong entity, float height)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_agent_set_height(_runtimeHandle, entity, height) == 0;
        }

        public bool TryGetHeight(ulong entity, out float height)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_height, out height);
        }

        public bool TryGetRemainingDistance(ulong entity, out float remainingDistance)
        {
            return TryReadScalar(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_remaining_distance, out remainingDistance);
        }

        public bool TryGetPathStatus(ulong entity, out NavMeshPathStatus status)
        {
            status = NavMeshPathStatus.PathInvalid;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_agent_get_path_status(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            status = (NavMeshPathStatus)code;
            return true;
        }

        public bool TryGetIsOnNavMesh(ulong entity, out bool isOnNavMesh)
        {
            isOnNavMesh = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_agent_is_on_navmesh(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            isOnNavMesh = code != 0;
            return true;
        }

        public bool TryGetHasPath(ulong entity, out bool hasPath)
        {
            hasPath = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_agent_has_path(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            hasPath = code != 0;
            return true;
        }

        public bool TryGetPosition(ulong entity, out Vector3 position)
        {
            return TryReadVec3(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_position, out position);
        }

        public bool TryGetVelocity(ulong entity, out Vector3 velocity)
        {
            return TryReadVec3(entity, RuntimeInterop.unitas_runtime_navmesh_agent_get_velocity, out velocity);
        }

        public bool SamplePosition(Vector3 sourcePosition, out NavMeshHit hit, float maxDistance, int areaMask)
        {
            hit = default;
            if (_runtimeHandle == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, HitBlobBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_sample_position(
                _runtimeHandle,
                sourcePosition.x,
                sourcePosition.y,
                sourcePosition.z,
                maxDistance,
                areaMask,
                buffer);
            if (code <= 0)
            {
                return false;
            }
            hit = ReadHitFromBuffer(buffer, true);
            return true;
        }

        public bool Raycast(Vector3 sourcePosition, Vector3 targetPosition, out NavMeshHit hit, int areaMask)
        {
            hit = default;
            if (_runtimeHandle == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, HitBlobBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_raycast(
                _runtimeHandle,
                sourcePosition.x,
                sourcePosition.y,
                sourcePosition.z,
                targetPosition.x,
                targetPosition.y,
                targetPosition.z,
                areaMask,
                buffer);
            if (code < 0)
            {
                return false;
            }
            bool blocked = code != 0;
            hit = ReadHitFromBuffer(buffer, blocked);
            return blocked;
        }

        public bool CalculatePath(
            Vector3 sourcePosition,
            Vector3 targetPosition,
            int areaMask,
            out NavMeshPathStatus status,
            out Vector3[] corners)
        {
            status = NavMeshPathStatus.PathInvalid;
            corners = Array.Empty<Vector3>();
            if (_runtimeHandle == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, PathInitialBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_calculate_path(
                _runtimeHandle,
                sourcePosition.x,
                sourcePosition.y,
                sourcePosition.z,
                targetPosition.x,
                targetPosition.y,
                targetPosition.z,
                areaMask,
                buffer);
            if (code < 0)
            {
                return false;
            }
            IntPtr readPtr = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 8);
            if (readPtr == IntPtr.Zero)
            {
                return false;
            }
            byte[] header = new byte[8];
            Marshal.Copy(readPtr, header, 0, 8);
            status = (NavMeshPathStatus)BitConverter.ToInt32(header, 0);
            int cornerCount = BitConverter.ToInt32(header, 4);
            if (cornerCount <= 0)
            {
                return status != NavMeshPathStatus.PathInvalid;
            }
            int totalBytes = 8 + cornerCount * 24;
            IntPtr fullPtr = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)totalBytes);
            if (fullPtr == IntPtr.Zero)
            {
                return false;
            }
            byte[] payload = new byte[totalBytes];
            Marshal.Copy(fullPtr, payload, 0, totalBytes);
            corners = new Vector3[cornerCount];
            for (int index = 0; index < cornerCount; index++)
            {
                int offset = 8 + index * 24;
                corners[index] = new Vector3(
                    (float)BitConverter.ToDouble(payload, offset),
                    (float)BitConverter.ToDouble(payload, offset + 8),
                    (float)BitConverter.ToDouble(payload, offset + 16));
            }
            return status != NavMeshPathStatus.PathInvalid;
        }

        public bool SetObstacleCarving(ulong entity, bool carving, Vector3 center, Vector3 size)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_navmesh_obstacle_set_carving(
                _runtimeHandle,
                entity,
                carving ? (byte)1 : (byte)0,
                center.x,
                center.y,
                center.z,
                size.x,
                size.y,
                size.z) == 0;
        }

        public bool TryGetObstacleCarving(ulong entity, out bool carving)
        {
            carving = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_navmesh_obstacle_get_carving(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            carving = code != 0;
            return true;
        }

        public bool ConfigureObstacle(
            ulong entity,
            bool enabled,
            bool carving,
            bool carveOnlyStationary,
            NavMeshObstacleShape shape,
            Vector3 center,
            Vector3 size,
            float radius,
            float height)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 64);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[64];
            BitConverter.GetBytes((double)center.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)center.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)center.z).CopyTo(bytes, 16);
            BitConverter.GetBytes((double)size.x).CopyTo(bytes, 24);
            BitConverter.GetBytes((double)size.y).CopyTo(bytes, 32);
            BitConverter.GetBytes((double)size.z).CopyTo(bytes, 40);
            BitConverter.GetBytes((double)radius).CopyTo(bytes, 48);
            BitConverter.GetBytes((double)height).CopyTo(bytes, 56);
            Marshal.Copy(bytes, 0, buffer, 64);
            return RuntimeInterop.unitas_runtime_navmesh_obstacle_configure(
                _runtimeHandle,
                entity,
                enabled ? (byte)1 : (byte)0,
                carving ? (byte)1 : (byte)0,
                carveOnlyStationary ? (byte)1 : (byte)0,
                (int)shape,
                buffer) == 0;
        }

        private bool TryReadScalar(ulong entity, Func<ulong, ulong, IntPtr, int> query, out float value)
        {
            value = 0f;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 8);
            if (buffer == IntPtr.Zero || query(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[8];
            Marshal.Copy(buffer, bytes, 0, 8);
            value = (float)BitConverter.ToDouble(bytes, 0);
            return true;
        }

        private bool TryReadVec3(ulong entity, Func<ulong, ulong, IntPtr, int> query, out Vector3 value)
        {
            value = Vector3.zero;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero || query(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            Marshal.Copy(buffer, bytes, 0, 24);
            value = new Vector3(
                (float)BitConverter.ToDouble(bytes, 0),
                (float)BitConverter.ToDouble(bytes, 8),
                (float)BitConverter.ToDouble(bytes, 16));
            return true;
        }

        private static NavMeshHit ReadHitFromBuffer(IntPtr buffer, bool hitFlag)
        {
            byte[] bytes = new byte[HitBlobBytes];
            Marshal.Copy(buffer, bytes, 0, HitBlobBytes);
            return new NavMeshHit
            {
                position = new Vector3(
                    (float)BitConverter.ToDouble(bytes, 0),
                    (float)BitConverter.ToDouble(bytes, 8),
                    (float)BitConverter.ToDouble(bytes, 16)),
                normal = new Vector3(
                    (float)BitConverter.ToDouble(bytes, 24),
                    (float)BitConverter.ToDouble(bytes, 32),
                    (float)BitConverter.ToDouble(bytes, 40)),
                distance = (float)BitConverter.ToDouble(bytes, 48),
                mask = (int)BitConverter.ToDouble(bytes, 56),
                hit = hitFlag,
            };
        }
    }
}
