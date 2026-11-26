using System;
using System.Collections.Generic;

namespace UnityEngine.Collections
{
    internal static class NativeArraySafety
    {
        private static readonly object Gate = new object();

        private static readonly Dictionary<INativeArray, List<INativeArrayDependency>> Pending =
            new Dictionary<INativeArray, List<INativeArrayDependency>>();

        [ThreadStatic]
        private static INativeArrayDependency? _current;

        internal static void Register(INativeArray array, INativeArrayDependency dependency)
        {
            lock (Gate)
            {
                if (!Pending.TryGetValue(array, out List<INativeArrayDependency>? list))
                {
                    list = new List<INativeArrayDependency>();
                    Pending.Add(array, list);
                }

                list.Add(dependency);
            }
        }

        internal static void Forget(INativeArray array)
        {
            lock (Gate)
            {
                Pending.Remove(array);
            }
        }

        internal static void CompletePending(INativeArray array)
        {
            INativeArrayDependency[] snapshot = Snapshot(array);
            for (int index = 0; index < snapshot.Length; index++)
            {
                snapshot[index].Complete();
            }
        }

        internal static void ThrowIfInUse(INativeArray array)
        {
            INativeArrayDependency? current = _current;
            INativeArrayDependency[] snapshot = Snapshot(array);
            for (int index = 0; index < snapshot.Length; index++)
            {
                INativeArrayDependency dependency = snapshot[index];
                if (ReferenceEquals(dependency, current))
                {
                    continue;
                }

                if (current != null && IsDescendant(dependency, current))
                {
                    continue;
                }

                if (dependency.IsCompleted)
                {
                    Forget(array, dependency);
                    continue;
                }

                throw new InvalidOperationException(
                    "NativeArray '" + array.Name + "' is used by a job that has not completed.");
            }
        }

        internal static void EnterJob(INativeArrayDependency dependency)
        {
            _current = dependency;
        }

        internal static void ExitJob()
        {
            _current = null;
        }

        private static INativeArrayDependency[] Snapshot(INativeArray array)
        {
            lock (Gate)
            {
                if (!Pending.TryGetValue(array, out List<INativeArrayDependency>? list) || list.Count == 0)
                {
                    return Array.Empty<INativeArrayDependency>();
                }

                return list.ToArray();
            }
        }

        private static void Forget(INativeArray array, INativeArrayDependency dependency)
        {
            lock (Gate)
            {
                if (!Pending.TryGetValue(array, out List<INativeArrayDependency>? list))
                {
                    return;
                }

                list.Remove(dependency);
                if (list.Count == 0)
                {
                    Pending.Remove(array);
                }
            }
        }

        private static bool IsDescendant(INativeArrayDependency dependency, INativeArrayDependency ancestor)
        {
            INativeArrayDependency? current = dependency.Parent;
            while (current != null)
            {
                if (ReferenceEquals(current, ancestor))
                {
                    return true;
                }

                current = current.Parent;
            }

            return false;
        }
    }
}
