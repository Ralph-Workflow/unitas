using System;
using System.Collections.Generic;

namespace UnityEngine.InputSystem
{
    public sealed class InputActionMap
    {
        private readonly List<InputAction> _actions = new List<InputAction>();

        public InputActionMap()
            : this(string.Empty)
        {
        }

        public InputActionMap(string? name)
        {
            this.name = name ?? string.Empty;
            id = Guid.NewGuid();
        }

        public Guid id { get; }

        public string name { get; internal set; }

        public InputActionAsset? asset { get; internal set; }

        public IReadOnlyList<InputAction> actions => _actions;

        public bool enabled
        {
            get
            {
                if (_actions.Count == 0)
                {
                    return false;
                }
                for (int i = 0; i < _actions.Count; i++)
                {
                    if (_actions[i].enabled)
                    {
                        return true;
                    }
                }
                return false;
            }
        }

        public event Action<InputAction.CallbackContext>? actionTriggered;

        public InputAction this[string actionNameOrId]
        {
            get
            {
                InputAction? found = FindAction(actionNameOrId, true);
                return found!;
            }
        }

        public InputAction AddAction(
            string name,
            InputActionType type = InputActionType.Value,
            string? binding = null,
            string? interactions = null,
            string? processors = null,
            string? expectedControlType = null)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("Action name cannot be null or empty.", nameof(name));
            }

            InputAction action = new InputAction(name, type, binding, interactions, processors, expectedControlType)
            {
                actionMap = this
            };
            _actions.Add(action);
            return action;
        }

        public void AddAction(InputAction action)
        {
            if (action == null)
            {
                throw new ArgumentNullException(nameof(action));
            }
            action.actionMap = this;
            if (!_actions.Contains(action))
            {
                _actions.Add(action);
            }
        }

        public InputAction? FindAction(string actionNameOrId, bool throwIfNotFound = false)
        {
            if (!string.IsNullOrEmpty(actionNameOrId))
            {
                for (int i = 0; i < _actions.Count; i++)
                {
                    InputAction candidate = _actions[i];
                    if (string.Equals(candidate.name, actionNameOrId, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(candidate.id.ToString("D"), actionNameOrId, StringComparison.OrdinalIgnoreCase))
                    {
                        return candidate;
                    }
                }
            }

            if (throwIfNotFound)
            {
                throw new ArgumentException("Cannot find action '" + actionNameOrId + "' in map '" + name + "'.", nameof(actionNameOrId));
            }

            return null;
        }

        public void Enable()
        {
            for (int i = 0; i < _actions.Count; i++)
            {
                _actions[i].Enable();
            }
        }

        public void Disable()
        {
            for (int i = 0; i < _actions.Count; i++)
            {
                _actions[i].Disable();
            }
        }

        internal void NotifyActionTriggered(InputAction.CallbackContext context)
        {
            actionTriggered?.Invoke(context);
        }
    }
}
