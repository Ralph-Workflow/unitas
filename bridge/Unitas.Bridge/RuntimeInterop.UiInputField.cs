using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine.UI;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_input_char_event")]
        public static extern int unitas_runtime_input_char_event(ulong runtime, uint codepoint);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_create")]
        public static extern int unitas_runtime_ui_input_field_create(
            ulong runtime,
            ulong objectId,
            uint interactable,
            uint characterLimit,
            float fontSize);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_set_text")]
        public static extern int unitas_runtime_ui_input_field_set_text(
            ulong runtime,
            ulong objectId,
            ulong textPointer,
            ulong textLength,
            uint notify);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_get_text")]
        public static extern int unitas_runtime_ui_input_field_get_text(ulong runtime, ulong objectId);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_set_placeholder")]
        public static extern int unitas_runtime_ui_input_field_set_placeholder(
            ulong runtime,
            ulong objectId,
            ulong textPointer,
            ulong textLength);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_set_interactable")]
        public static extern int unitas_runtime_ui_input_field_set_interactable(
            ulong runtime,
            ulong objectId,
            uint interactable);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_set_character_limit")]
        public static extern int unitas_runtime_ui_input_field_set_character_limit(
            ulong runtime,
            ulong objectId,
            uint characterLimit);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_get_caret_position")]
        public static extern int unitas_runtime_ui_input_field_get_caret_position(ulong runtime, ulong objectId);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_set_caret_position")]
        public static extern int unitas_runtime_ui_input_field_set_caret_position(
            ulong runtime,
            ulong objectId,
            uint caretPosition);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_focus")]
        public static extern int unitas_runtime_ui_input_field_focus(
            ulong runtime,
            ulong objectId,
            uint focused);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_is_focused")]
        public static extern int unitas_runtime_ui_input_field_is_focused(ulong runtime, ulong objectId);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_was_canceled")]
        public static extern int unitas_runtime_ui_input_field_was_canceled(ulong runtime, ulong objectId);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_take_value_changed")]
        public static extern int unitas_runtime_ui_input_field_take_value_changed(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "unitas_runtime_ui_input_field_take_end_edit")]
        public static extern int unitas_runtime_ui_input_field_take_end_edit(ulong runtime);
    }

    internal sealed partial class RuntimeUiProvider
    {
        public int CreateInputField(
            ulong objectId,
            bool interactable,
            int characterLimit,
            float fontSize)
        {
            uint limit = characterLimit > 0 ? (uint)characterLimit : 0U;
            float size = fontSize > 0f ? fontSize : 16f;
            return RuntimeInterop.unitas_runtime_ui_input_field_create(
                _runtimeHandle,
                objectId,
                interactable ? 1U : 0U,
                limit,
                size);
        }

        public int SetInputFieldText(
            ulong objectId,
            string text,
            bool notify)
        {
            byte[] utf8 = Encoding.UTF8.GetBytes(text ?? string.Empty);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(
                _runtimeHandle,
                (ulong)Math.Max(utf8.Length, 1));
            if (buffer == IntPtr.Zero)
            {
                return -1;
            }

            if (utf8.Length > 0)
            {
                Marshal.Copy(utf8, 0, buffer, utf8.Length);
            }

            return RuntimeInterop.unitas_runtime_ui_input_field_set_text(
                _runtimeHandle,
                objectId,
                (ulong)buffer.ToInt64(),
                (ulong)utf8.Length,
                notify ? 1U : 0U);
        }

        public bool TryGetInputFieldText(
            ulong objectId,
            out string text)
        {
            text = string.Empty;
            int byteLength = RuntimeInterop.unitas_runtime_ui_input_field_get_text(_runtimeHandle, objectId);
            if (byteLength < 0)
            {
                return false;
            }

            if (byteLength == 0)
            {
                return true;
            }

            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)byteLength);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            text = Marshal.PtrToStringUTF8(buffer, byteLength) ?? string.Empty;
            return true;
        }

        public int SetInputFieldPlaceholder(
            ulong objectId,
            string placeholder)
        {
            byte[] utf8 = Encoding.UTF8.GetBytes(placeholder ?? string.Empty);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(
                _runtimeHandle,
                (ulong)Math.Max(utf8.Length, 1));
            if (buffer == IntPtr.Zero)
            {
                return -1;
            }

            if (utf8.Length > 0)
            {
                Marshal.Copy(utf8, 0, buffer, utf8.Length);
            }

            return RuntimeInterop.unitas_runtime_ui_input_field_set_placeholder(
                _runtimeHandle,
                objectId,
                (ulong)buffer.ToInt64(),
                (ulong)utf8.Length);
        }

        public int SetInputFieldInteractable(
            ulong objectId,
            bool interactable)
        {
            return RuntimeInterop.unitas_runtime_ui_input_field_set_interactable(
                _runtimeHandle,
                objectId,
                interactable ? 1U : 0U);
        }

        public int SetInputFieldCharacterLimit(
            ulong objectId,
            int characterLimit)
        {
            uint limit = characterLimit > 0 ? (uint)characterLimit : 0U;
            return RuntimeInterop.unitas_runtime_ui_input_field_set_character_limit(
                _runtimeHandle,
                objectId,
                limit);
        }

        public int GetInputFieldCaretPosition(ulong objectId)
        {
            return RuntimeInterop.unitas_runtime_ui_input_field_get_caret_position(_runtimeHandle, objectId);
        }

        public int SetInputFieldCaretPosition(
            ulong objectId,
            int caretPosition)
        {
            uint caret = caretPosition > 0 ? (uint)caretPosition : 0U;
            return RuntimeInterop.unitas_runtime_ui_input_field_set_caret_position(
                _runtimeHandle,
                objectId,
                caret);
        }

        public int FocusInputField(
            ulong objectId,
            bool focused)
        {
            return RuntimeInterop.unitas_runtime_ui_input_field_focus(
                _runtimeHandle,
                objectId,
                focused ? 1U : 0U);
        }

        public bool IsInputFieldFocused(ulong objectId)
        {
            return RuntimeInterop.unitas_runtime_ui_input_field_is_focused(_runtimeHandle, objectId) > 0;
        }

        public bool WasInputFieldCanceled(ulong objectId)
        {
            return RuntimeInterop.unitas_runtime_ui_input_field_was_canceled(_runtimeHandle, objectId) > 0;
        }

        public int InjectCharEvent(char character)
        {
            return RuntimeInterop.unitas_runtime_input_char_event(_runtimeHandle, character);
        }

        public int PollInputFieldValueChanged(List<UiInputFieldValueChangedEvent> events)
        {
            if (events == null)
            {
                return 0;
            }

            int count = RuntimeInterop.unitas_runtime_ui_input_field_take_value_changed(_runtimeHandle);
            if (count <= 0)
            {
                return 0;
            }

            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 1UL);
            if (buffer == IntPtr.Zero)
            {
                return 0;
            }

            int offset = 0;
            for (int index = 0; index < count; index++)
            {
                ulong entityId = (ulong)Marshal.ReadInt64(buffer, offset);
                offset += 8;
                int textLen = Marshal.ReadInt32(buffer, offset);
                offset += 4;
                string text = textLen > 0
                    ? (Marshal.PtrToStringUTF8(IntPtr.Add(buffer, offset), textLen) ?? string.Empty)
                    : string.Empty;
                if (textLen > 0)
                {
                    offset += textLen;
                }

                events.Add(new UiInputFieldValueChangedEvent(entityId, text));
            }

            return count;
        }

        public int PollInputFieldEndEdit(List<UiInputFieldEndEditEvent> events)
        {
            if (events == null)
            {
                return 0;
            }

            int count = RuntimeInterop.unitas_runtime_ui_input_field_take_end_edit(_runtimeHandle);
            if (count <= 0)
            {
                return 0;
            }

            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 1UL);
            if (buffer == IntPtr.Zero)
            {
                return 0;
            }

            int offset = 0;
            for (int index = 0; index < count; index++)
            {
                ulong entityId = (ulong)Marshal.ReadInt64(buffer, offset);
                offset += 8;
                bool canceled = Marshal.ReadInt32(buffer, offset) != 0;
                offset += 4;
                int textLen = Marshal.ReadInt32(buffer, offset);
                offset += 4;
                string text = textLen > 0
                    ? (Marshal.PtrToStringUTF8(IntPtr.Add(buffer, offset), textLen) ?? string.Empty)
                    : string.Empty;
                if (textLen > 0)
                {
                    offset += textLen;
                }

                events.Add(new UiInputFieldEndEditEvent(entityId, text, canceled));
            }

            return count;
        }
    }
}
