using System;

namespace UnityEngine.InputSystem
{
    public enum PlayerNotifications
    {
        SendMessages = 0,
        BroadcastMessages = 1,
        InvokeUnityEvents = 2,
        InvokeCSharpEvents = 3
    }

    public sealed class PlayerInput : MonoBehaviour
    {
        private InputActionAsset? _actions;
        private InputActionMap? _currentActionMap;
        private InputActionMap? _subscribedMap;
        private bool _inputActive;

        public InputActionAsset? actions
        {
            get => _actions;
            set
            {
                if (ReferenceEquals(_actions, value))
                {
                    return;
                }
                DeactivateInput();
                _actions = value;
                _currentActionMap = null;
                if (isActiveAndEnabled && _actions != null)
                {
                    ActivateInput();
                }
            }
        }

        public string? defaultActionMap { get; set; }

        public PlayerNotifications notificationBehavior { get; set; } = PlayerNotifications.SendMessages;

        public InputActionMap? currentActionMap
        {
            get => _currentActionMap;
            set
            {
                if (ReferenceEquals(_currentActionMap, value))
                {
                    return;
                }
                SwitchToMap(value);
            }
        }

        public bool inputIsActive => _inputActive;

        public event Action<InputAction.CallbackContext>? onActionTriggered;

        public override void Awake()
        {
            base.Awake();
            EnsureDefaultMapSelected();
        }

        public override void OnEnable()
        {
            base.OnEnable();
            ActivateInput();
        }

        public override void OnDisable()
        {
            DeactivateInput();
            base.OnDisable();
        }

        public override void OnDestroy()
        {
            DeactivateInput();
            base.OnDestroy();
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            if (isEnabled && isActiveAndEnabled)
            {
                ActivateInput();
            }
            else if (!isEnabled)
            {
                DeactivateInput();
            }
        }

        public void ActivateInput()
        {
            EnsureDefaultMapSelected();
            if (_currentActionMap == null)
            {
                return;
            }
            _inputActive = true;
            SubscribeToMap(_currentActionMap);
            _currentActionMap.Enable();
        }

        public void DeactivateInput()
        {
            _inputActive = false;
            UnsubscribeFromCurrentMap();
            _currentActionMap?.Disable();
        }

        public void SwitchCurrentActionMap(string mapNameOrId)
        {
            if (_actions == null || string.IsNullOrEmpty(mapNameOrId))
            {
                return;
            }

            InputActionMap? target = _actions.FindActionMap(mapNameOrId, false);
            if (target == null)
            {
                return;
            }

            SwitchToMap(target);
        }

        private void SwitchToMap(InputActionMap? nextMap)
        {
            bool wasActive = _inputActive || isActiveAndEnabled;
            UnsubscribeFromCurrentMap();
            _currentActionMap?.Disable();
            _currentActionMap = nextMap;
            if (wasActive && _currentActionMap != null)
            {
                _inputActive = true;
                SubscribeToMap(_currentActionMap);
                _currentActionMap.Enable();
            }
        }

        private void EnsureDefaultMapSelected()
        {
            if (_currentActionMap != null || _actions == null)
            {
                return;
            }

            if (!string.IsNullOrEmpty(defaultActionMap))
            {
                _currentActionMap = _actions.FindActionMap(defaultActionMap!, false);
            }

            if (_currentActionMap == null && _actions.actionMaps.Count > 0)
            {
                _currentActionMap = _actions.actionMaps[0];
            }
        }

        private void SubscribeToMap(InputActionMap map)
        {
            if (ReferenceEquals(_subscribedMap, map))
            {
                return;
            }
            UnsubscribeFromCurrentMap();
            _subscribedMap = map;
            _subscribedMap.actionTriggered += OnMapActionTriggered;
        }

        private void UnsubscribeFromCurrentMap()
        {
            if (_subscribedMap != null)
            {
                _subscribedMap.actionTriggered -= OnMapActionTriggered;
                _subscribedMap = null;
            }
        }

        private void OnMapActionTriggered(InputAction.CallbackContext context)
        {
            if (!_inputActive || IsDestroyed)
            {
                return;
            }

            if (notificationBehavior == PlayerNotifications.InvokeCSharpEvents
                || notificationBehavior == PlayerNotifications.InvokeUnityEvents)
            {
                onActionTriggered?.Invoke(context);
                return;
            }

            InputAction action = context.action;
            if (action == null || string.IsNullOrEmpty(action.name))
            {
                return;
            }

            bool shouldNotify = context.phase == InputActionPhase.Performed
                || (context.phase == InputActionPhase.Canceled && action.type == InputActionType.Value);
            if (!shouldNotify)
            {
                return;
            }

            GameObject? owner = OwnerOrNull;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }

            string methodName = "On" + action.name;
            InputValue inputValue = action.CreateInputValue();
            if (notificationBehavior == PlayerNotifications.BroadcastMessages)
            {
                owner.BroadcastMessage(methodName, inputValue, SendMessageOptions.DontRequireReceiver);
            }
            else
            {
                owner.SendMessage(methodName, inputValue, SendMessageOptions.DontRequireReceiver);
            }
        }
    }
}
