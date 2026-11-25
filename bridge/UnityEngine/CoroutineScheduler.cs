using System;
using System.Collections;
using System.Collections.Generic;

namespace UnityEngine
{
    public static class CoroutineScheduler
    {
        private enum WaitKind
        {
            NextFrame,
            Seconds,
            SecondsRealtime,
            FixedUpdate,
            EndOfFrame,
            Until,
            ChildCoroutine
        }

        private sealed class CoroutineEntry
        {
            public CoroutineEntry(MonoBehaviour owner, Coroutine handle, IEnumerator routine)
            {
                Owner = owner;
                Handle = handle;
                Enumerators = new Stack<IEnumerator>();
                Enumerators.Push(routine);
                Wait = WaitKind.NextFrame;
            }

            public MonoBehaviour Owner { get; }

            public Coroutine Handle { get; }

            public Stack<IEnumerator> Enumerators { get; }

            public WaitKind Wait { get; set; }

            public float RemainingSeconds { get; set; }

            public Func<bool>? Predicate { get; set; }

            public Coroutine? Child { get; set; }
        }

        private static readonly List<CoroutineEntry> Entries = new List<CoroutineEntry>();

        public static int ActiveCount => Entries.Count;

        public static Coroutine Start(MonoBehaviour owner, IEnumerator routine)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            if (routine == null)
            {
                throw new ArgumentNullException(nameof(routine));
            }

            Coroutine handle = new Coroutine();
            if (owner.gameObject != null && !owner.gameObject.activeInHierarchy)
            {
                string objectName = string.IsNullOrEmpty(owner.gameObject.name) ? "GameObject" : owner.gameObject.name;
                Debug.LogError(
                    "Coroutine couldn't be started because the the game object '"
                    + objectName
                    + "' is inactive!",
                    owner);
                handle.IsCompleted = true;
                return handle;
            }

            if (IsDestroyedOwner(owner))
            {
                handle.IsCompleted = true;
                return handle;
            }

            CoroutineEntry entry = new CoroutineEntry(owner, handle, routine);
            Entries.Add(entry);
            AdvanceEntry(entry);
            return handle;
        }

