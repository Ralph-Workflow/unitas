using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_capsule_cast(
            ulong runtime,
            double p1X,
            double p1Y,
            double p1Z,
            double p2X,
            double p2Y,
            double p2Z,
            double radius,
            double dirX,
            double dirY,
            double dirZ,
            double maxDistance,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_overlap_box(
            ulong runtime,
            double centerX,
            double centerY,
            double centerZ,
            double halfX,
            double halfY,
            double halfZ,
            double rotX,
            double rotY,
            double rotZ,
            double rotW,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_overlap_capsule(
            ulong runtime,
            double p0X,
            double p0Y,
            double p0Z,
            double p1X,
            double p1Y,
            double p1Z,
            double radius,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_check_box(
            ulong runtime,
            double centerX,
            double centerY,
            double centerZ,
            double halfX,
            double halfY,
            double halfZ,
            double rotX,
            double rotY,
            double rotZ,
            double rotW,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_check_sphere(
            ulong runtime,
            double centerX,
            double centerY,
            double centerZ,
            double radius,
            uint layerMask,
            int includeTriggers);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_check_capsule(
            ulong runtime,
            double p0X,
            double p0Y,
            double p0Z,
            double p1X,
            double p1Y,
            double p1Z,
            double radius,
            uint layerMask,
            int includeTriggers);
    }
}
