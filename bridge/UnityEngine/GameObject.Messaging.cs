using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public enum SendMessageOptions
    {
        RequireReceiver = 0,
        DontRequireReceiver = 1
    }

    public partial class GameObject
    {
        public static Func<MonoBehaviour, IUnitasMessageHandler?>? MessageHandlerResolver { get; set; }

        internal static IUnitasMessageHandler? ResolveMessageHandler(MonoBehaviour behaviour)
        {
            if (behaviour is null || behaviour.IsDestroyed)
            {
                return null;
            }

            if (behaviour is IUnitasMessageHandler directHandler)
            {
                return directHandler;
            }

            Func<MonoBehaviour, IUnitasMessageHandler?>? resolver = MessageHandlerResolver;
            return resolver != null ? resolver(behaviour) : null;
        }

        public void SendMessage(string methodName) => SendMessage(methodName, SendMessageOptions.RequireReceiver);

        public void SendMessage(string methodName, SendMessageOptions options)
        {
            ValidateMessageCall(methodName);
            bool handled = DispatchMessageOnObject(this, methodName);
            if (!handled && options == SendMessageOptions.RequireReceiver)
            {
                Debug.LogError("SendMessage " + methodName + " has no receiver on " + name + "!", this);
            }
        }

        public void SendMessage<T>(string methodName, T value) => SendMessage(methodName, value, SendMessageOptions.RequireReceiver);

        public void SendMessage<T>(string methodName, T value, SendMessageOptions options)
        {
            ValidateMessageCall(methodName);
            bool handled = DispatchMessageOnObject(this, methodName, value);
            if (!handled && options == SendMessageOptions.RequireReceiver)
            {
                Debug.LogError("SendMessage " + methodName + " has no receiver on " + name + "!", this);
            }
        }

        public void BroadcastMessage(string methodName) => BroadcastMessage(methodName, SendMessageOptions.RequireReceiver);

        public void BroadcastMessage(string methodName, SendMessageOptions options)
        {
            ValidateMessageCall(methodName);
            bool handled = DispatchBroadcastMessage(this, methodName);
            if (!handled && options == SendMessageOptions.RequireReceiver)
            {
                Debug.LogError("BroadcastMessage " + methodName + " has no receiver on " + name + "!", this);
            }
        }

        public void BroadcastMessage<T>(string methodName, T parameter) => BroadcastMessage(methodName, parameter, SendMessageOptions.RequireReceiver);

        public void BroadcastMessage<T>(string methodName, T parameter, SendMessageOptions options)
        {
            ValidateMessageCall(methodName);
            bool handled = DispatchBroadcastMessage(this, methodName, parameter);
            if (!handled && options == SendMessageOptions.RequireReceiver)
            {
                Debug.LogError("BroadcastMessage " + methodName + " has no receiver on " + name + "!", this);
            }
        }

        public void SendMessageUpwards(string methodName) => SendMessageUpwards(methodName, SendMessageOptions.RequireReceiver);

        public void SendMessageUpwards(string methodName, SendMessageOptions options)
        {
            ValidateMessageCall(methodName);
            bool handled = false;
            GameObject? current = this;
            while (current is not null && !current.IsDestroyed)
            {
                if (DispatchMessageOnObject(current, methodName))
                {
                    handled = true;
                }

                Transform? parent = current.transform.parent;
                current = parent is not null ? parent.gameObject : null;
            }

            if (!handled && options == SendMessageOptions.RequireReceiver)
            {
                Debug.LogError("SendMessageUpwards " + methodName + " has no receiver on " + name + "!", this);
            }
        }

        public void SendMessageUpwards<T>(string methodName, T value) => SendMessageUpwards(methodName, value, SendMessageOptions.RequireReceiver);

        public void SendMessageUpwards<T>(string methodName, T value, SendMessageOptions options)
        {
            ValidateMessageCall(methodName);
            bool handled = false;
            GameObject? current = this;
            while (current is not null && !current.IsDestroyed)
            {
                if (DispatchMessageOnObject(current, methodName, value))
                {
                    handled = true;
                }

                Transform? parent = current.transform.parent;
                current = parent is not null ? parent.gameObject : null;
            }

            if (!handled && options == SendMessageOptions.RequireReceiver)
            {
                Debug.LogError("SendMessageUpwards " + methodName + " has no receiver on " + name + "!", this);
            }
        }

        private void ValidateMessageCall(string methodName)
        {
            if (IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            if (string.IsNullOrEmpty(methodName))
            {
                throw new ArgumentException("Method name cannot be null or empty.", nameof(methodName));
            }
        }

        private static bool DispatchBroadcastMessage(GameObject node, string methodName)
        {
            if (node is null || node.IsDestroyed || !node.activeInHierarchy)
            {
                return false;
            }

            bool handled = DispatchMessageOnObject(node, methodName);
            int childCount = node.transform.childCount;
            List<GameObject> children = new List<GameObject>(childCount);
            for (int i = 0; i < childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child is not null)
                {
                    children.Add(child);
                }
            }

            for (int i = 0; i < children.Count; i++)
            {
                if (DispatchBroadcastMessage(children[i], methodName))
                {
                    handled = true;
                }
            }

            return handled;
        }

        private static bool DispatchBroadcastMessage<T>(GameObject node, string methodName, T parameter)
        {
            if (node is null || node.IsDestroyed || !node.activeInHierarchy)
            {
                return false;
            }

            bool handled = DispatchMessageOnObject(node, methodName, parameter);
            int childCount = node.transform.childCount;
            List<GameObject> children = new List<GameObject>(childCount);
            for (int i = 0; i < childCount; i++)
            {
                GameObject child = node.transform.GetChild(i).gameObject;
                if (child is not null)
                {
                    children.Add(child);
                }
            }

            for (int i = 0; i < children.Count; i++)
            {
                if (DispatchBroadcastMessage(children[i], methodName, parameter))
                {
                    handled = true;
                }
            }

            return handled;
        }

        private static bool DispatchMessageOnObject(GameObject target, string methodName)
        {
            if (target is null || target.IsDestroyed || !target.activeInHierarchy)
            {
                return false;
            }

            bool handled = false;
            List<Component> snapshot = new List<Component>(target._components);
            for (int index = 0; index < snapshot.Count; index++)
            {
                if (!(snapshot[index] is MonoBehaviour behaviour) || behaviour.IsDestroyed)
                {
                    continue;
                }

                IUnitasMessageHandler? handler = ResolveMessageHandler(behaviour);
                if (handler == null)
                {
                    continue;
                }

                Object? previousOrigin = LogRecorder.CurrentOrigin;
                LogRecorder.CurrentOrigin = behaviour;
                try
                {
                    if (handler.Dispatch(methodName))
                    {
                        handled = true;
                    }
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex, behaviour);
                    handled = true;
                }
                finally
                {
                    LogRecorder.CurrentOrigin = previousOrigin;
                }
            }

            return handled;
        }

        private static bool DispatchMessageOnObject<T>(GameObject target, string methodName, T parameter)
        {
            if (target is null || target.IsDestroyed || !target.activeInHierarchy)
            {
                return false;
            }

            bool handled = false;
            List<Component> snapshot = new List<Component>(target._components);
            for (int index = 0; index < snapshot.Count; index++)
            {
                if (!(snapshot[index] is MonoBehaviour behaviour) || behaviour.IsDestroyed)
                {
                    continue;
                }

                IUnitasMessageHandler? handler = ResolveMessageHandler(behaviour);
                if (handler == null)
                {
                    continue;
                }

                Object? previousOrigin = LogRecorder.CurrentOrigin;
                LogRecorder.CurrentOrigin = behaviour;
                try
                {
                    if (handler.Dispatch(methodName, parameter) || handler.Dispatch(methodName))
                    {
                        handled = true;
                    }
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex, behaviour);
                    handled = true;
                }
                finally
                {
                    LogRecorder.CurrentOrigin = previousOrigin;
                }
            }

            return handled;
        }
    }
}
