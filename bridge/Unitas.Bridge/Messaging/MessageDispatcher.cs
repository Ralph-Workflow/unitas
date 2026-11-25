using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Messaging
{
    public enum InputHandlingMode
    {
        LegacyInputManager = 0,
        InputSystemPackage = 1,
        Both = 2
    }

    public sealed class PlatformLimitedDiagnostic
    {
        public PlatformLimitedDiagnostic(
            RuntimePlatform targetPlatform,
            string apiName,
            string callingMember,
            string compatibilityState,
            string message)
        {
            TargetPlatform = targetPlatform;
            ApiName = apiName ?? string.Empty;
            CallingMember = callingMember ?? string.Empty;
            CompatibilityState = compatibilityState ?? "Platform-limited";
            Message = message ?? string.Empty;
        }

        public RuntimePlatform TargetPlatform { get; }

        public string ApiName { get; }

        public string CallingMember { get; }

        public string CompatibilityState { get; }

        public string Message { get; }
    }

    public static class MessageDispatcher
    {
        private static readonly Dictionary<int, IUnitasMessageHandler> InstanceHandlers =
            new Dictionary<int, IUnitasMessageHandler>();

        private static readonly List<Func<MonoBehaviour, IUnitasMessageHandler?>> HandlerFactories =
            new List<Func<MonoBehaviour, IUnitasMessageHandler?>>();

        private static readonly Dictionary<int, HashSet<int>> RendererVisibleCameras =
            new Dictionary<int, HashSet<int>>();

        private static readonly HashSet<int> VisibleRenderers =
            new HashSet<int>();

        private static readonly List<PlatformLimitedDiagnostic> PlatformDiagnostics =
            new List<PlatformLimitedDiagnostic>();

        static MessageDispatcher()
        {
            EnsureInstalled();
        }

        public static InputHandlingMode ActiveInputHandling { get; set; } = InputHandlingMode.LegacyInputManager;

        public static RuntimePlatform SimulatedTargetPlatform { get; set; } = RuntimePlatform.OSXEditor;

        public static IReadOnlyList<PlatformLimitedDiagnostic> PlatformLimitedDiagnostics => PlatformDiagnostics;

        public static bool IsLegacyMouseMessagingEnabled =>
            ActiveInputHandling == InputHandlingMode.LegacyInputManager
            || ActiveInputHandling == InputHandlingMode.Both;

        public static bool IsBrowserTarget =>
            SimulatedTargetPlatform == RuntimePlatform.WebGLPlayer
            || Application.platform == RuntimePlatform.WebGLPlayer;

        public static void EnsureInstalled()
        {
            GameObject.MessageHandlerResolver = ResolveHandler;
        }

        public static void RegisterHandler(MonoBehaviour target, IUnitasMessageHandler handler)
        {
            ArgumentNullException.ThrowIfNull(target);
            ArgumentNullException.ThrowIfNull(handler);

            EnsureInstalled();
            InstanceHandlers[target.GetInstanceID()] = handler;
        }

        public static void RegisterHandler<TScript>(TScript target, Action<TScript> callback, string methodName)
            where TScript : MonoBehaviour
        {
            ArgumentNullException.ThrowIfNull(target);
            ArgumentNullException.ThrowIfNull(callback);

            if (string.IsNullOrEmpty(methodName))
            {
                throw new ArgumentException("Method name cannot be null or empty.", nameof(methodName));
            }

            EnsureInstalled();
            int id = target.GetInstanceID();
            if (!InstanceHandlers.TryGetValue(id, out IUnitasMessageHandler? existing)
                || !(existing is DelegateMessageHandler<TScript> typedHandler))
            {
                typedHandler = new DelegateMessageHandler<TScript>(target);
                InstanceHandlers[id] = typedHandler;
            }

            typedHandler.AddVoid(methodName, callback);
        }

        public static void RegisterHandler<TScript, TArg>(TScript target, Action<TScript, TArg> callback, string methodName)
            where TScript : MonoBehaviour
        {
            ArgumentNullException.ThrowIfNull(target);
            ArgumentNullException.ThrowIfNull(callback);

            if (string.IsNullOrEmpty(methodName))
            {
                throw new ArgumentException("Method name cannot be null or empty.", nameof(methodName));
            }

            EnsureInstalled();
            int id = target.GetInstanceID();
            if (!InstanceHandlers.TryGetValue(id, out IUnitasMessageHandler? existing)
                || !(existing is DelegateMessageHandler<TScript> typedHandler))
            {
                typedHandler = new DelegateMessageHandler<TScript>(target);
                InstanceHandlers[id] = typedHandler;
            }

            typedHandler.AddArg(methodName, callback);
        }

        public static void RegisterFactory(Func<MonoBehaviour, IUnitasMessageHandler?> factory)
        {
            ArgumentNullException.ThrowIfNull(factory);

            EnsureInstalled();
            HandlerFactories.Add(factory);
        }

        public static void UnregisterHandler(MonoBehaviour target)
        {
            if (target is null)
            {
                return;
            }

            InstanceHandlers.Remove(target.GetInstanceID());
        }

        public static void Clear()
        {
            InstanceHandlers.Clear();
            HandlerFactories.Clear();
            RendererVisibleCameras.Clear();
            VisibleRenderers.Clear();
            PlatformDiagnostics.Clear();
            ActiveInputHandling = InputHandlingMode.LegacyInputManager;
            SimulatedTargetPlatform = RuntimePlatform.OSXEditor;
            EnsureInstalled();
        }

        public static void ClearPlatformLimitedDiagnostics()
        {
            PlatformDiagnostics.Clear();
        }

        public static bool IsRendererVisible(GameObject? target)
        {
            if (target is null || target.IsDestroyed)
            {
                return false;
            }

            return VisibleRenderers.Contains(target.GetInstanceID());
        }

        public static bool NotifyCameraVisibility(GameObject target, int cameraId, bool isVisibleToCamera)
        {
            ArgumentNullException.ThrowIfNull(target);
            if (target.IsDestroyed || !target.activeInHierarchy)
            {
                return false;
            }

            int id = target.GetInstanceID();
            if (!RendererVisibleCameras.TryGetValue(id, out HashSet<int>? cameras))
            {
                cameras = new HashSet<int>();
                RendererVisibleCameras[id] = cameras;
            }

            bool wasVisible = VisibleRenderers.Contains(id);
            if (isVisibleToCamera)
            {
                cameras.Add(cameraId);
            }
            else
            {
                cameras.Remove(cameraId);
            }

            bool isNowVisible = cameras.Count > 0;
            if (!wasVisible && isNowVisible)
            {
                VisibleRenderers.Add(id);
                return DispatchVisibilityMessage(target, becameVisible: true);
            }

            if (wasVisible && !isNowVisible)
            {
                VisibleRenderers.Remove(id);
                return DispatchVisibilityMessage(target, becameVisible: false);
            }

            return false;
        }

        public static bool NotifyBecameVisible(GameObject target)
        {
            ArgumentNullException.ThrowIfNull(target);
            if (target.IsDestroyed || !target.activeInHierarchy)
            {
                return false;
            }

            VisibleRenderers.Add(target.GetInstanceID());
            return DispatchVisibilityMessage(target, becameVisible: true);
        }

        public static bool NotifyBecameInvisible(GameObject target)
        {
            ArgumentNullException.ThrowIfNull(target);
            if (target.IsDestroyed || !target.activeInHierarchy)
            {
                return false;
            }

            int id = target.GetInstanceID();
            VisibleRenderers.Remove(id);
            RendererVisibleCameras.Remove(id);
            return DispatchVisibilityMessage(target, becameVisible: false);
        }

        public static bool DispatchMouseEvent(GameObject target, string mouseMessageName)
        {
            ArgumentNullException.ThrowIfNull(target);
            if (string.IsNullOrEmpty(mouseMessageName))
            {
                throw new ArgumentException("Mouse message name cannot be null or empty.", nameof(mouseMessageName));
            }

            if (target.IsDestroyed || !target.activeInHierarchy)
            {
                return false;
            }

            if (!IsLegacyMouseMessagingEnabled)
            {
                Debug.LogWarning(
                    "Suppressed "
                    + mouseMessageName
                    + " on '"
                    + target.name
                    + "' because Active Input Handling is set to Input System Package (New) only.",
                    target);
                return false;
            }

            Collider? collider = target.GetComponent<Collider>();
            if (collider is null || !collider.enabled)
            {
                return false;
            }

            return DispatchLifecycleMessageToBehaviours(
                target,
                mouseMessageName,
                behaviour => InvokeBuiltInMouseCallback(behaviour, mouseMessageName));
        }

        public static bool NotifyMouseEnter(GameObject target) => DispatchMouseEvent(target, "OnMouseEnter");

        public static bool NotifyMouseOver(GameObject target) => DispatchMouseEvent(target, "OnMouseOver");

        public static bool NotifyMouseExit(GameObject target) => DispatchMouseEvent(target, "OnMouseExit");

        public static bool NotifyMouseDown(GameObject target) => DispatchMouseEvent(target, "OnMouseDown");

        public static bool NotifyMouseDrag(GameObject target) => DispatchMouseEvent(target, "OnMouseDrag");

        public static bool NotifyMouseUp(GameObject target) => DispatchMouseEvent(target, "OnMouseUp");

        public static bool NotifyMouseUpAsButton(GameObject target) => DispatchMouseEvent(target, "OnMouseUpAsButton");

        public static bool TryStartThread(
            Action threadStart,
            string callingMember,
            out PlatformLimitedDiagnostic? diagnostic,
            string apiName = "System.Threading.Thread.Start",
            UnityEngine.Object? origin = null)
        {
            ArgumentNullException.ThrowIfNull(threadStart);
            if (string.IsNullOrEmpty(callingMember))
            {
                throw new ArgumentException("Calling member must be non-empty.", nameof(callingMember));
            }

            string effectiveApi = string.IsNullOrEmpty(apiName) ? "System.Threading.Thread.Start" : apiName;
            if (IsBrowserTarget)
            {
                diagnostic = ReportBrowserThreadingLimitation(effectiveApi, callingMember, origin);
                return false;
            }

            diagnostic = null;
            System.Threading.Thread worker = new System.Threading.Thread(() => threadStart())
            {
                IsBackground = true
            };
            worker.Start();
            worker.Join();
            return true;
        }

        public static PlatformLimitedDiagnostic ReportBrowserThreadingLimitation(
            string apiName,
            string callingMember,
            UnityEngine.Object? origin = null)
        {
            if (string.IsNullOrEmpty(apiName))
            {
                throw new ArgumentException("API name must be non-empty.", nameof(apiName));
            }

            if (string.IsNullOrEmpty(callingMember))
            {
                throw new ArgumentException("Calling member must be non-empty.", nameof(callingMember));
            }

            RuntimePlatform target = IsBrowserTarget ? RuntimePlatform.WebGLPlayer : SimulatedTargetPlatform;
            string message =
                "Platform-limited on "
                + target
                + ": API '"
                + apiName
                + "' called from '"
                + callingMember
                + "' is unavailable on targets without threads.";
            PlatformLimitedDiagnostic diagnostic = new PlatformLimitedDiagnostic(
                target,
                apiName,
                callingMember,
                "Platform-limited",
                message);
            PlatformDiagnostics.Add(diagnostic);
            Debug.LogWarning(message, origin);
            return diagnostic;
        }

        private static bool DispatchVisibilityMessage(GameObject target, bool becameVisible)
        {
            string messageName = becameVisible ? "OnBecameVisible" : "OnBecameInvisible";
            return DispatchLifecycleMessageToBehaviours(
                target,
                messageName,
                behaviour =>
                {
                    if (becameVisible)
                    {
                        behaviour.OnBecameVisible();
                    }
                    else
                    {
                        behaviour.OnBecameInvisible();
                    }
                });
        }

        private static void InvokeBuiltInMouseCallback(MonoBehaviour behaviour, string mouseMessageName)
        {
            switch (mouseMessageName)
            {
                case "OnMouseDown":
                    behaviour.OnMouseDown();
                    break;
                case "OnMouseUp":
                    behaviour.OnMouseUp();
                    break;
                case "OnMouseUpAsButton":
                    behaviour.OnMouseUpAsButton();
                    break;
                case "OnMouseEnter":
                    behaviour.OnMouseEnter();
                    break;
                case "OnMouseExit":
                    behaviour.OnMouseExit();
                    break;
                case "OnMouseOver":
                    behaviour.OnMouseOver();
                    break;
                case "OnMouseDrag":
                    behaviour.OnMouseDrag();
                    break;
            }
        }

        private static bool DispatchLifecycleMessageToBehaviours(
            GameObject target,
            string messageName,
            Action<MonoBehaviour> virtualFallback)
        {
            EnsureInstalled();
            IReadOnlyList<Component> components = target.GetComponents<Component>();
            bool delivered = false;
            for (int i = 0; i < components.Count; i++)
            {
                if (!(components[i] is MonoBehaviour behaviour) || !behaviour)
                {
                    continue;
                }

                UnityEngine.Object? previousOrigin = LogRecorder.CurrentOrigin;
                LogRecorder.CurrentOrigin = behaviour;
                try
                {
                    IUnitasMessageHandler? handler = ResolveHandler(behaviour);
                    if (handler != null && handler.Dispatch(messageName))
                    {
                        delivered = true;
                    }
                    else
                    {
                        virtualFallback(behaviour);
                        delivered = true;
                    }
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex, behaviour);
                    delivered = true;
                }
                finally
                {
                    LogRecorder.CurrentOrigin = previousOrigin;
                }
            }

            return delivered;
        }

        public static IUnitasMessageHandler? ResolveHandler(MonoBehaviour? behaviour)
        {
            if (behaviour is null || !behaviour)
            {
                return null;
            }

            if (behaviour is IUnitasMessageHandler direct)
            {
                return direct;
            }

            if (Unitas.Bridge.Scripting.ScriptGlueRegistry.TryGetHandler(behaviour, out IUnitasMessageHandler? glueHandler) && glueHandler != null)
            {
                return glueHandler;
            }

            if (InstanceHandlers.TryGetValue(behaviour.GetInstanceID(), out IUnitasMessageHandler? registered))
            {
                return registered;
            }

            for (int index = 0; index < HandlerFactories.Count; index++)
            {
                IUnitasMessageHandler? created = HandlerFactories[index](behaviour);
                if (created != null)
                {
                    return created;
                }
            }

            return null;
        }

        public static bool SendMessage(GameObject target, string methodName, SendMessageOptions options = SendMessageOptions.RequireReceiver)
        {
            ArgumentNullException.ThrowIfNull(target);

            EnsureInstalled();
            int errorsBefore = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            target.SendMessage(methodName, options);
            int errorsAfter = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            return errorsAfter == errorsBefore;
        }

        public static bool SendMessage<T>(GameObject target, string methodName, T argument, SendMessageOptions options = SendMessageOptions.RequireReceiver)
        {
            ArgumentNullException.ThrowIfNull(target);

            EnsureInstalled();
            int errorsBefore = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            target.SendMessage(methodName, argument, options);
            int errorsAfter = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            return errorsAfter == errorsBefore;
        }

        public static bool BroadcastMessage(GameObject target, string methodName, SendMessageOptions options = SendMessageOptions.RequireReceiver)
        {
            ArgumentNullException.ThrowIfNull(target);

            EnsureInstalled();
            int errorsBefore = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            target.BroadcastMessage(methodName, options);
            int errorsAfter = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            return errorsAfter == errorsBefore;
        }

        public static bool BroadcastMessage<T>(GameObject target, string methodName, T argument, SendMessageOptions options = SendMessageOptions.RequireReceiver)
        {
            ArgumentNullException.ThrowIfNull(target);

            EnsureInstalled();
            int errorsBefore = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            target.BroadcastMessage(methodName, argument, options);
            int errorsAfter = LogRecorder.OfSeverity(LogSeverity.Error).Count;
            return errorsAfter == errorsBefore;
        }

        private sealed class DelegateMessageHandler<TScript> : IUnitasMessageHandler
            where TScript : MonoBehaviour
        {
            private readonly TScript _target;
            private readonly Dictionary<string, Action<TScript>> _voidMethods =
                new Dictionary<string, Action<TScript>>(StringComparer.Ordinal);
            private readonly Dictionary<string, IArgInvoker> _argMethods =
                new Dictionary<string, IArgInvoker>(StringComparer.Ordinal);

            public DelegateMessageHandler(TScript target)
            {
                _target = target;
            }

            public void AddVoid(string methodName, Action<TScript> callback)
            {
                _voidMethods[methodName] = callback;
            }

            public void AddArg<TArg>(string methodName, Action<TScript, TArg> callback)
            {
                _argMethods[methodName] = new TypedArgInvoker<TArg>(_target, callback);
            }

            public bool Dispatch(string message)
            {
                if (_voidMethods.TryGetValue(message, out Action<TScript>? action))
                {
                    action(_target);
                    return true;
                }

                return false;
            }

            public bool Dispatch<T>(string message, T argument)
            {
                if (_argMethods.TryGetValue(message, out IArgInvoker? invoker) && invoker.TryInvoke(argument))
                {
                    return true;
                }

                return Dispatch(message);
            }

            private interface IArgInvoker
            {
                bool TryInvoke<T>(T argument);
            }

            private sealed class TypedArgInvoker<TArg> : IArgInvoker
            {
                private readonly TScript _owner;
                private readonly Action<TScript, TArg> _callback;

                public TypedArgInvoker(TScript owner, Action<TScript, TArg> callback)
                {
                    _owner = owner;
                    _callback = callback;
                }

                public bool TryInvoke<T>(T argument)
                {
                    if (argument is TArg typed)
                    {
                        _callback(_owner, typed);
                        return true;
                    }

                    if (argument is null && default(TArg) is null)
                    {
                        _callback(_owner, default!);
                        return true;
                    }

                    return false;
                }
            }
        }
    }
}
