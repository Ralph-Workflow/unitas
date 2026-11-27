using System;
using System.Collections.Generic;
using System.Runtime.Loader;
using UnityEngine;

namespace Unitas.Bridge.Scripting
{
    public static class ScriptRuntimeInitializeRegistry
    {
        public static int ReleaseLoadContext(AssemblyLoadContext context)
        {
            ArgumentNullException.ThrowIfNull(context);

            List<(RuntimeInitializeLoadType LoadType, Action Callback)> kept =
                new List<(RuntimeInitializeLoadType LoadType, Action Callback)>();
            int released = 0;

            IReadOnlyList<RuntimeInitializeLoadType> loadTypes = RuntimeInitializeOnLoadMethodAttribute.ExecutionOrder;
            for (int i = 0; i < loadTypes.Count; i++)
            {
                List<Action> callbacks = RuntimeInitializeOnLoadMethodAttribute.GetCallbacksForLoadType(loadTypes[i]);
                for (int c = 0; c < callbacks.Count; c++)
                {
                    Action callback = callbacks[c];
                    Type? declaringType = callback.Method.DeclaringType;
                    if (declaringType != null && AssemblyLoadContext.GetLoadContext(declaringType.Assembly) == context)
                    {
                        released++;
                        continue;
                    }

                    kept.Add((loadTypes[i], callback));
                }
            }

            RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            for (int i = 0; i < kept.Count; i++)
            {
                RuntimeInitializeOnLoadMethodAttribute.Register(kept[i].LoadType, kept[i].Callback);
            }

            return released;
        }
    }
}
