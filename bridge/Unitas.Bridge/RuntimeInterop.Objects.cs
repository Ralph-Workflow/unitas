using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong unitas_runtime_object_create(ulong handle, ulong nameAddress, ulong nameLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_object_attach_script(ulong handle, ulong entity, ulong guidHi, ulong guidLo);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_object_set_transform(
            ulong handle,
            ulong entity,
            double positionX,
            double positionY,
            double positionZ,
            double rotationX,
            double rotationY,
            double rotationZ,
            double rotationW,
            double scaleX,
            double scaleY,
            double scaleZ);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_object_create_body(
            ulong handle,
            ulong entity,
            uint shape,
            double radius,
            double halfX,
            double halfY,
            double halfZ,
            double height,
            uint isTrigger);
    }
}
