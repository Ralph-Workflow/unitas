using System;
using System.Collections.Generic;
using UnityEngine;
using Unitas.Bridge;

namespace Unitas.Bridge.Tests
{
    public sealed class BatchRecordingScript : MonoBehaviour
    {
        public string Label = string.Empty;

        public List<string> Log = new List<string>();

        public string ThrowInPhase = string.Empty;

        public override void FixedUpdate()
        {
            Record("FixedUpdate");
        }

        public override void Update()
        {
            Record("Update");
        }

        public override void LateUpdate()
        {
            Record("LateUpdate");
        }

        private void Record(string phase)
        {
            Log.Add(Label + ":" + phase);
            if (ThrowInPhase == phase)
            {
                throw new InvalidOperationException(Label + " " + phase + " failure");
            }
        }
    }

    public static class ScriptBatchDispatchTests
    {
        public static void Run()
        {
            OrderPreservedAcrossKinds();
            FaultIsolatedAndReportedWithScriptOrigin();
            EmptyBatchIsNoOp();
        }

        private static void OrderPreservedAcrossKinds()
        {
            LogRecorder.Clear();
            List<string> log = new List<string>();
            using (HostScope scope = new HostScope(log))
            {
                Dispatch(scope, kind: 0, delta: 0.02);
                Check.Equal("batch fixed-update runs in handle order", "a:FixedUpdate,b:FixedUpdate,c:FixedUpdate", JoinedLog(log));
                Dispatch(scope, kind: 2, delta: 0.016);
                Check.Equal("batch late-update runs in handle order", "a:FixedUpdate,b:FixedUpdate,c:FixedUpdate,a:LateUpdate,b:LateUpdate,c:LateUpdate", JoinedLog(log));
            }

            Check.Equal("no errors recorded for a clean batch", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);
        }

        private static void FaultIsolatedAndReportedWithScriptOrigin()
        {
            LogRecorder.Clear();
            List<string> log = new List<string>();
            BatchRecordingScript middle;
            using (HostScope scope = new HostScope(log, middleThrows: true))
            {
                middle = scope.Middle;
                Dispatch(scope, kind: 1, delta: 0.016);
                Check.That("faulting script ran before throwing", JoinedLog(log).Contains("b:Update", StringComparison.Ordinal));
                Check.That("script after the fault still ran", JoinedLog(log).Contains("c:Update", StringComparison.Ordinal));
                Check.That("later script ran after the faulting one", JoinedLog(log).IndexOf("c:Update", StringComparison.Ordinal) > JoinedLog(log).IndexOf("b:Update", StringComparison.Ordinal));
            }

            IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
            Check.Equal("one error recorded for the throwing script", 1, errors.Count);
            Check.That("error entry carries the throwing script as origin", errors.Count == 1 && ReferenceEquals(errors[0].Origin, middle));
            Check.That("error entry names the failure", errors.Count == 1 && errors[0].Message.Contains("b Update failure", StringComparison.Ordinal));
        }

        private static void EmptyBatchIsNoOp()
        {
            LogRecorder.Clear();
            List<string> log = new List<string>();
            using (HostScope scope = new HostScope(log))
            {
                Dispatch(scope, kind: 1, delta: 0.016, count: 0);
            }

            Check.Equal("empty batch invokes no scripts", string.Empty, JoinedLog(log));
            Check.Equal("empty batch records no errors", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);
        }

        // Invoke through a function pointer: UnmanagedCallersOnly entries cannot be called
        // directly from managed code, and this is exactly how the native runtime will call it.
        private static unsafe void Dispatch(HostScope scope, uint kind, double delta, int count = 3)
        {
            ulong* handles = stackalloc ulong[3];
            double* deltas = stackalloc double[3];
            for (int i = 0; i < count; i++)
            {
                handles[i] = scope.Handles[i];
                deltas[i] = delta;
            }

            delegate* unmanaged[Cdecl]<uint, ulong*, double*, ulong, void> dispatch = &BridgeHost.ScriptBatchDispatch;
            dispatch(kind, handles, deltas, (ulong)count);
        }

        private static string JoinedLog(List<string> log)
        {
            return string.Join(",", log);
        }

        private sealed class HostScope : IDisposable
        {
            public HostScope(List<string> log, bool middleThrows = false)
            {
#pragma warning disable CA2000
                // Disposing would run Shutdown against a zero native handle; the managed-only
                // host is abandoned once the scope ends and holds no native resources.
                BridgeHost host = BridgeHost.CreateManagedOnlyForBatchDispatch();
#pragma warning restore CA2000
                First = Attach("a", log);
                Middle = Attach("b", log);
                Middle.ThrowInPhase = middleThrows ? "Update" : string.Empty;
                Last = Attach("c", log);
                Handles = new ulong[3];
                Handles[0] = host.RegisterScriptForManagedDispatch(First);
                Handles[1] = host.RegisterScriptForManagedDispatch(Middle);
                Handles[2] = host.RegisterScriptForManagedDispatch(Last);
            }

            public BatchRecordingScript First { get; }

            public BatchRecordingScript Middle { get; }

            public BatchRecordingScript Last { get; }

            public ulong[] Handles { get; }

            public void Dispose()
            {
                UnityEngine.Object.Destroy(First.gameObject);
                UnityEngine.Object.Destroy(Middle.gameObject);
                UnityEngine.Object.Destroy(Last.gameObject);
            }

            private static BatchRecordingScript Attach(string label, List<string> log)
            {
                GameObject owner = new GameObject("batch-" + label);
                BatchRecordingScript script = owner.AddComponent<BatchRecordingScript>();
                script.Label = label;
                script.Log = log;
                return script;
            }
        }
    }
}
