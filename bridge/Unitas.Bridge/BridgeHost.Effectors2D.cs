using UnityEngine;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IPhysicsEffector2DProvider
    {
        public bool TrySetAreaEffector2D(
            ulong entity,
            float forceAngle,
            float forceMagnitude,
            float forceVariation,
            float drag,
            float angularDrag,
            bool useGlobalAngle,
            uint colliderMask)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_collider2d_set_area_effector(
                _runtimeHandle,
                entity,
                forceAngle,
                forceMagnitude,
                forceVariation,
                drag,
                angularDrag,
                useGlobalAngle ? 1u : 0u,
                colliderMask) == 0;
        }

        public bool TrySetPointEffector2D(
            ulong entity,
            float forceMagnitude,
            float forceVariation,
            float distanceScale,
            float drag,
            float angularDrag,
            uint colliderMask)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_collider2d_set_point_effector(
                _runtimeHandle,
                entity,
                forceMagnitude,
                forceVariation,
                distanceScale,
                drag,
                angularDrag,
                colliderMask) == 0;
        }

        public bool TrySetBuoyancyEffector2D(
            ulong entity,
            float surfaceLevel,
            float density,
            float linearDrag,
            float angularDrag,
            float flowAngle,
            float flowMagnitude,
            float flowVariation,
            uint colliderMask)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_collider2d_set_buoyancy_effector(
                _runtimeHandle,
                entity,
                surfaceLevel,
                density,
                linearDrag,
                angularDrag,
                flowAngle,
                flowMagnitude,
                flowVariation,
                colliderMask) == 0;
        }
    }
}
