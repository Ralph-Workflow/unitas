using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_joint3d_set(ulong runtime, ulong entity, ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_joint3d_remove(ulong runtime, ulong entity, uint kind);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_joint2d_set(ulong runtime, ulong entity, ulong inBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_joint2d_remove(ulong runtime, ulong entity, uint kind);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_body_rotation(ulong runtime, ulong entity, ulong outBuffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_body2d_position(ulong runtime, ulong entity, ulong outBuffer);
    }
}
