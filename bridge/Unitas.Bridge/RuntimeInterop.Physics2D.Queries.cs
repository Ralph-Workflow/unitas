using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_circle_cast2d(ulong runtime, float originX, float originY, float radius, float directionX, float directionY, float distance, int useTriggers, uint layerMask, int useDepth, float minDepth, float maxDepth, int useNormalAngle, float minNormalAngle, float maxNormalAngle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_capsule_cast2d(ulong runtime, float originX, float originY, float sizeX, float sizeY, uint capsuleDirection, float angle, float directionX, float directionY, float distance, int useTriggers, uint layerMask, int useDepth, float minDepth, float maxDepth, int useNormalAngle, float minNormalAngle, float maxNormalAngle);
    }
}
