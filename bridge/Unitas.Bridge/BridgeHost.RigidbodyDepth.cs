using System;
using System.Runtime.InteropServices;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IPhysicsRigidbodyDepthProvider
    {
        public bool TryGetBodyAngularVelocity(ulong entity, out Vector3 angularVelocity)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                angularVelocity = Vector3.zero;
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24).ToInt64();
            if (buffer == 0)
            {
                angularVelocity = Vector3.zero;
                return false;
            }

            int rc = RuntimeInterop.unitas_runtime_body_angular_velocity(_runtimeHandle, entity, buffer);
            if (rc != 0)
            {
                angularVelocity = Vector3.zero;
                return false;
            }

            double[] values = new double[3];
            Marshal.Copy(new IntPtr((long)buffer), values, 0, 3);
            angularVelocity = new Vector3((float)values[0], (float)values[1], (float)values[2]);
            return true;
        }

        public bool TrySetBodyAngularVelocity(ulong entity, Vector3 angularVelocity)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24).ToInt64();
            if (buffer == 0)
            {
                return false;
            }

            double[] values =
            {
                angularVelocity.x,
                angularVelocity.y,
                angularVelocity.z,
            };
            Marshal.Copy(values, 0, new IntPtr((long)buffer), 3);
            return RuntimeInterop.unitas_runtime_set_body_angular_velocity(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryAddBodyTorque(ulong entity, Vector3 torque, ForceMode mode)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32).ToInt64();
            if (buffer == 0)
            {
                return false;
            }

            double[] values = { torque.x, torque.y, torque.z, (double)(int)mode };
            Marshal.Copy(values, 0, new IntPtr((long)buffer), 4);
            return RuntimeInterop.unitas_runtime_add_body_torque(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryAddBodyRelativeForce(ulong entity, Vector3 force, ForceMode mode)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32).ToInt64();
            if (buffer == 0)
            {
                return false;
            }

            double[] values = { force.x, force.y, force.z, (double)(int)mode };
            Marshal.Copy(values, 0, new IntPtr((long)buffer), 4);
            return RuntimeInterop.unitas_runtime_add_body_relative_force(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryAddBodyRelativeTorque(ulong entity, Vector3 torque, ForceMode mode)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32).ToInt64();
            if (buffer == 0)
            {
                return false;
            }

            double[] values = { torque.x, torque.y, torque.z, (double)(int)mode };
            Marshal.Copy(values, 0, new IntPtr((long)buffer), 4);
            return RuntimeInterop.unitas_runtime_add_body_relative_torque(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryAddBodyForceAtPosition(ulong entity, Vector3 force, Vector3 position, ForceMode mode)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 56).ToInt64();
            if (buffer == 0)
            {
                return false;
            }

            double[] values =
            {
                force.x,
                force.y,
                force.z,
                position.x,
                position.y,
                position.z,
                (double)(int)mode,
            };
            Marshal.Copy(values, 0, new IntPtr((long)buffer), 7);
            return RuntimeInterop.unitas_runtime_add_body_force_at_position(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryAddBodyExplosionForce(
            ulong entity,
            float explosionForce,
            Vector3 explosionPosition,
            float explosionRadius,
            float upwardsModifier,
            ForceMode mode)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 56).ToInt64();
            if (buffer == 0)
            {
                return false;
            }

            double[] values =
            {
                explosionForce,
                explosionPosition.x,
                explosionPosition.y,
                explosionPosition.z,
                explosionRadius,
                upwardsModifier,
                (double)(int)mode,
            };
            Marshal.Copy(values, 0, new IntPtr((long)buffer), 7);
            return RuntimeInterop.unitas_runtime_add_body_explosion_force(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TrySleepBody(ulong entity)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_body_sleep(_runtimeHandle, entity) == 0;
        }

        public bool TryWakeUpBody(ulong entity)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_body_wake_up(_runtimeHandle, entity) == 0;
        }

        public bool TryIsBodySleeping(ulong entity, out bool sleeping)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                sleeping = false;
                return false;
            }

            int rc = RuntimeInterop.unitas_runtime_body_is_sleeping(_runtimeHandle, entity);
            if (rc < 0)
            {
                sleeping = false;
                return false;
            }

            sleeping = rc != 0;
            return true;
        }

        public bool TrySetBodyInterpolation(ulong entity, RigidbodyInterpolation mode)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_body_set_interpolation(_runtimeHandle, entity, (uint)mode) == 0;
        }

        public bool TrySetBodyConstraints(ulong entity, RigidbodyConstraints constraints)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_body_set_constraints(_runtimeHandle, entity, (uint)constraints) == 0;
        }
    }
}
