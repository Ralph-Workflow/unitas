using System;
using System.Collections.Generic;
using System.Globalization;

namespace UnityEngine.InputSystem
{
    public sealed class InputAction
    {
        private static readonly List<InputAction> EnabledActions = new List<InputAction>();

        private readonly List<InputBinding> _bindings = new List<InputBinding>();
        private readonly List<InputControl> _resolvedControls = new List<InputControl>();
        private readonly string _defaultInteractions;
        private readonly string _defaultProcessors;

        private InputActionPhase _phase = InputActionPhase.Disabled;
        private bool _enabled;
        private bool _wasPressedThisFrame;
        private bool _wasReleasedThisFrame;
        private bool _wasPerformedThisFrame;
        private float _currentFloat;
        private Vector2 _currentVector2;
        private bool _currentIsVector2;
        private float _currentMagnitude;
        private float _pressPoint = 0.5f;
        private InputControl? _activeControl;

        public InputAction()
            : this(null, InputActionType.Value, null, null, null, null)
        {
        }

        public InputAction(
            string? name = null,
            InputActionType type = InputActionType.Value,
            string? binding = null,
            string? interactions = null,
            string? processors = null,
            string? expectedControlType = null)
        {
            this.name = name ?? string.Empty;
            this.type = type;
            this.expectedControlType = expectedControlType ?? string.Empty;
            _defaultInteractions = interactions ?? string.Empty;
            _defaultProcessors = processors ?? string.Empty;
            _pressPoint = ParsePressPoint(_defaultInteractions, 0.5f);
            id = Guid.NewGuid();
            if (!string.IsNullOrEmpty(binding))
            {
                AddBinding(binding, interactions, processors, null);
            }
        }

        public Guid id { get; }

        public string name { get; internal set; }

        public InputActionType type { get; }

        public string expectedControlType { get; set; }

        public InputActionPhase phase => _phase;

        public bool enabled => _enabled;

        public bool triggered => WasPerformedThisFrame();

        public InputActionMap? actionMap { get; internal set; }

        public IReadOnlyList<InputBinding> bindings => _bindings;

        public IReadOnlyList<InputControl> controls
        {
            get
            {
                RefreshResolvedControls();
                return _resolvedControls;
            }
        }

        public InputControl? activeControl => _activeControl;

        public bool isPressed => IsPressed();

        public bool wasPressedThisFrame => WasPressedThisFrame();

        public bool wasReleasedThisFrame => WasReleasedThisFrame();

        public event Action<CallbackContext>? started;

        public event Action<CallbackContext>? performed;

        public event Action<CallbackContext>? canceled;

        public InputBinding AddBinding(
            string path,
            string? interactions = null,
            string? processors = null,
            string? groups = null)
        {
            InputBinding binding = new InputBinding(
                path,
                name,
                groups,
                processors ?? _defaultProcessors,
                interactions ?? _defaultInteractions,
                null,
                false,
                false);
            return AddBinding(binding);
        }

        public InputBinding AddBinding(InputBinding binding)
        {
            if (binding.path.StartsWith("composite:", StringComparison.OrdinalIgnoreCase))
            {
                string compositeName = binding.path.Substring("composite:".Length).Trim();
                binding = new InputBinding(
                    compositeName,
                    binding.action,
                    binding.groups,
                    binding.processors,
                    binding.interactions,
                    string.IsNullOrEmpty(binding.name) ? compositeName : binding.name,
                    true,
                    false);
            }
            _bindings.Add(binding);
            float parsedPress = ParsePressPoint(binding.interactions, _pressPoint);
            _pressPoint = parsedPress;
            RefreshResolvedControls();
            return binding;
        }

