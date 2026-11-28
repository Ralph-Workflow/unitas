using System;
using System.Collections.Generic;

namespace UnityEngine.InputSystem
{
    /// <summary>Outcome of an interactive rebind.</summary>
    public enum RebindStatus
    {
        Waiting = 0,
        Completed = 1,
        Canceled = 2,
        TimedOut = 3,
    }

    /// <summary>
    /// Rebinds one binding of an <see cref="InputAction"/> to the next control the
    /// user actuates. The operation listens for control activity on the managed
    /// input layer, so it needs no device: any driver that feeds the input system
    /// (the bridge host in the headless tests) completes the rebind.
    /// </summary>
    public sealed class RebindOperation
    {
        private readonly List<string> _excludedPaths = new List<string>();
        private readonly InputAction _target;
        private readonly int _bindingIndex;
        private readonly float _timeoutSeconds;
        private float _elapsedSeconds;

        private RebindOperation(InputAction target, int bindingIndex, float timeoutSeconds)
        {
            _target = target;
            _bindingIndex = bindingIndex;
            _timeoutSeconds = timeoutSeconds;
        }

        /// <summary>The action whose binding is being rebound.</summary>
        public InputAction target => _target;

        /// <summary>Index of the rebound binding inside <see cref="target"/>.</summary>
        public int bindingIndex => _bindingIndex;

        public RebindStatus status { get; private set; } = RebindStatus.Waiting;

        public bool isDone => status != RebindStatus.Waiting;

        /// <summary>The control the rebind completed with, or null while waiting.</summary>
        public InputControl? boundControl { get; private set; }

        /// <summary>The path applied to the rebound binding, or an empty string.</summary>
        public string boundPath => boundControl?.path ?? string.Empty;

        public event Action<RebindOperation>? completed;

        public event Action<RebindOperation>? canceled;

        public event Action<RebindOperation>? timedOut;

        /// <summary>
        /// Starts listening for control activity on <paramref name="target"/>.
        /// </summary>
        /// <param name="target">Action that owns the binding to rebind.</param>
        /// <param name="bindingIndex">Index of the binding to rebind.</param>
        /// <param name="timeoutSeconds">Seconds before the operation times out; 0 never times out.</param>
        /// <param name="excludedControlPaths">Control paths that may not complete the rebind.</param>
        public static RebindOperation Start(
            InputAction target,
            int bindingIndex = 0,
            float timeoutSeconds = 0f,
            params string[] excludedControlPaths)
        {
            if (target == null)
            {
                throw new ArgumentNullException(nameof(target));
            }

            if (bindingIndex < 0 || bindingIndex >= target.bindings.Count)
            {
                throw new ArgumentOutOfRangeException(nameof(bindingIndex));
            }

            RebindOperation operation = new RebindOperation(
                target,
                bindingIndex,
                timeoutSeconds > 0f ? timeoutSeconds : 0f);

            if (excludedControlPaths != null)
            {
                for (int i = 0; i < excludedControlPaths.Length; i++)
                {
                    string excluded = excludedControlPaths[i];
                    if (!string.IsNullOrWhiteSpace(excluded))
                    {
                        operation._excludedPaths.Add(NormalizePath(excluded));
                    }
                }
            }

            InputSystem.OnUpdate += operation.OnInputUpdate;
            return operation;
        }

        /// <summary>
        /// Completes the rebind with <paramref name="control"/> and applies the
        /// resulting binding override to the target action.
        /// </summary>
        public void Complete(InputControl control)
        {
            if (control == null)
            {
                throw new ArgumentNullException(nameof(control));
            }

            if (isDone)
            {
                throw new InvalidOperationException("The rebind operation has already finished.");
            }

            _target.ApplyBindingOverride(_bindingIndex, control.path);
            boundControl = control;
            Finish(RebindStatus.Completed);
        }

        /// <summary>
        /// Aborts the rebind. The bindings of the target action are left unchanged.
        /// </summary>
        public void Cancel()
        {
            if (isDone)
            {
                return;
            }

            Finish(RebindStatus.Canceled);
        }

        private void OnInputUpdate()
        {
            if (isDone)
            {
                return;
            }

            _elapsedSeconds += Time.unscaledDeltaTime;
            if (_timeoutSeconds > 0f && _elapsedSeconds >= _timeoutSeconds)
            {
                Finish(RebindStatus.TimedOut);
                return;
            }

            InputControl? actuated = FindActuatedControl();
            if (actuated != null)
            {
                Complete(actuated);
            }
        }

        private InputControl? FindActuatedControl()
        {
            IReadOnlyList<InputDevice> devices = InputSystem.devices;
            for (int i = 0; i < devices.Count; i++)
            {
                IReadOnlyList<InputControl> controls = devices[i].allControls;
                for (int j = 0; j < controls.Count; j++)
                {
                    InputControl control = controls[j];
                    if (!control.wasPressedThisFrame)
                    {
                        continue;
                    }

                    if (IsAggregate(control) || IsExcluded(control))
                    {
                        continue;
                    }

                    return control;
                }
            }

            return null;
        }

        private bool IsExcluded(InputControl control)
        {
            string path = NormalizePath(control.path);
            for (int i = 0; i < _excludedPaths.Count; i++)
            {
                if (string.Equals(_excludedPaths[i], path, StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            return false;
        }

        private void Finish(RebindStatus finalStatus)
        {
            status = finalStatus;
            InputSystem.OnUpdate -= OnInputUpdate;
            if (finalStatus == RebindStatus.Completed)
            {
                completed?.Invoke(this);
            }
            else if (finalStatus == RebindStatus.Canceled)
            {
                canceled?.Invoke(this);
            }
            else if (finalStatus == RebindStatus.TimedOut)
            {
                timedOut?.Invoke(this);
            }
        }

        /// <summary>
        /// Keyboard.anyKey stands for "some key", so it never identifies the
        /// control the user rebound.
        /// </summary>
        private static bool IsAggregate(InputControl control)
        {
            return control.device is Keyboard keyboard && ReferenceEquals(control, keyboard.anyKey);
        }

        /// <summary>Turns "&lt;Keyboard&gt;/space" and "Keyboard/space" into "/Keyboard/space".</summary>
        private static string NormalizePath(string path)
        {
            string trimmed = path.Trim();
            if (trimmed.StartsWith('<'))
            {
                int closeBracket = trimmed.IndexOf('>', StringComparison.Ordinal);
                if (closeBracket > 0)
                {
                    trimmed = trimmed.Substring(1, closeBracket - 1) + trimmed.Substring(closeBracket + 1);
                }
            }

            if (!trimmed.StartsWith('/'))
            {
                trimmed = "/" + trimmed;
            }

            return trimmed;
        }
    }
}
