using System;
using System.Collections.Generic;
using UnityEngine.Events;

namespace UnityEngine.UI
{
    public class InputField : MonoBehaviour
    {
        public sealed class OnChangeEvent : UnityEvent<string>
        {
            public OnChangeEvent() : base("InputField", "m_OnValueChanged")
            {
            }
        }

        public sealed class SubmitEvent : UnityEvent<string>
        {
            public SubmitEvent() : base("InputField", "m_OnEndEdit")
            {
            }
        }

        private OnChangeEvent? _onValueChanged;
        private SubmitEvent? _onEndEdit;
        private string _text = string.Empty;
        private string _placeholderText = string.Empty;
        private Component? _placeholder;
        private Text? _textComponent;
        private int _characterLimit;
        private bool _interactable = true;
        private int _caretPosition;
        private bool _isFocused;
        private bool _wasCanceled;
        private string? _editingOriginalText;
        private ulong _syncedObjectId;
        private IUiProvider? _syncedProvider;

        public InputField()
        {
            UiInputFields.Register(this);
        }

        public OnChangeEvent onValueChanged
        {
            get => _onValueChanged ??= new OnChangeEvent();
            set => _onValueChanged = value;
        }

        public SubmitEvent onEndEdit
        {
            get => _onEndEdit ??= new SubmitEvent();
            set => _onEndEdit = value;
        }

        public string text
        {
            get
            {
                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId)
                    && provider.TryGetInputFieldText(objectId, out string nativeText))
                {
                    _text = nativeText;
                    SyncTextComponent();
                }

