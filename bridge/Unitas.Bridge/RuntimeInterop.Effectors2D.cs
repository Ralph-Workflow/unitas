using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_collider2d_set_area_effector(
            ulong runtime,
            ulong entity,
            float forceAngle,
            float forceMagnitude,
            float forceVariation,
            float drag,
            float angularDrag,
            uint useGlobalAngle,
            uint colliderMask);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_collider2d_set_point_effector(
            ulong runtime,
            ulong entity,
            float forceMagnitude,
            float forceVariation,
            float distanceScale,
            float drag,
            float angularDrag,
            uint colliderMask);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_collider2d_set_buoyancy_effector(
            ulong runtime,
            ulong entity,
            float surfaceLevel,
            float density,
            float linearDrag,
            float angularDrag,
            float flowAngle,
            float flowMagnitude,
            float flowVariation,
            uint colliderMask);
    }
}
