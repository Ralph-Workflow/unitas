using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public enum RuntimeInitializeLoadType
    {
        AfterSceneLoad = 0,
        BeforeSceneLoad = 1,
        AfterAssembliesLoaded = 2,
        BeforeSplashScreen = 3,
        SubsystemRegistration = 4,
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class RuntimeInitializeOnLoadMethodAttribute : Attribute
    {
        private static readonly object s_gate = new object();
        private static readonly List<(RuntimeInitializeLoadType LoadType, Action Callback)> s_registeredMethods =
            new List<(RuntimeInitializeLoadType LoadType, Action Callback)>();

        private static readonly RuntimeInitializeLoadType[] s_executionOrder =
        {
            RuntimeInitializeLoadType.SubsystemRegistration,
            RuntimeInitializeLoadType.AfterAssembliesLoaded,
            RuntimeInitializeLoadType.BeforeSplashScreen,
            RuntimeInitializeLoadType.BeforeSceneLoad,
            RuntimeInitializeLoadType.AfterSceneLoad,
        };

        public RuntimeInitializeOnLoadMethodAttribute()
            : this(RuntimeInitializeLoadType.AfterSceneLoad)
        {
        }

        public RuntimeInitializeOnLoadMethodAttribute(RuntimeInitializeLoadType loadType)
        {
            this.loadType = loadType;
        }

        public RuntimeInitializeLoadType loadType { get; }

        public static IReadOnlyList<RuntimeInitializeLoadType> ExecutionOrder => s_executionOrder;

        public static void Register(Action callback)
        {
            Register(RuntimeInitializeLoadType.AfterSceneLoad, callback);
        }

        public static void Register(RuntimeInitializeLoadType loadType, Action callback)
        {
            if (callback == null)
            {
                throw new ArgumentNullException(nameof(callback));
            }

            lock (s_gate)
            {
                for (int i = 0; i < s_registeredMethods.Count; i++)
                {
                    if (s_registeredMethods[i].LoadType == loadType
                        && s_registeredMethods[i].Callback == callback)
                    {
                        return;
                    }
                }

                s_registeredMethods.Add((loadType, callback));
            }
        }

        public static void ClearRegisteredMethods()
        {
            lock (s_gate)
            {
                s_registeredMethods.Clear();
            }
        }

        internal static bool HasRegisteredCallbacks(RuntimeInitializeLoadType loadType)
        {
            lock (s_gate)
            {
                for (int i = 0; i < s_registeredMethods.Count; i++)
                {
                    if (s_registeredMethods[i].LoadType == loadType)
                    {
                        return true;
                    }
                }
            }

            return false;
        }

        internal static List<Action> GetCallbacksForLoadType(RuntimeInitializeLoadType loadType)
        {
            List<Action> matching = new List<Action>();
            lock (s_gate)
            {
                for (int i = 0; i < s_registeredMethods.Count; i++)
                {
                    if (s_registeredMethods[i].LoadType == loadType)
                    {
                        matching.Add(s_registeredMethods[i].Callback);
                    }
                }
            }

            return matching;
        }
    }
}