        public static void Stop(Coroutine coroutine)
        {
            if (coroutine == null)
            {
                throw new ArgumentNullException(nameof(coroutine));
            }

            coroutine.IsCompleted = true;
            for (int i = Entries.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(Entries[i].Handle, coroutine))
                {
                    Entries.RemoveAt(i);
                }
            }
        }

        public static void StopAll(MonoBehaviour owner)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            RemoveForOwner(owner);
        }

        public static void StopAll()
        {
            for (int i = 0; i < Entries.Count; i++)
            {
                Entries[i].Handle.IsCompleted = true;
            }

            Entries.Clear();
        }

        public static void OwnerDeactivated(MonoBehaviour owner)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            RemoveForOwner(owner);
        }

        public static void OwnerDestroyed(MonoBehaviour owner)
        {
            if (owner is null)
            {
                throw new ArgumentNullException(nameof(owner));
            }

            RemoveForOwner(owner);
        }

        public static void PumpFixed()
        {
            List<CoroutineEntry> snapshot = new List<CoroutineEntry>(Entries);
            for (int i = 0; i < snapshot.Count; i++)
            {
                CoroutineEntry entry = snapshot[i];
                if (entry.Handle.IsCompleted || !IsOwnerActive(entry.Owner))
                {
                    continue;
                }

                if (entry.Wait == WaitKind.FixedUpdate)
                {
                    AdvanceEntry(entry);
                }
            }

            ResumeCompletedChildWaiters();
        }

        public static void PumpFrame()
        {
            List<CoroutineEntry> snapshot = new List<CoroutineEntry>(Entries);
            List<CoroutineEntry> endOfFrameEntries = new List<CoroutineEntry>();

            for (int i = 0; i < snapshot.Count; i++)
            {
                CoroutineEntry entry = snapshot[i];
                if (entry.Handle.IsCompleted || !IsOwnerActive(entry.Owner))
                {
                    continue;
                }

                switch (entry.Wait)
                {
                    case WaitKind.NextFrame:
                        AdvanceEntry(entry);
                        break;

                    case WaitKind.Seconds:
                        entry.RemainingSeconds -= Time.deltaTime;
                        if (entry.RemainingSeconds <= 1e-6f)
                        {
                            AdvanceEntry(entry);
                        }
                        break;

                    case WaitKind.SecondsRealtime:
                        entry.RemainingSeconds -= Time.unscaledDeltaTime;
                        if (entry.RemainingSeconds <= 1e-6f)
                        {
                            AdvanceEntry(entry);
                        }
                        break;

                    case WaitKind.Until:
                        if (EvaluatePredicate(entry))
                        {
                            AdvanceEntry(entry);
                        }
                        break;

                    case WaitKind.ChildCoroutine:
                        if (entry.Child == null || entry.Child.IsCompleted)
                        {
                            AdvanceEntry(entry);
                        }
                        break;

                    case WaitKind.EndOfFrame:
                        endOfFrameEntries.Add(entry);
                        break;

                    case WaitKind.FixedUpdate:
                        break;
                }
            }

            ResumeCompletedChildWaiters();

            for (int i = 0; i < endOfFrameEntries.Count; i++)
            {
                CoroutineEntry entry = endOfFrameEntries[i];
                if (entry.Handle.IsCompleted || !IsOwnerActive(entry.Owner))
                {
                    continue;
                }

                if (entry.Wait == WaitKind.EndOfFrame)
                {
                    AdvanceEntry(entry);
                }
            }

            ResumeCompletedChildWaiters();
        }

        private static void ResumeCompletedChildWaiters()
        {
            bool progressed = true;
            while (progressed)
            {
                progressed = false;
                List<CoroutineEntry> snapshot = new List<CoroutineEntry>(Entries);
                for (int i = 0; i < snapshot.Count; i++)
                {
                    CoroutineEntry entry = snapshot[i];
                    if (entry.Handle.IsCompleted || !IsOwnerActive(entry.Owner))
                    {
                        continue;
                    }

                    if (entry.Wait == WaitKind.ChildCoroutine && (entry.Child == null || entry.Child.IsCompleted))
                    {
                        AdvanceEntry(entry);
                        progressed = true;
                    }
                }
            }
        }

        private static bool IsDestroyedOwner(MonoBehaviour owner)
        {
            return owner is not null && owner == null;
        }

        private static bool IsOwnerActive(MonoBehaviour owner)
        {
            return owner.gameObject == null || owner.gameObject.activeInHierarchy;
        }

        private static void RemoveForOwner(MonoBehaviour owner)
        {
            for (int i = Entries.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(Entries[i].Owner, owner))
                {
                    Entries[i].Handle.IsCompleted = true;
                    Entries.RemoveAt(i);
                }
            }
        }

        private static void CompleteEntry(CoroutineEntry entry)
        {
            entry.Handle.IsCompleted = true;
            Entries.Remove(entry);
        }

        private static bool EvaluatePredicate(CoroutineEntry entry)
        {
            Func<bool>? predicate = entry.Predicate;
            if (predicate == null)
            {
                return true;
            }

            Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = entry.Owner;
            try
            {
                return predicate();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, entry.Owner);
                CompleteEntry(entry);
                return false;
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }

        private static void AdvanceEntry(CoroutineEntry entry)
        {
            Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = entry.Owner;
            try
            {
                while (!entry.Handle.IsCompleted && entry.Enumerators.Count > 0)
                {
                    if (!IsOwnerActive(entry.Owner))
                    {
                        CompleteEntry(entry);
                        return;
                    }

                    IEnumerator currentEnumerator = entry.Enumerators.Peek();
                    bool movedNext;
                    try
                    {
                        movedNext = currentEnumerator.MoveNext();
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex, entry.Owner);
                        CompleteEntry(entry);
                        return;
                    }

                    if (entry.Handle.IsCompleted)
                    {
                        return;
                    }

                    if (!movedNext)
                    {
                        entry.Enumerators.Pop();
                        continue;
                    }

                    object? yielded = currentEnumerator.Current;
                    if (yielded == null)
                    {
                        entry.Wait = WaitKind.NextFrame;
                        entry.Child = null;
                        entry.Predicate = null;
                        return;
                    }

                    if (yielded is WaitForSeconds waitSeconds)
                    {
                        entry.Wait = WaitKind.Seconds;
                        entry.RemainingSeconds = waitSeconds.Seconds;
                        entry.Child = null;
                        entry.Predicate = null;
                        return;
                    }

                    if (yielded is WaitForSecondsRealtime waitRealtime)
                    {
                        entry.Wait = WaitKind.SecondsRealtime;
                        entry.RemainingSeconds = waitRealtime.Seconds;
                        entry.Child = null;
                        entry.Predicate = null;
                        return;
                    }

                    if (yielded is WaitForFixedUpdate)
                    {
                        entry.Wait = WaitKind.FixedUpdate;
                        entry.Child = null;
                        entry.Predicate = null;
                        return;
                    }

                    if (yielded is WaitForEndOfFrame)
                    {
                        entry.Wait = WaitKind.EndOfFrame;
                        entry.Child = null;
                        entry.Predicate = null;
                        return;
                    }

                    if (yielded is WaitUntil waitUntil)
                    {
                        entry.Wait = WaitKind.Until;
                        entry.Predicate = waitUntil.Predicate;
                        entry.Child = null;
                        return;
                    }

                    if (yielded is Coroutine childCoroutine)
                    {
                        if (childCoroutine.IsCompleted)
                        {
                            continue;
                        }

                        entry.Wait = WaitKind.ChildCoroutine;
                        entry.Child = childCoroutine;
                        entry.Predicate = null;
                        return;
                    }

                    if (yielded is IEnumerator nestedEnumerator)
                    {
                        entry.Enumerators.Push(nestedEnumerator);
                        continue;
                    }

                    entry.Wait = WaitKind.NextFrame;
                    entry.Child = null;
                    entry.Predicate = null;
                    return;
                }

                CompleteEntry(entry);
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }
    }
}
