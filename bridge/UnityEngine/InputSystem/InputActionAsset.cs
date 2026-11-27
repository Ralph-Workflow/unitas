using System;
using System.Collections.Generic;

namespace UnityEngine.InputSystem
{
    public sealed class InputActionAsset : ScriptableObject
    {
        private readonly List<InputActionMap> _actionMaps = new List<InputActionMap>();

        public IReadOnlyList<InputActionMap> actionMaps => _actionMaps;

        public bool enabled
        {
            get
            {
                for (int i = 0; i < _actionMaps.Count; i++)
                {
                    if (_actionMaps[i].enabled)
                    {
                        return true;
                    }
                }
                return false;
            }
        }

        public InputAction this[string actionNameOrId]
        {
            get
            {
                InputAction? found = FindAction(actionNameOrId, true);
                return found!;
            }
        }

        public InputActionMap AddActionMap(string name)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("Action map name cannot be null or empty.", nameof(name));
            }

            InputActionMap map = new InputActionMap(name);
            AddActionMap(map);
            return map;
        }

        public void AddActionMap(InputActionMap map)
        {
            if (map == null)
            {
                throw new ArgumentNullException(nameof(map));
            }

            map.asset = this;
            if (!_actionMaps.Contains(map))
            {
                _actionMaps.Add(map);
            }
        }

        public InputActionMap? FindActionMap(string nameOrId, bool throwIfNotFound = false)
        {
            if (!string.IsNullOrEmpty(nameOrId))
            {
                for (int i = 0; i < _actionMaps.Count; i++)
                {
                    InputActionMap candidate = _actionMaps[i];
                    if (string.Equals(candidate.name, nameOrId, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(candidate.id.ToString("D"), nameOrId, StringComparison.OrdinalIgnoreCase))
                    {
                        return candidate;
                    }
                }
            }

            if (throwIfNotFound)
            {
                throw new ArgumentException("Cannot find action map '" + nameOrId + "'.", nameof(nameOrId));
            }

            return null;
        }

        public InputAction? FindAction(string actionNameOrId, bool throwIfNotFound = false)
        {
            if (!string.IsNullOrEmpty(actionNameOrId))
            {
                int slash = actionNameOrId.IndexOf('/', StringComparison.Ordinal);
                if (slash > 0 && slash < actionNameOrId.Length - 1)
                {
                    string mapName = actionNameOrId.Substring(0, slash);
                    string actionName = actionNameOrId.Substring(slash + 1);
                    InputActionMap? map = FindActionMap(mapName, false);
                    InputAction? qualified = map?.FindAction(actionName, false);
                    if (qualified != null)
                    {
                        return qualified;
                    }
                }
                else
                {
                    for (int i = 0; i < _actionMaps.Count; i++)
                    {
                        InputAction? found = _actionMaps[i].FindAction(actionNameOrId, false);
                        if (found != null)
                        {
                            return found;
                        }
                    }
                }
            }

            if (throwIfNotFound)
            {
                throw new ArgumentException("Cannot find action '" + actionNameOrId + "'.", nameof(actionNameOrId));
            }

            return null;
        }

        public void Enable()
        {
            for (int i = 0; i < _actionMaps.Count; i++)
            {
                _actionMaps[i].Enable();
            }
        }

        public void Disable()
        {
            for (int i = 0; i < _actionMaps.Count; i++)
            {
                _actionMaps[i].Disable();
            }
        }
    }
}