                return _text;
            }
            set => SetTextCore(value ?? string.Empty, sendCallback: true);
        }

        public Component? placeholder
        {
            get => _placeholder;
            set
            {
                _placeholder = value;
                if (value is Text placeholderTextComponent)
                {
                    _placeholderText = placeholderTextComponent.text ?? string.Empty;
                }

                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    provider.SetInputFieldPlaceholder(objectId, ResolveEffectivePlaceholderText());
                }
            }
        }

        public string placeholderText
        {
            get => ResolveEffectivePlaceholderText();
            set
            {
                _placeholderText = value ?? string.Empty;
                if (_placeholder is Text placeholderTextComponent)
                {
                    placeholderTextComponent.text = _placeholderText;
                }

                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    provider.SetInputFieldPlaceholder(objectId, _placeholderText);
                }
            }
        }

        public Text? textComponent
        {
            get => _textComponent;
            set
            {
                _textComponent = value;
                SyncTextComponent();
                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    provider.CreateInputField(
                        objectId,
                        _interactable && isActiveAndEnabled,
                        _characterLimit,
                        ResolveFontSize());
                }
            }
        }

        public int characterLimit
        {
            get => _characterLimit;
            set
            {
                int normalized = Math.Max(0, value);
                _characterLimit = normalized;
                if (normalized > 0 && _text.Length > normalized)
                {
                    _text = _text.Substring(0, normalized);
                    _caretPosition = Math.Clamp(_caretPosition, 0, _text.Length);
                    SyncTextComponent();
                }

                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    provider.SetInputFieldCharacterLimit(objectId, normalized);
                }
            }
        }

        public bool interactable
        {
            get => _interactable;
            set
            {
                _interactable = value;
                if (!value && _isFocused)
                {
                    _isFocused = false;
                    _editingOriginalText = null;
                }

                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    provider.SetInputFieldInteractable(objectId, value && isActiveAndEnabled);
                }
            }
        }

        public int caretPosition
        {
            get
            {
                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    int nativeCaret = provider.GetInputFieldCaretPosition(objectId);
                    if (nativeCaret >= 0)
                    {
                        _caretPosition = nativeCaret;
                    }
                }

                return Math.Clamp(_caretPosition, 0, _text.Length);
            }
            set
            {
                _caretPosition = Math.Clamp(value, 0, _text.Length);
                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    provider.SetInputFieldCaretPosition(objectId, _caretPosition);
                }
            }
        }

        public bool isFocused
        {
            get
            {
                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    _isFocused = provider.IsInputFieldFocused(objectId);
                }

                return _isFocused;
            }
        }

        public bool wasCanceled
        {
            get
            {
                if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
                {
                    _wasCanceled = provider.WasInputFieldCanceled(objectId);
                }

                return _wasCanceled;
            }
        }

        public ulong NativeObjectId
        {
            get
            {
                GameObject? owner = Owner;
                return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            }
        }

        internal GameObject? Owner => OwnerOrNull;

        public void SetTextWithoutNotify(string input)
        {
            SetTextCore(input ?? string.Empty, sendCallback: false);
        }

        public void ActivateInputField()
        {
            if (!_interactable || !isActiveAndEnabled)
            {
                return;
            }

            _wasCanceled = false;
            _editingOriginalText ??= _text;
            _caretPosition = _text.Length;
            _isFocused = true;

            if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
            {
                provider.FocusInputField(objectId, true);
            }
        }

        public void Select()
        {
            ActivateInputField();
        }

        public void DeactivateInputField(bool clearSelection = false)
        {
            _ = clearSelection;
            if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
            {
                provider.FocusInputField(objectId, false);
                _isFocused = false;
                _editingOriginalText = null;
                Ui.PumpInputFields();
                return;
            }

            bool wasEditing = _isFocused || _editingOriginalText != null;
            _isFocused = false;
            _editingOriginalText = null;
            if (wasEditing)
            {
                onEndEdit.Invoke(_text);
            }
        }

        public void ProcessEvent(char character)
        {
            if (!_interactable || !isActiveAndEnabled)
            {
                return;
            }

            if (!_isFocused)
            {
                ActivateInputField();
            }

            if (character == '\r' || character == '\n')
            {
                _wasCanceled = false;
                _editingOriginalText = _text;
                onEndEdit.Invoke(_text);
                return;
            }

            if (character == '\u001b')
            {
                _wasCanceled = true;
                if (_editingOriginalText != null && !string.Equals(_text, _editingOriginalText, StringComparison.Ordinal))
                {
                    _text = _editingOriginalText;
                    _caretPosition = _text.Length;
                    SyncTextComponent();
                    if (TryEnsureNativeSync(out IUiProvider escProvider, out ulong escObjectId))
                    {
                        escProvider.SetInputFieldText(escObjectId, _text, notify: false);
                    }

                    onValueChanged.Invoke(_text);
                }

                onEndEdit.Invoke(_text);
                return;
            }

            if (character == '\b')
            {
                int pos = Math.Clamp(_caretPosition, 0, _text.Length);
                if (pos > 0)
                {
                    _text = _text.Remove(pos - 1, 1);
                    _caretPosition = pos - 1;
                    SyncAfterLocalEdit();
                }

                return;
            }

            if (character == '\u007f')
            {
                int pos = Math.Clamp(_caretPosition, 0, _text.Length);
                if (pos < _text.Length)
                {
                    _text = _text.Remove(pos, 1);
                    SyncAfterLocalEdit();
                }

                return;
            }

            if (char.IsControl(character))
            {
                return;
            }

            if (_characterLimit > 0 && _text.Length >= _characterLimit)
            {
                return;
            }

            int insertPos = Math.Clamp(_caretPosition, 0, _text.Length);
            _text = _text.Insert(insertPos, character.ToString());
            _caretPosition = insertPos + 1;
            SyncAfterLocalEdit();
        }

        public override void OnEnable()
        {
            base.OnEnable();
            TryEnsureNativeSync(out _, out _);
        }

        public override void OnDisable()
        {
            if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
            {
                provider.SetInputFieldInteractable(objectId, false);
            }

            _isFocused = false;
            _editingOriginalText = null;
            base.OnDisable();
        }

        internal void HandleNativeValueChanged(string newText)
        {
            _text = newText ?? string.Empty;
            _caretPosition = Math.Clamp(_caretPosition, 0, _text.Length);
            SyncTextComponent();
            onValueChanged.Invoke(_text);
        }

        internal void HandleNativeEndEdit(string finalText, bool canceled)
        {
            _text = finalText ?? string.Empty;
            _wasCanceled = canceled;
            _caretPosition = Math.Clamp(_caretPosition, 0, _text.Length);
            SyncTextComponent();
            onEndEdit.Invoke(_text);
        }

        internal void EnsureNativeSynced(IUiProvider provider)
        {
            ulong objectId = NativeObjectId;
            if (objectId == 0UL)
            {
                return;
            }

            if (_syncedProvider == provider && _syncedObjectId == objectId)
            {
                if (_placeholder is Text placeholderTextComponent)
                {
                    string currentPlaceholder = placeholderTextComponent.text ?? string.Empty;
                    if (!string.Equals(currentPlaceholder, _placeholderText, StringComparison.Ordinal))
                    {
                        _placeholderText = currentPlaceholder;
                        provider.SetInputFieldPlaceholder(objectId, _placeholderText);
                    }
                }

                return;
            }

            _syncedProvider = provider;
            _syncedObjectId = objectId;
            provider.CreateInputField(
                objectId,
                _interactable && isActiveAndEnabled,
                _characterLimit,
                ResolveFontSize());
            provider.SetInputFieldPlaceholder(objectId, ResolveEffectivePlaceholderText());
            provider.SetInputFieldText(objectId, _text, notify: false);
            provider.SetInputFieldCaretPosition(objectId, Math.Clamp(_caretPosition, 0, _text.Length));
            if (_isFocused && _interactable && isActiveAndEnabled)
            {
                provider.FocusInputField(objectId, true);
            }
        }

        private bool TryEnsureNativeSync(out IUiProvider provider, out ulong objectId)
        {
            IUiProvider? currentProvider = Ui.Provider;
            objectId = NativeObjectId;
            if (currentProvider == null || objectId == 0UL)
            {
                provider = null!;
                return false;
            }

            provider = currentProvider;
            EnsureNativeSynced(provider);
            return true;
        }

        private void SetTextCore(string input, bool sendCallback)
        {
            string normalized = input;
            if (_characterLimit > 0 && normalized.Length > _characterLimit)
            {
                normalized = normalized.Substring(0, _characterLimit);
            }

            bool changed = !string.Equals(_text, normalized, StringComparison.Ordinal);
            _text = normalized;
            _caretPosition = _text.Length;
            SyncTextComponent();

            if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
            {
                provider.SetInputFieldText(objectId, _text, notify: false);
                provider.SetInputFieldCaretPosition(objectId, _caretPosition);
            }

            if (changed && sendCallback)
            {
                onValueChanged.Invoke(_text);
            }
        }

        private void SyncAfterLocalEdit()
        {
            SyncTextComponent();
            if (TryEnsureNativeSync(out IUiProvider provider, out ulong objectId))
            {
                provider.SetInputFieldText(objectId, _text, notify: false);
                provider.SetInputFieldCaretPosition(objectId, _caretPosition);
            }

            onValueChanged.Invoke(_text);
        }

        private void SyncTextComponent()
        {
            if (_textComponent != null && !_textComponent.IsDestroyed)
            {
                _textComponent.text = _text;
            }
        }

        private string ResolveEffectivePlaceholderText()
        {
            if (_placeholder is Text placeholderTextComponent && !placeholderTextComponent.IsDestroyed)
            {
                return placeholderTextComponent.text ?? string.Empty;
            }

            return _placeholderText;
        }

        private float ResolveFontSize()
        {
            if (_textComponent != null && !_textComponent.IsDestroyed && _textComponent.fontSize > 0)
            {
                return _textComponent.fontSize;
            }

            return 16f;
        }
    }

    internal static class UiInputFields
    {
        private static readonly List<InputField> Fields = new List<InputField>();

        internal static void Register(InputField field)
        {
            Fields.Add(field);
        }

        internal static void SyncAll(IUiProvider provider)
        {
            for (int index = Fields.Count - 1; index >= 0; index--)
            {
                InputField field = Fields[index];
                if (field == null || field.IsDestroyed)
                {
                    Fields.RemoveAt(index);
                    continue;
                }

                if (field.NativeObjectId != 0UL)
                {
                    field.EnsureNativeSynced(provider);
                }
            }
        }

        internal static InputField? Find(ulong objectId)
        {
            if (objectId == 0UL)
            {
                return null;
            }

            for (int index = Fields.Count - 1; index >= 0; index--)
            {
                InputField field = Fields[index];
                if (field == null || field.IsDestroyed)
                {
                    Fields.RemoveAt(index);
                    continue;
                }

                if (field.NativeObjectId == objectId)
                {
                    return field;
                }
            }

            return null;
        }
    }
}
