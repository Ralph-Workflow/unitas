using System;
using System.Collections.Generic;

namespace UnityEngine.InputSystem
{
    public abstract class InputDevice : InputControl
    {
        private readonly List<InputControl> _allControls = new List<InputControl>();
        private readonly Dictionary<string, InputControl> _controlsByName =
            new Dictionary<string, InputControl>(StringComparer.OrdinalIgnoreCase);

        protected InputDevice(string name, string path)
            : base(name, path, null)
        {
            device = this;
        }

        public IReadOnlyList<InputControl> allControls => _allControls;

        public override bool isPressed
        {
            get
            {
                for (int i = 0; i < _allControls.Count; i++)
                {
                    if (_allControls[i].isPressed)
                    {
                        return true;
                    }
                }
                return false;
            }
        }

        public override bool wasPressedThisFrame
        {
            get
            {
                for (int i = 0; i < _allControls.Count; i++)
                {
                    if (_allControls[i].wasPressedThisFrame)
                    {
                        return true;
                    }
                }
                return false;
            }
        }

        public override bool wasReleasedThisFrame
        {
            get
            {
                for (int i = 0; i < _allControls.Count; i++)
                {
                    if (_allControls[i].wasReleasedThisFrame)
                    {
                        return true;
                    }
                }
                return false;
            }
        }

        public InputControl? this[string controlPath] => TryGetControl(controlPath);

        public virtual InputControl? TryGetControl(string controlPath)
        {
            if (string.IsNullOrWhiteSpace(controlPath))
            {
                return null;
            }

            string trimmed = controlPath.Trim().TrimStart('/');
            if (trimmed.Length == 0)
            {
                return null;
            }

            if (_controlsByName.TryGetValue(trimmed, out InputControl? direct))
            {
                return direct;
            }

            int slashIndex = trimmed.IndexOf('/', StringComparison.Ordinal);
            if (slashIndex <= 0)
            {
                return null;
            }

            string head = trimmed.Substring(0, slashIndex);
            string tail = trimmed.Substring(slashIndex + 1);
            if (!_controlsByName.TryGetValue(head, out InputControl? current))
            {
                return null;
            }

            string[] segments = tail.Split('/', StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < segments.Length; i++)
            {
                current = current.TryGetChildControl(segments[i]);
                if (current == null)
                {
                    return null;
                }
            }

            return current;
        }

        public override InputControl? TryGetChildControl(string childName)
        {
            return TryGetControl(childName);
        }

        protected TControl RegisterControl<TControl>(TControl control, params string[] aliases)
            where TControl : InputControl
        {
            control.device = this;
            _allControls.Add(control);
            _controlsByName[control.name] = control;
            if (aliases != null)
            {
                for (int i = 0; i < aliases.Length; i++)
                {
                    if (!string.IsNullOrEmpty(aliases[i]))
                    {
                        _controlsByName[aliases[i]] = control;
                    }
                }
            }
            return control;
        }

        protected void RegisterControlAlias(string alias, InputControl control)
        {
            if (!string.IsNullOrEmpty(alias))
            {
                _controlsByName[alias] = control;
            }
        }
    }
}