        public CompositeSyntax AddCompositeBinding(
            string composite,
            string? interactions = null,
            string? processors = null)
        {
            if (string.IsNullOrWhiteSpace(composite))
            {
                throw new ArgumentException("Composite name cannot be empty.", nameof(composite));
            }

            string normalized = composite.Trim();
            if (normalized.StartsWith("composite:", StringComparison.OrdinalIgnoreCase))
            {
                normalized = normalized.Substring("composite:".Length).Trim();
            }

            InputBinding header = new InputBinding(
                normalized,
                name,
                null,
                processors ?? _defaultProcessors,
                interactions ?? _defaultInteractions,
                normalized,
                true,
                false);
            int index = _bindings.Count;
            _bindings.Add(header);
            return new CompositeSyntax(this, index);
        }

        internal void AddCompositePart(
            int compositeIndex,
            string partName,
            string path,
            string? groups,
            string? processors)
        {
            _ = compositeIndex;
            InputBinding part = new InputBinding(
                path,
                name,
                groups,
                processors,
                null,
                partName,
                false,
                true);
            _bindings.Add(part);
            RefreshResolvedControls();
        }

        /// <summary>Overrides the path of one binding; the override stays until it is removed.</summary>
        public void ApplyBindingOverride(int bindingIndex, string path)
        {
            if (bindingIndex < 0 || bindingIndex >= _bindings.Count)
            {
                throw new ArgumentOutOfRangeException(nameof(bindingIndex));
            }

            InputBinding binding = _bindings[bindingIndex];
            binding.overridePath = path ?? string.Empty;
            _bindings[bindingIndex] = binding;
            RefreshResolvedControls();
        }

        /// <summary>Restores the original path of one binding.</summary>
        public void RemoveBindingOverride(int bindingIndex)
        {
            if (bindingIndex < 0 || bindingIndex >= _bindings.Count)
            {
                throw new ArgumentOutOfRangeException(nameof(bindingIndex));
            }

            InputBinding binding = _bindings[bindingIndex];
            binding.overridePath = string.Empty;
            _bindings[bindingIndex] = binding;
            RefreshResolvedControls();
        }

        public void Enable()
        {
            if (_enabled)
            {
                return;
            }

            _enabled = true;
            _phase = InputActionPhase.Waiting;
            _wasPressedThisFrame = false;
            _wasReleasedThisFrame = false;
            _wasPerformedThisFrame = false;
            RefreshResolvedControls();
            if (!EnabledActions.Contains(this))
            {
                EnabledActions.Add(this);
            }

            EvaluateAndAdvance(true);
        }

        public void Disable()
        {
            if (!_enabled)
            {
                return;
            }

            bool wasActive = _phase == InputActionPhase.Started || _phase == InputActionPhase.Performed;
            _enabled = false;
            EnabledActions.Remove(this);
            _wasPressedThisFrame = false;
            _wasReleasedThisFrame = false;
            _wasPerformedThisFrame = false;
            _currentFloat = 0f;
            _currentVector2 = Vector2.zero;
            _currentMagnitude = 0f;
            if (wasActive)
            {
                FirePhase(InputActionPhase.Canceled);
            }
            _phase = InputActionPhase.Disabled;
            _activeControl = null;
        }

        public TValue ReadValue<TValue>() where TValue : struct
        {
            if (!_enabled)
            {
                return default;
            }

            if (typeof(TValue) == typeof(float))
            {
                float scalar = _currentIsVector2 ? _currentVector2.magnitude : _currentFloat;
                if (scalar is TValue typedFloat)
                {
                    return typedFloat;
                }
            }

            if (typeof(TValue) == typeof(Vector2))
            {
                Vector2 vec = _currentIsVector2 ? _currentVector2 : new Vector2(_currentFloat, 0f);
                if (vec is TValue typedVec)
                {
                    return typedVec;
                }
            }

            return default;
        }

        public bool IsPressed()
        {
            if (!_enabled)
            {
                return false;
            }
            return _phase == InputActionPhase.Performed && _currentMagnitude >= _pressPoint;
        }

        public bool WasPressedThisFrame()
        {
            return _enabled && _wasPressedThisFrame;
        }

        public bool WasReleasedThisFrame()
        {
            return _enabled && _wasReleasedThisFrame;
        }

        public bool WasPerformedThisFrame()
        {
            return _enabled && _wasPerformedThisFrame;
        }

