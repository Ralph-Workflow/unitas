using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public static class InvokeScheduler
    {
        private sealed class InvokeEntry
        {
            public InvokeEntry(MonoBehaviour owner, string methodName, float remainingSeconds, float repeatRate, bool isRepeating)
            {
                Owner = owner;
                MethodName = methodName;
                RemainingSeconds = remainingSeconds;
                RepeatRate = repeatRate;
                IsRepeating = isRepeating;
            }

            public MonoBehaviour Owner { get; }

            public string MethodName { get; }

            public float RemainingSeconds { get; set; }

            public float RepeatRate { get; }

            public bool IsRepeating { get; }

            public bool IsCancelled { get; set; }
        }

        private static readonly List<InvokeEntry> Entries = new List<InvokeEntry>();

        public static int ActiveCount
        {
            get
            {
                PruneDestroyedEntries();
                return Entries.Count;
            }
        }

        public static void Schedule(MonoBehaviour owner, string methodName, float time)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            if (string.IsNullOrEmpty(methodName))
            {
                throw new ArgumentException("Method name cannot be null or empty.", nameof(methodName));
            }

            if (IsOwnerDestroyed(owner))
            {
                return;
            }

            float delay = time < 0f ? 0f : time;
            Entries.Add(new InvokeEntry(owner, methodName, delay, 0f, false));
        }

        public static void ScheduleRepeating(MonoBehaviour owner, string methodName, float time, float repeatRate)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            if (string.IsNullOrEmpty(methodName))
            {
                throw new ArgumentException("Method name cannot be null or empty.", nameof(methodName));
            }

            if (repeatRate <= 0.00001f && repeatRate != 0f || repeatRate < 0f)
            {
                throw new UnityException("Invoke repeat rate cannot be negative or less than 0.00001f.");
            }

            if (IsOwnerDestroyed(owner))
            {
                return;
            }

            float delay = time < 0f ? 0f : time;
            if (repeatRate == 0f)
            {
                Entries.Add(new InvokeEntry(owner, methodName, delay, 0f, false));
                return;
            }

            Entries.Add(new InvokeEntry(owner, methodName, delay, repeatRate, true));
        }

        public static void CancelAll(MonoBehaviour owner)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            for (int index = Entries.Count - 1; index >= 0; index--)
            {
                if (ReferenceEquals(Entries[index].Owner, owner))
                {
                    Entries[index].IsCancelled = true;
                    Entries.RemoveAt(index);
                }
            }
        }

        public static void Cancel(MonoBehaviour owner, string methodName)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            if (string.IsNullOrEmpty(methodName))
            {
                CancelAll(owner);
                return;
            }

            for (int index = Entries.Count - 1; index >= 0; index--)
            {
                InvokeEntry entry = Entries[index];
                if (ReferenceEquals(entry.Owner, owner) && string.Equals(entry.MethodName, methodName, StringComparison.Ordinal))
                {
                    entry.IsCancelled = true;
                    Entries.RemoveAt(index);
                }
            }
        }

        public static void StopAll()
        {
            for (int index = 0; index < Entries.Count; index++)
            {
                Entries[index].IsCancelled = true;
            }

            Entries.Clear();
        }

        public static void OwnerDestroyed(MonoBehaviour owner)
        {
            if (owner is null)
            {
                return;
            }

            CancelAll(owner);
        }

        public static bool IsInvoking(MonoBehaviour owner)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            PruneDestroyedEntries();
            for (int index = 0; index < Entries.Count; index++)
            {
                InvokeEntry entry = Entries[index];
                if (!entry.IsCancelled && ReferenceEquals(entry.Owner, owner))
                {
                    return true;
                }
            }

            return false;
        }

        public static bool IsInvoking(MonoBehaviour owner, string methodName)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            if (string.IsNullOrEmpty(methodName))
            {
                return false;
            }

            PruneDestroyedEntries();
            for (int index = 0; index < Entries.Count; index++)
            {
                InvokeEntry entry = Entries[index];
                if (!entry.IsCancelled
                    && ReferenceEquals(entry.Owner, owner)
                    && string.Equals(entry.MethodName, methodName, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        public static void PumpFrame()
        {
            Step(Time.deltaTime);
        }

        public static void Step(float scaledDeltaTime)
        {
            PruneDestroyedEntries();
            if (Entries.Count == 0)
            {
                return;
            }

            if (Time.timeScale <= 0f || scaledDeltaTime < 0f)
            {
                return;
            }

            List<InvokeEntry> snapshot = new List<InvokeEntry>(Entries);
            for (int index = 0; index < snapshot.Count; index++)
            {
                InvokeEntry entry = snapshot[index];
                if (entry.IsCancelled)
                {
                    continue;
                }

                if (IsOwnerDestroyed(entry.Owner))
                {
                    entry.IsCancelled = true;
                    Entries.Remove(entry);
                    continue;
                }

                entry.RemainingSeconds -= scaledDeltaTime;
                if (entry.RemainingSeconds > 1e-6f)
                {
                    continue;
                }

                if (!entry.IsRepeating)
                {
                    entry.IsCancelled = true;
                    Entries.Remove(entry);
                    FireInvocation(entry);
                    continue;
                }

                int safetyIterations = 0;
                while (!entry.IsCancelled && entry.RemainingSeconds <= 1e-6f && safetyIterations < 64)
                {
                    safetyIterations++;
                    entry.RemainingSeconds += entry.RepeatRate;
                    FireInvocation(entry);
                    if (IsOwnerDestroyed(entry.Owner))
                    {
                        entry.IsCancelled = true;
                        Entries.Remove(entry);
                        break;
                    }
                }
            }

            PruneDestroyedEntries();
        }

        private static void FireInvocation(InvokeEntry entry)
        {
            MonoBehaviour owner = entry.Owner;
            if (IsOwnerDestroyed(owner))
            {
                entry.IsCancelled = true;
                Entries.Remove(entry);
                return;
            }

            IUnitasMessageHandler? handler = GameObject.ResolveMessageHandler(owner);
            Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = owner;
            try
            {
                if (handler == null || !handler.Dispatch(entry.MethodName))
                {
                    string ownerName = owner.name;
                    Debug.LogError(
                        "Trying to Invoke method: "
                        + ownerName
                        + "."
                        + entry.MethodName
                        + " couldn't be called.",
                        owner);
                }
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, owner);
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }

        private static void PruneDestroyedEntries()
        {
            for (int index = Entries.Count - 1; index >= 0; index--)
            {
                if (Entries[index].IsCancelled || IsOwnerDestroyed(Entries[index].Owner))
                {
                    Entries[index].IsCancelled = true;
                    Entries.RemoveAt(index);
                }
            }
        }

        private static bool IsOwnerDestroyed(MonoBehaviour? owner)
        {
            if (owner is null || owner.IsDestroyed || owner == null)
            {
                return true;
            }

            GameObject? go = owner.gameObject;
            return go is not null && (go.IsDestroyed || go == null);
        }
    }

    public class UnityException : Exception
    {
        public UnityException() : base("A Unity exception occurred.")
        {
        }

        public UnityException(string message) : base(message)
        {
        }

        public UnityException(string message, Exception innerException) : base(message, innerException)
        {
        }
    }
}
