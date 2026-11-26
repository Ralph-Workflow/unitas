using System;
using System.Collections.Generic;

namespace Unitas.Bridge.Scripting
{
    // Compile-time registry of editor-callable methods. The generated glue
    // registers every [EditorCallableMethod] method it can see, so the editor
    // resolves a method by name without reflecting over the script assembly.
    public static class EditorMethodRegistry
    {
        private static readonly object Gate = new object();

        private static readonly Dictionary<string, Func<object?>> Methods =
            new Dictionary<string, Func<object?>>(StringComparer.Ordinal);

        public static void Register(string name, Func<object?> method)
        {
            ArgumentNullException.ThrowIfNull(name);
            ArgumentNullException.ThrowIfNull(method);

            lock (Gate)
            {
                Methods[name] = method;
            }
        }

        public static bool TryInvoke(string name, out object? result)
        {
            ArgumentNullException.ThrowIfNull(name);

            Func<object?>? method = null;
            lock (Gate)
            {
                Methods.TryGetValue(name, out Func<object?>? registered);
                method = registered;
            }

            if (method is null)
            {
                result = null;
                return false;
            }

            result = method();
            return true;
        }

        public static void Clear()
        {
            lock (Gate)
            {
                Methods.Clear();
            }
        }
    }
}