        internal InputValue CreateInputValue()
        {
            return new InputValue(_currentFloat, _currentVector2, _currentIsVector2, _pressPoint);
        }

        internal static void UpdateEnabledActions()
        {
            if (EnabledActions.Count == 0)
            {
                return;
            }

            List<InputAction> snapshot = new List<InputAction>(EnabledActions);
            for (int i = 0; i < snapshot.Count; i++)
            {
                InputAction action = snapshot[i];
                if (action._enabled)
                {
                    action.EvaluateAndAdvance(false);
                }
            }
        }

        internal static void ResetRegistry()
        {
            List<InputAction> snapshot = new List<InputAction>(EnabledActions);
            EnabledActions.Clear();
            for (int i = 0; i < snapshot.Count; i++)
            {
                snapshot[i]._enabled = false;
                snapshot[i]._phase = InputActionPhase.Disabled;
                snapshot[i]._wasPressedThisFrame = false;
                snapshot[i]._wasReleasedThisFrame = false;
                snapshot[i]._wasPerformedThisFrame = false;
                snapshot[i]._currentFloat = 0f;
                snapshot[i]._currentVector2 = Vector2.zero;
                snapshot[i]._currentMagnitude = 0f;
                snapshot[i]._activeControl = null;
            }
        }

        private void EvaluateAndAdvance(bool isInitialCheck)
        {
            if (!isInitialCheck)
            {
                _wasPressedThisFrame = false;
                _wasReleasedThisFrame = false;
                _wasPerformedThisFrame = false;
            }

            SampleResult sample = EvaluateMostActuatedBinding();
            float prevFloat = _currentFloat;
            Vector2 prevVec = _currentVector2;
            bool prevIsVec = _currentIsVector2;

            _currentFloat = sample.FloatValue;
            _currentVector2 = sample.Vector2Value;
            _currentIsVector2 = sample.IsVector2;
            _currentMagnitude = sample.Magnitude;
            if (sample.Control != null)
            {
                _activeControl = sample.Control;
            }

            if (type == InputActionType.Button)
            {
                AdvanceButtonState();
            }
            else
            {
                bool valueChanged = prevIsVec != _currentIsVector2
                    || (_currentIsVector2 ? prevVec != _currentVector2 : MathF.Abs(prevFloat - _currentFloat) > 0.0001f);
                AdvanceValueState(valueChanged);
            }
        }

        private void AdvanceButtonState()
        {
            bool actuated = _currentMagnitude >= _pressPoint;
            if (actuated)
            {
                if (_phase == InputActionPhase.Waiting || _phase == InputActionPhase.Canceled)
                {
                    _wasPressedThisFrame = true;
                    _wasPerformedThisFrame = true;
                    FirePhase(InputActionPhase.Started);
                    FirePhase(InputActionPhase.Performed);
                }
            }
            else if (_phase == InputActionPhase.Started || _phase == InputActionPhase.Performed)
            {
                _wasReleasedThisFrame = true;
                _currentFloat = 0f;
                _currentVector2 = Vector2.zero;
                _currentMagnitude = 0f;
                FirePhase(InputActionPhase.Canceled);
                _phase = InputActionPhase.Waiting;
            }
        }

        private void AdvanceValueState(bool valueChanged)
        {
            bool actuated = _currentMagnitude > 0.0001f;
            if (actuated)
            {
                if (_phase == InputActionPhase.Waiting || _phase == InputActionPhase.Canceled)
                {
                    _wasPressedThisFrame = _currentMagnitude >= _pressPoint;
                    _wasPerformedThisFrame = true;
                    FirePhase(InputActionPhase.Started);
                    FirePhase(InputActionPhase.Performed);
                }
                else if (_phase == InputActionPhase.Performed && valueChanged)
                {
                    _wasPerformedThisFrame = true;
                    FirePhase(InputActionPhase.Performed);
                }
            }
            else if (_phase == InputActionPhase.Started || _phase == InputActionPhase.Performed)
            {
                _wasReleasedThisFrame = true;
                _currentFloat = 0f;
                _currentVector2 = Vector2.zero;
                _currentMagnitude = 0f;
                FirePhase(InputActionPhase.Canceled);
                _phase = InputActionPhase.Waiting;
            }
        }

