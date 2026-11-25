using System;
using System.Reflection;

namespace Unitas.Editor.Tests
{
    public static class Program
    {
        public static int Main()
        {
            // Test groups are discovered by name so that each unit can be built and
            // run on its own while the other units are still being written.
            Assembly assembly = typeof(Program).Assembly;
            Type[] types = assembly.GetTypes();
            Array.Sort(types, (left, right) => string.CompareOrdinal(left.FullName, right.FullName));
            foreach (Type type in types)
            {
                if (!type.IsPublic || !type.Name.EndsWith("Tests", StringComparison.Ordinal))
                {
                    continue;
                }

                MethodInfo? run = type.GetMethod(
                    "Run",
                    BindingFlags.Public | BindingFlags.Static,
                    null,
                    Type.EmptyTypes,
                    null);
                if (run == null)
                {
                    continue;
                }

                try
                {
                    run.Invoke(null, null);
                }
                catch (TargetInvocationException failure)
                {
                    Check.That(type.Name + " runs without throwing", false);
                    System.Console.WriteLine("      " + failure.InnerException);
                }
            }

            System.Console.WriteLine("failures: " + Check.FailureCount);
            return Check.FailureCount;
        }
    }
}
