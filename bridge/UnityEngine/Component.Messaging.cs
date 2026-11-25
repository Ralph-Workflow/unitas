namespace UnityEngine
{
    public partial class Component
    {
        public void SendMessage(string methodName) => SendMessage(methodName, SendMessageOptions.RequireReceiver);

        public void SendMessage(string methodName, SendMessageOptions options)
        {
            if (IsDestroyed || gameObject == null || gameObject.IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            gameObject.SendMessage(methodName, options);
        }

        public void SendMessage<T>(string methodName, T value) => SendMessage(methodName, value, SendMessageOptions.RequireReceiver);

        public void SendMessage<T>(string methodName, T value, SendMessageOptions options)
        {
            if (IsDestroyed || gameObject == null || gameObject.IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            gameObject.SendMessage(methodName, value, options);
        }

        public void BroadcastMessage(string methodName) => BroadcastMessage(methodName, SendMessageOptions.RequireReceiver);

        public void BroadcastMessage(string methodName, SendMessageOptions options)
        {
            if (IsDestroyed || gameObject == null || gameObject.IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            gameObject.BroadcastMessage(methodName, options);
        }

        public void BroadcastMessage<T>(string methodName, T parameter) => BroadcastMessage(methodName, parameter, SendMessageOptions.RequireReceiver);

        public void BroadcastMessage<T>(string methodName, T parameter, SendMessageOptions options)
        {
            if (IsDestroyed || gameObject == null || gameObject.IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            gameObject.BroadcastMessage(methodName, parameter, options);
        }

        public void SendMessageUpwards(string methodName) => SendMessageUpwards(methodName, SendMessageOptions.RequireReceiver);

        public void SendMessageUpwards(string methodName, SendMessageOptions options)
        {
            if (IsDestroyed || gameObject == null || gameObject.IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            gameObject.SendMessageUpwards(methodName, options);
        }

        public void SendMessageUpwards<T>(string methodName, T value) => SendMessageUpwards(methodName, value, SendMessageOptions.RequireReceiver);

        public void SendMessageUpwards<T>(string methodName, T value, SendMessageOptions options)
        {
            if (IsDestroyed || gameObject == null || gameObject.IsDestroyed)
            {
                throw new MissingReferenceException("The object has been destroyed and can no longer be accessed.");
            }

            gameObject.SendMessageUpwards(methodName, value, options);
        }
    }
}