        private void FirePhase(InputActionPhase targetPhase)
        {
            _phase = targetPhase;
            CallbackContext context = new CallbackContext(this, targetPhase, _activeControl);
            if (targetPhase == InputActionPhase.Started)
            {
                started?.Invoke(context);
            }
            else if (targetPhase == InputActionPhase.Performed)
            {
                performed?.Invoke(context);
            }
            else if (targetPhase == InputActionPhase.Canceled)
            {
                canceled?.Invoke(context);
            }
            actionMap?.NotifyActionTriggered(context);
        }

        private SampleResult EvaluateMostActuatedBinding()
        {
            SampleResult best = default;
            int index = 0;
            while (index < _bindings.Count)
            {
                InputBinding binding = _bindings[index];
                SampleResult current;
                if (binding.isComposite)
                {
                    current = EvaluateComposite(index, out int nextIndex);
                    index = nextIndex;
                }
                else if (binding.isPartOfComposite)
                {
                    index++;
                    continue;
                }
                else
                {
                    current = EvaluateSingleBinding(binding.effectivePath);
                    index++;
                }

                if (current.Magnitude > best.Magnitude || (best.Control == null && current.Control != null && current.Magnitude > 0f))
                {
                    best = current;
                }
            }
            return best;
        }

        private static SampleResult EvaluateSingleBinding(string path)
        {
            InputControl? control = ResolveActiveControl(path);
            if (control == null)
            {
                return default;
            }

            if (control is InputControl<Vector2> vecControl)
            {
                Vector2 v = vecControl.ReadValue();
                return new SampleResult(0f, v, true, v.magnitude, control);
            }

            if (control is InputControl<float> floatControl)
            {
                float f = floatControl.ReadValue();
                return new SampleResult(f, Vector2.zero, false, MathF.Abs(f), control);
            }

            float mag = control.EvaluateMagnitude();
            return new SampleResult(mag, Vector2.zero, false, mag, control);
        }

        private SampleResult EvaluateComposite(int compositeIndex, out int nextIndex)
        {
            string compositeType = _bindings[compositeIndex].path;
            nextIndex = compositeIndex + 1;
            float neg = 0f;
            float pos = 0f;
            float up = 0f;
            float down = 0f;
            float left = 0f;
            float right = 0f;
            InputControl? activePartControl = null;
            float activePartMag = 0f;

            while (nextIndex < _bindings.Count && _bindings[nextIndex].isPartOfComposite)
            {
                InputBinding part = _bindings[nextIndex];
                SampleResult partSample = EvaluateSingleBinding(part.effectivePath);
                if (partSample.Magnitude > activePartMag && partSample.Control != null)
                {
                    activePartMag = partSample.Magnitude;
                    activePartControl = partSample.Control;
                }
                AssignCompositePart(part.name, partSample.FloatValue, ref neg, ref pos, ref up, ref down, ref left, ref right);
                nextIndex++;
            }

            if (Is2DVectorComposite(compositeType))
            {
                Vector2 vec = new Vector2(right - left, up - down);
                if (vec.sqrMagnitude > 1f)
                {
                    vec = vec.normalized;
                }
                return new SampleResult(0f, vec, true, vec.magnitude, activePartControl);
            }

            float axis = pos - neg;
            return new SampleResult(axis, Vector2.zero, false, MathF.Abs(axis), activePartControl);
        }

