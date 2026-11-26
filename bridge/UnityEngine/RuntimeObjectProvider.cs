using System;

namespace UnityEngine
{
    public enum RuntimeColliderShape
    {
        Sphere = 0,
        Box = 1,
        Capsule = 2,
    }

    public interface IRuntimeObjectProvider
    {
        ulong CreateRuntimeObject(string name);

        Component? CreateRuntimeScript(ulong entity, Type scriptType);

        bool SetRuntimeTransform(ulong entity, Vector3 position, Quaternion rotation, Vector3 scale);

        bool CreateRuntimeBody(
            ulong entity,
            RuntimeColliderShape shape,
            float radius,
            Vector3 halfExtents,
            float height,
            bool isTrigger);

        bool DestroyRuntimeObject(ulong entity);
    }

    public static class RuntimeObjectProvider
    {
        public static IRuntimeObjectProvider? Current { get; set; }

        public static Action<Type, Func<MonoBehaviour>>? ScriptFactoryRegistrar { get; set; }

        public static Action<ulong, GameObject>? RuntimeObjectRegistrationHandler { get; set; }

        internal static void RegisterScriptFactory(Type scriptType, Func<MonoBehaviour> factory)
        {
            ScriptFactoryRegistrar?.Invoke(scriptType, factory);
        }

        internal static void NotifyRuntimeObjectCreated(ulong entity, GameObject gameObject)
        {
            RuntimeObjectRegistrationHandler?.Invoke(entity, gameObject);
        }
    }
}
