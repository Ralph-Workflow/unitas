using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Messaging
{
    public static class MessageDispatcher
    {
        private static readonly Dictionary<int, IUnitasMessageHandler> InstanceHandlers =
            new Dictionary<int, IUnitasMessageHandler>();

        private static readonly List<Func<MonoBehaviour, IUnitasMessageHandler?>> HandlerFactories =
            new List<Func<MonoBehaviour, IUnitasMessageHandler?>>();

        static MessageDispatcher()
        {
            EnsureInstalled();
        }

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
            EnsureInstalled();
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
