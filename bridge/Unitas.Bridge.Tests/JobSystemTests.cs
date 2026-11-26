using System;
using System.Runtime.CompilerServices;
using System.Threading;
using Unitas.Bridge;
using UnityEngine.Collections;
using UnityEngine.Jobs;

namespace Unitas.Bridge.Tests
{
    public static class JobSystemTests
    {
        private const int Length = 1000;
        private const int BatchSize = 64;

        private static NativeArray<int>? _leakedArray;

        public static void Run()
        {
            RuntimeInterop.InitializeJobs();

            ParallelForMatchesSequentialResult();
            DependentJobWaitsForItsDependency();
            SafetyRejectsAccessWhileTheJobRuns();
            DoubleDisposeThrows();
            UndisposedArrayIsReported();
            CombinedHandleCompletesAfterBoth();
        }

        private static void ParallelForMatchesSequentialResult()
        {
            using (NativeArray<int> values = new NativeArray<int>(Length, Allocator.Persistent, "values"))
            using (NativeArray<int> visits = new NativeArray<int>(Length, Allocator.Persistent, "visits"))
            {
                for (int index = 0; index < Length; index++)
                {
                    visits[index] = 0;
                }

                FillJob job = new FillJob(values, visits);
                JobHandle handle = Job.ScheduleParallelFor(job, Length, BatchSize, new INativeArray[] { values, visits });
                handle.Complete();

                bool matches = true;
                bool once = true;
                for (int index = 0; index < Length; index++)
                {
                    if (values[index] != (index * 7) + 3)
                    {
                        matches = false;
                    }

                    if (visits[index] != 1)
                    {
                        once = false;
                    }
                }

                Check.That("a parallel-for job produces the sequential result", matches);
                Check.That("a parallel-for job writes every index exactly once", once);
            }
        }

        private static void DependentJobWaitsForItsDependency()
        {
            using (NativeArray<int> values = new NativeArray<int>(8, Allocator.Persistent, "dependency-values"))
            using (NativeArray<int> sink = new NativeArray<int>(8, Allocator.Persistent, "dependency-sink"))
            using (ManualResetEventSlim gate = new ManualResetEventSlim(false))
            {
                ObserveJob observer = new ObserveJob(values, sink);
                JobHandle dependency = Job.Schedule(new GateJob(gate, values), new INativeArray[] { values });
                JobHandle dependent = Job.Schedule(observer, new INativeArray[] { values, sink }, dependency);

                Check.That("a dependent job does not start before its dependency completes", !observer.Ran);

                gate.Set();
                dependent.Complete();

                Check.That("a dependent job observes the writes of its dependency", sink[0] == 42);
            }
        }

        private static void SafetyRejectsAccessWhileTheJobRuns()
        {
            using (NativeArray<int> values = new NativeArray<int>(16, Allocator.Persistent, "guarded-values"))
            using (ManualResetEventSlim gate = new ManualResetEventSlim(false))
            {
                JobHandle handle = Job.ScheduleParallelFor(new BlockingJob(gate, values), 16, 4, values);

                bool threw = false;
                string message = string.Empty;
                try
                {
                    _ = values[0];
                }
                catch (InvalidOperationException error)
                {
                    threw = true;
                    message = error.Message;
                }

                Check.That("reading a NativeArray used by an incomplete job throws", threw);
                Check.That(
                    "the safety failure names the array",
                    message.Contains("guarded-values", StringComparison.Ordinal));

                gate.Set();
                handle.Complete();
            }
        }

        private static void DoubleDisposeThrows()
        {
            NativeArray<int> values = new NativeArray<int>(4, Allocator.Persistent, "double-dispose-values");
            values.Dispose();

            bool threw = false;
            try
            {
                values.Dispose();
            }
            catch (InvalidOperationException)
            {
                threw = true;
            }

            Check.That("disposing a NativeArray twice throws", threw);
        }

        private static void UndisposedArrayIsReported()
        {
            int leakedBefore = NativeArrayDiagnostics.LeakedAllocationCount;
            int liveBefore = NativeArrayDiagnostics.LiveAllocationCount;

            AllocateUndisposed();

            GC.Collect();
            GC.WaitForPendingFinalizers();

            Check.Equal(
                "an undisposed NativeArray is reported at teardown",
                leakedBefore + 1,
                NativeArrayDiagnostics.LeakedAllocationCount);
            Check.Equal(
                "a finalized NativeArray releases its native memory",
                liveBefore,
                NativeArrayDiagnostics.LiveAllocationCount);
        }

        private static void CombinedHandleCompletesAfterBoth()
        {
            using (NativeArray<int> firstValues = new NativeArray<int>(4, Allocator.Persistent, "combined-first"))
            using (NativeArray<int> secondValues = new NativeArray<int>(4, Allocator.Persistent, "combined-second"))
            using (ManualResetEventSlim firstGate = new ManualResetEventSlim(false))
            using (ManualResetEventSlim secondGate = new ManualResetEventSlim(false))
            {
                JobHandle first = Job.Schedule(new GateJob(firstGate, firstValues), new INativeArray[] { firstValues });
                JobHandle second = Job.Schedule(new GateJob(secondGate, secondValues), new INativeArray[] { secondValues });
                JobHandle combined = JobHandle.CombineDependencies(first, second);

                Check.That("a combined handle is incomplete while a dependency runs", !combined.IsCompleted);

                firstGate.Set();
                first.Complete();
                Check.That("a combined handle stays incomplete while the other dependency runs", !combined.IsCompleted);

                secondGate.Set();
                second.Complete();
                Check.That("a combined handle completes after both dependencies complete", combined.IsCompleted);
            }
        }

        [MethodImpl(MethodImplOptions.NoInlining)]
        private static void AllocateUndisposed()
        {
            _leakedArray = new NativeArray<int>(8, Allocator.Persistent, "undisposed-values");
            _leakedArray = null;
        }

        private sealed class FillJob : IJobParallelFor
        {
            private readonly NativeArray<int> _values;
            private readonly NativeArray<int> _visits;

            public FillJob(NativeArray<int> values, NativeArray<int> visits)
            {
                _values = values;
                _visits = visits;
            }

            public void Execute(int index)
            {
                _values[index] = (index * 7) + 3;
                _visits[index] = _visits[index] + 1;
            }
        }

        private sealed class BlockingJob : IJobParallelFor
        {
            private readonly ManualResetEventSlim _gate;
            private readonly NativeArray<int> _values;

            public BlockingJob(ManualResetEventSlim gate, NativeArray<int> values)
            {
                _gate = gate;
                _values = values;
            }

            public void Execute(int index)
            {
                _gate.Wait();
                _values[index] = index;
            }
        }

        private sealed class GateJob : IJob
        {
            private readonly ManualResetEventSlim _gate;
            private readonly NativeArray<int> _values;

            public GateJob(ManualResetEventSlim gate, NativeArray<int> values)
            {
                _gate = gate;
                _values = values;
            }

            public void Execute()
            {
                _gate.Wait();
                _values[0] = 42;
            }
        }

        private sealed class ObserveJob : IJob
        {
            private readonly NativeArray<int> _source;
            private readonly NativeArray<int> _sink;
            private int _ran;

            public ObserveJob(NativeArray<int> source, NativeArray<int> sink)
            {
                _source = source;
                _sink = sink;
            }

            public bool Ran => Volatile.Read(ref _ran) != 0;

            public void Execute()
            {
                _sink[0] = _source[0];
                Volatile.Write(ref _ran, 1);
            }
        }
    }
}
