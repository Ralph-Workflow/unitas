using System.Runtime.InteropServices;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint unitas_runtime_input_gamepad_button_state(ulong handle, uint button);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double unitas_runtime_input_gamepad_axis(ulong handle, uint axis);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint unitas_runtime_input_gamepad_connected(ulong handle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_input_gamepad_set_connected(ulong handle, uint connected);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_input_gamepad_button_event(ulong handle, uint button, uint pressed);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int unitas_runtime_input_gamepad_axis_event(ulong handle, uint axis, double value);
    }
}