        private static void AssignCompositePart(
            string partName,
            float value,
            ref float neg,
            ref float pos,
            ref float up,
            ref float down,
            ref float left,
            ref float right)
        {
            if (string.Equals(partName, "negative", StringComparison.OrdinalIgnoreCase))
            {
                neg = MathF.Max(neg, value);
            }
            else if (string.Equals(partName, "positive", StringComparison.OrdinalIgnoreCase))
            {
                pos = MathF.Max(pos, value);
            }
            else if (string.Equals(partName, "up", StringComparison.OrdinalIgnoreCase))
            {
                up = MathF.Max(up, value);
            }
            else if (string.Equals(partName, "down", StringComparison.OrdinalIgnoreCase))
            {
                down = MathF.Max(down, value);
            }
            else if (string.Equals(partName, "left", StringComparison.OrdinalIgnoreCase))
            {
                left = MathF.Max(left, value);
            }
            else if (string.Equals(partName, "right", StringComparison.OrdinalIgnoreCase))
            {
                right = MathF.Max(right, value);
            }
        }

        private static bool Is2DVectorComposite(string compositeType)
        {
            return compositeType.StartsWith("2DVector", StringComparison.OrdinalIgnoreCase)
                || compositeType.StartsWith("Dpad", StringComparison.OrdinalIgnoreCase);
        }

        private static InputControl? ResolveActiveControl(string path)
        {
            InputControl? control = InputSystem.FindControl(path);
            if (control == null)
            {
                return null;
            }
            if (control.device is Gamepad && Gamepad.current == null)
            {
                return null;
            }
            return control;
        }

        private void RefreshResolvedControls()
        {
            _resolvedControls.Clear();
            for (int i = 0; i < _bindings.Count; i++)
            {
                if (_bindings[i].isComposite)
                {
                    continue;
                }
                InputControl? control = InputSystem.FindControl(_bindings[i].effectivePath);
                if (control != null && !_resolvedControls.Contains(control))
                {
                    _resolvedControls.Add(control);
                }
            }
        }

        private static float ParsePressPoint(string? interactions, float fallback)
        {
            if (string.IsNullOrWhiteSpace(interactions))
            {
                return fallback;
            }
            int idx = interactions.IndexOf("pressPoint=", StringComparison.OrdinalIgnoreCase);
            if (idx < 0)
            {
                return fallback;
            }
            int start = idx + "pressPoint=".Length;
            int end = start;
            while (end < interactions.Length && (char.IsDigit(interactions[end]) || interactions[end] == '.'))
            {
                end++;
            }
            if (end > start
                && float.TryParse(interactions.AsSpan(start, end - start), NumberStyles.Float, CultureInfo.InvariantCulture, out float parsed)
                && parsed > 0f)
            {
                return parsed;
            }
            return fallback;
        }

        private readonly struct SampleResult
        {
            public SampleResult(float floatValue, Vector2 vector2Value, bool isVector2, float magnitude, InputControl? control)
            {
                FloatValue = floatValue;
                Vector2Value = vector2Value;
                IsVector2 = isVector2;
                Magnitude = magnitude;
                Control = control;
            }

            public float FloatValue { get; }
            public Vector2 Vector2Value { get; }
            public bool IsVector2 { get; }
            public float Magnitude { get; }
            public InputControl? Control { get; }
        }

        public readonly struct CallbackContext
        {
            internal CallbackContext(InputAction action, InputActionPhase phase, InputControl? control)
            {
                this.action = action;
                this.phase = phase;
                this.control = control;
            }

            public InputAction action { get; }
            public InputActionPhase phase { get; }
            public InputControl? control { get; }
            public bool started => phase == InputActionPhase.Started;
            public bool performed => phase == InputActionPhase.Performed;
            public bool canceled => phase == InputActionPhase.Canceled;

            public TValue ReadValue<TValue>() where TValue : struct
            {
                return action != null ? action.ReadValue<TValue>() : default;
            }

            public bool ReadValueAsButton()
            {
                return action != null && action.IsPressed();
            }
        }

        public readonly struct CompositeSyntax
        {
            private readonly InputAction _action;
            private readonly int _compositeIndex;

            internal CompositeSyntax(InputAction action, int compositeIndex)
            {
                _action = action;
                _compositeIndex = compositeIndex;
            }

            public CompositeSyntax With(string name, string binding, string? groups = null, string? processors = null)
            {
                _action.AddCompositePart(_compositeIndex, name, binding, groups, processors);
                return this;
            }
        }
    }
}
