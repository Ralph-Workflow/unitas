using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_physics_set_simulation_mode(ulong runtime, uint mode);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_physics_get_simulation_mode(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_physics_simulate(ulong runtime, double stepSeconds);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_physics2d_set_simulation_mode(ulong runtime, uint mode);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_physics2d_get_simulation_mode(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int unitas_runtime_physics2d_simulate(ulong runtime, float stepSeconds);
    }
}
