using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_text_set")]
        public static extern int unitas_runtime_ui_text_set(ulong runtime, ulong objectId, ulong textPointer, ulong textLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_text_style")]
        public static extern int unitas_runtime_ui_text_style(ulong runtime, ulong objectId, float fontSize, float r, float g, float b, float a);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_pointer_over")]
        public static extern int unitas_runtime_ui_pointer_over(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_poll_clicks")]
        public static extern int unitas_runtime_ui_poll_clicks(ulong runtime, ulong clicksPointer, ulong capacity);
    }

    /// <summary>
    /// The native uGUI surface. It talks to the runtime ABI the same way the other
    /// providers do: the runtime owns one scratch buffer, so every call fetches it
    /// again before it writes into it.
    /// </summary>
    internal sealed class RuntimeUiProvider : UnityEngine.UI.IUiProvider
    {
        private readonly ulong _runtimeHandle;

        internal RuntimeUiProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public int SetText(ulong objectId, string text)
        {
            byte[] utf8 = Encoding.UTF8.GetBytes(text ?? string.Empty);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)Math.Max(utf8.Length, 1));
            if (buffer == IntPtr.Zero)
            {
                return -1;
            }

            Marshal.Copy(utf8, 0, buffer, utf8.Length);
            return RuntimeInterop.unitas_runtime_ui_text_set(
                _runtimeHandle,
                objectId,
                (ulong)buffer.ToInt64(),
                (ulong)utf8.Length);
        }

        public int SetTextStyle(ulong objectId, float fontSize, UnityEngine.Color color)
        {
            return RuntimeInterop.unitas_runtime_ui_text_style(
                _runtimeHandle,
                objectId,
                fontSize,
                color.r,
                color.g,
                color.b,
                color.a);
        }

        public bool IsPointerOver()
        {
            return RuntimeInterop.unitas_runtime_ui_pointer_over(_runtimeHandle) != 0;
        }

        public int PollClicks(ulong[] clicks)
        {
            if (clicks == null || clicks.Length == 0)
            {
                return 0;
            }

            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)(clicks.Length * sizeof(ulong)));
            if (buffer == IntPtr.Zero)
            {
                return 0;
            }

            int count = RuntimeInterop.unitas_runtime_ui_poll_clicks(
                _runtimeHandle,
                (ulong)buffer.ToInt64(),
                (ulong)clicks.Length);
            if (count <= 0)
            {
                return 0;
            }

            if (count > clicks.Length)
            {
                count = clicks.Length;
            }

            for (int index = 0; index < count; index++)
            {
                clicks[index] = (ulong)Marshal.ReadInt64(buffer, index * sizeof(ulong));
            }

            return count;
        }
    }
}
