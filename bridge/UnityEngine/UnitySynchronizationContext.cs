using System;
using System.Collections.Generic;
using System.Globalization;
using System.Threading;
using System.Threading.Tasks;

namespace UnityEngine
{
    public sealed class UnitySynchronizationContext : SynchronizationContext
    {
        private readonly struct QueuedContinuation
        {
            public QueuedContinuation(SendOrPostCallback callback, object? state, Object? origin)
            {
                Callback = callback;
                State = state;
                Origin = origin;
            }

            public SendOrPostCallback Callback { get; }

            public object? State { get; }

            public Object? Origin { get; }
        }

        private sealed class TrackedTaskEntry
        {
            public TrackedTaskEntry(Task task, Object? origin, string originName)
            {
                Task = task;
                Origin = origin;
                OriginName = originName;
            }

            public Task Task { get; }

            public Object? Origin { get; }

            public string OriginName { get; }
        }

        private static readonly object QueueLock = new object();
        private static readonly List<QueuedContinuation> Pending = new List<QueuedContinuation>();
        private static readonly List<TrackedTaskEntry> TrackedTasks = new List<TrackedTaskEntry>();
        private static readonly UnitySynchronizationContext SharedInstance = new UnitySynchronizationContext();
        private static int _mainThreadId = Environment.CurrentManagedThreadId;

        public static UnitySynchronizationContext Instance => SharedInstance;

        public static int MainThreadId => _mainThreadId;

        public static bool IsMainThread => Environment.CurrentManagedThreadId == _mainThreadId;

        public static int PendingContinuationCount
        {
            get
            {
                lock (QueueLock)
                {
                    return Pending.Count;
                }
            }
        }

        public static int LiveTaskCount
        {
            get
            {
                lock (QueueLock)
                {
                    PruneCompletedTasksUnderLock();
                    return TrackedTasks.Count;
                }
            }
        }

        public static void Install()
        {
            _mainThreadId = Environment.CurrentManagedThreadId;
            SetSynchronizationContext(SharedInstance);
        }

        public static void EnsureMainThread(string apiName)
        {
            string resolvedName = string.IsNullOrEmpty(apiName) ? "UnityEngine API" : apiName;
            if (Environment.CurrentManagedThreadId != _mainThreadId)
            {
                throw new UnityException(
                    resolvedName + " can only be called from the main thread.");
            }
        }

        public static Task TrackTask(Task task, Object? origin = null, string? originLabel = null)
        {
            if (task == null)
            {
                throw new ArgumentNullException(nameof(task));
            }

            Object? effectiveOrigin = origin ?? LogRecorder.CurrentOrigin;
            string effectiveLabel = !string.IsNullOrEmpty(originLabel)
                ? originLabel!
                : ResolveOriginName(effectiveOrigin);

            lock (QueueLock)
            {
                PruneCompletedTasksUnderLock();
                if (!task.IsCompleted)
                {
                    TrackedTasks.Add(new TrackedTaskEntry(task, effectiveOrigin, effectiveLabel));
                }
            }

            return task;
        }

        public static Task<T> TrackTask<T>(Task<T> task, Object? origin = null, string? originLabel = null)
        {
            TrackTask((Task)task, origin, originLabel);
            return task;
        }

        public static void PumpFrame()
        {
            DrainPendingContinuations();
        }

        public static void PumpFixed()
        {
            DrainPendingContinuations();
        }

        public static int ReportAndClearSession()
        {
            List<TrackedTaskEntry> running = new List<TrackedTaskEntry>();
            lock (QueueLock)
            {
                for (int i = 0; i < TrackedTasks.Count; i++)
                {
                    TrackedTaskEntry entry = TrackedTasks[i];
                    if (!entry.Task.IsCompleted)
                    {
                        running.Add(entry);
                    }
                }

                TrackedTasks.Clear();
                Pending.Clear();
            }

            for (int i = 0; i < running.Count; i++)
            {
                TrackedTaskEntry entry = running[i];
                Debug.LogWarning(
                    "Task started from '"
                    + entry.OriginName
                    + "' is still running when play mode ended.",
                    entry.Origin);
            }

            return running.Count;
        }

        public static void Reset()
        {
            lock (QueueLock)
            {
                Pending.Clear();
                TrackedTasks.Clear();
            }
        }

        public override void Post(SendOrPostCallback d, object? state)
        {
            if (d == null)
            {
                throw new ArgumentNullException(nameof(d));
            }

            Object? capturedOrigin = Environment.CurrentManagedThreadId == _mainThreadId
                ? LogRecorder.CurrentOrigin
                : null;
            lock (QueueLock)
            {
                Pending.Add(new QueuedContinuation(d, state, capturedOrigin));
            }
        }

        public override void Send(SendOrPostCallback d, object? state)
        {
            if (d == null)
            {
                throw new ArgumentNullException(nameof(d));
            }

            if (Environment.CurrentManagedThreadId == _mainThreadId)
            {
                d(state);
                return;
            }

            using ManualResetEventSlim done = new ManualResetEventSlim(false);
            Exception? capturedException = null;
            Post(
                s =>
                {
                    try
                    {
                        d(s);
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        capturedException = ex;
                    }
                    finally
                    {
                        done.Set();
                    }
                },
                state);
            done.Wait();
            if (capturedException != null)
            {
                throw new InvalidOperationException("SynchronizationContext.Send callback faulted.", capturedException);
            }
        }

        public override SynchronizationContext CreateCopy()
        {
            return SharedInstance;
        }

        private static void DrainPendingContinuations()
        {
            if (Application.isPaused)
            {
                return;
            }

            List<QueuedContinuation> batch;
            lock (QueueLock)
            {
                if (Pending.Count == 0)
                {
                    PruneCompletedTasksUnderLock();
                    return;
                }

                batch = new List<QueuedContinuation>(Pending);
                Pending.Clear();
                PruneCompletedTasksUnderLock();
            }

            SynchronizationContext? previousContext = Current;
            SetSynchronizationContext(SharedInstance);
            try
            {
                for (int i = 0; i < batch.Count; i++)
                {
                    QueuedContinuation item = batch[i];
                    Object? previousOrigin = LogRecorder.CurrentOrigin;
                    if (item.Origin != null)
                    {
                        LogRecorder.CurrentOrigin = item.Origin;
                    }

                    try
                    {
                        item.Callback(item.State);
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex, item.Origin);
                    }
                    finally
                    {
                        LogRecorder.CurrentOrigin = previousOrigin;
                    }
                }
            }
            finally
            {
                SetSynchronizationContext(previousContext ?? SharedInstance);
            }
        }

        private static void PruneCompletedTasksUnderLock()
        {
            for (int i = TrackedTasks.Count - 1; i >= 0; i--)
            {
                if (TrackedTasks[i].Task.IsCompleted)
                {
                    TrackedTasks.RemoveAt(i);
                }
            }
        }

        private static string ResolveOriginName(Object? origin)
        {
            if (origin is null)
            {
                return "MainThread";
            }

            try
            {
                string name = origin.name;
                if (!string.IsNullOrEmpty(name))
                {
                    return name;
                }
            }
            catch (MissingReferenceException)
            {
            }

            return "Object#" + origin.GetInstanceID().ToString(CultureInfo.InvariantCulture);
        }
    }
}
