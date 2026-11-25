using System;
using System.Globalization;
using System.IO;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class RandomTests
    {
        public static void Run()
        {
            TestInitStatePopulatesXorshift128StateAndValueSequence();
            TestIntegerRangeSequenceExcludesMaximumAndHandlesReversedOrEqualBounds();
            TestFloatRangeSequenceIncludesEndpointsAndReproducesWithSameSeed();
            TestDifferentSeedsProduceDistinctSequences();
            TestStateSaveAndRestoreReproducesExactContinuation();
            TestDerivedGeometricAndRotationValues();
        }

        private static void TestInitStatePopulatesXorshift128StateAndValueSequence()
        {
            UnityEngine.Random.InitState(12345);
            UnityEngine.Random.State seeded = UnityEngine.Random.state;
            Check.Equal("Random.InitState(12345) s0", 12345, seeded.s0);
            Check.Equal("Random.InitState(12345) s1", 2003863422, seeded.s1);
            Check.Equal("Random.InitState(12345) s2", 878305975, seeded.s2);
            Check.Equal("Random.InitState(12345) s3", 684417332, seeded.s3);

            float[] expectedValues =
            {
                0.586850703f,
                0.899560571f,
                0.53251642f,
                0.91548574f,
                0.257733971f,
                0.425064266f,
            };

            for (int i = 0; i < expectedValues.Length; i++)
            {
                float actual = UnityEngine.Random.value;
                Check.Equal("Random.value seed 12345 step " + i, expectedValues[i], actual);
                Check.That("Random.value in [0, 1] step " + i, actual >= 0f && actual <= 1f);
            }

            UnityEngine.Random.InitState(0);
            UnityEngine.Random.State zeroState = UnityEngine.Random.state;
            Check.Equal("Random.InitState(0) s0", 0, zeroState.s0);
            Check.Equal("Random.InitState(0) s1", 1, zeroState.s1);
            Check.Equal("Random.InitState(0) s2", 1812433254, zeroState.s2);
            Check.Equal("Random.InitState(0) s3", 1900727103, zeroState.s3);
            Check.Equal("Random.value seed 0 step 0", 0.584139645f, UnityEngine.Random.value);
            Check.Equal("Random.value seed 0 step 1", 0.584082425f, UnityEngine.Random.value);
            Check.Equal("Random.value seed 0 step 2", 0.673606932f, UnityEngine.Random.value);
            Check.Equal("Random.value seed 0 step 3", 0.76650703f, UnityEngine.Random.value);
        }

        private static void TestIntegerRangeSequenceExcludesMaximumAndHandlesReversedOrEqualBounds()
        {
            UnityEngine.Random.InitState(12345);
            int[] expectedPositive = { 46, 36, 99, 20, 49, 71, 69, 50 };
            for (int i = 0; i < expectedPositive.Length; i++)
            {
                int actual = UnityEngine.Random.Range(10, 100);
                Check.Equal("Random.Range(10, 100) seed 12345 step " + i, expectedPositive[i], actual);
                Check.That("Random.Range(10, 100) within [10, 100) step " + i, actual >= 10 && actual < 100);
            }

            UnityEngine.Random.InitState(12345);
            int[] expectedSigned = { -34, 6, 49, 20, 39, -9 };
            for (int i = 0; i < expectedSigned.Length; i++)
            {
                int actual = UnityEngine.Random.Range(-50, 50);
                Check.Equal("Random.Range(-50, 50) seed 12345 step " + i, expectedSigned[i], actual);
                Check.That("Random.Range(-50, 50) within [-50, 50) step " + i, actual >= -50 && actual < 50);
            }

            UnityEngine.Random.InitState(-1);
            int[] expectedNegOne = { 304, 272, 131, 328, 646 };
            for (int i = 0; i < expectedNegOne.Length; i++)
            {
                Check.Equal("Random.Range(0, 1000) seed -1 step " + i, expectedNegOne[i], UnityEngine.Random.Range(0, 1000));
            }

            UnityEngine.Random.InitState(99);
            for (int i = 0; i < 64; i++)
            {
                int v = UnityEngine.Random.Range(5, 8);
                Check.That("Random.Range(5, 8) never returns maxExclusive 8", v >= 5 && v < 8);
            }

            Check.Equal("Random.Range(int) with equal bounds returns minInclusive", 42, UnityEngine.Random.Range(42, 42));

            UnityEngine.Random.InitState(12345);
            int reversedFirst = UnityEngine.Random.Range(100, 10);
            Check.Equal("Random.Range(100, 10) swaps reversed integer bounds", 46, reversedFirst);
        }

        private static void TestFloatRangeSequenceIncludesEndpointsAndReproducesWithSameSeed()
        {
            UnityEngine.Random.InitState(12345);
            float[] expectedFloats =
            {
                2.39447927f,
                -6.98681688f,
                4.02450705f,
                -7.46457243f,
                12.2679825f,
                7.24807167f,
            };

            float[] firstRun = new float[expectedFloats.Length];
            for (int i = 0; i < expectedFloats.Length; i++)
            {
                firstRun[i] = UnityEngine.Random.Range(-10f, 20f);
                Check.Equal("Random.Range(-10f, 20f) seed 12345 step " + i, expectedFloats[i], firstRun[i]);
                Check.That("Random.Range(-10f, 20f) within [-10, 20] step " + i, firstRun[i] >= -10f && firstRun[i] <= 20f);
            }

            UnityEngine.Random.InitState(12345);
            for (int i = 0; i < expectedFloats.Length; i++)
            {
                float secondRun = UnityEngine.Random.Range(-10f, 20f);
                Check.Equal("Re-seeding with 12345 reproduces exact float step " + i, firstRun[i], secondRun);
            }

            Check.Equal("Random.Range(float) with equal bounds returns bound", 3.5f, UnityEngine.Random.Range(3.5f, 3.5f));
        }

        private static void TestDifferentSeedsProduceDistinctSequences()
        {
            UnityEngine.Random.InitState(12345);
            int a0 = UnityEngine.Random.Range(0, 1000000);
            int a1 = UnityEngine.Random.Range(0, 1000000);
            float af = UnityEngine.Random.Range(0f, 100f);

            UnityEngine.Random.InitState(42);
            int b0 = UnityEngine.Random.Range(0, 1000000);
            int b1 = UnityEngine.Random.Range(0, 1000000);
            float bf = UnityEngine.Random.Range(0f, 100f);

            Check.That("Seed 12345 and seed 42 differ at int step 0", a0 != b0);
            Check.That("Seed 12345 and seed 42 differ at int step 1", a1 != b1);
            Check.That("Seed 12345 and seed 42 differ at float step 2", Mathf.Abs(af - bf) > 0.001f);
        }

        private static void TestStateSaveAndRestoreReproducesExactContinuation()
        {
            UnityEngine.Random.InitState(777);
            _ = UnityEngine.Random.Range(0, 100);
            _ = UnityEngine.Random.value;

            UnityEngine.Random.State saved = UnityEngine.Random.state;
            int expectedInt0 = UnityEngine.Random.Range(0, 10000);
            float expectedFloat0 = UnityEngine.Random.Range(-5f, 5f);
            int expectedInt1 = UnityEngine.Random.Range(-500, 500);
            float expectedValue = UnityEngine.Random.value;
            UnityEngine.Random.State afterExpected = UnityEngine.Random.state;

            Check.That("Saved state differs from advanced state", saved != afterExpected);

            UnityEngine.Random.InitState(99999);
            _ = UnityEngine.Random.value;

            UnityEngine.Random.state = saved;
            Check.That("Restored state equals saved state", UnityEngine.Random.state == saved);
            Check.Equal("Restored state reproduces int step 0", expectedInt0, UnityEngine.Random.Range(0, 10000));
            Check.Equal("Restored state reproduces float step 1", expectedFloat0, UnityEngine.Random.Range(-5f, 5f));
            Check.Equal("Restored state reproduces int step 2", expectedInt1, UnityEngine.Random.Range(-500, 500));
            Check.Equal("Restored state reproduces value step 3", expectedValue, UnityEngine.Random.value);
            Check.That("State after replaying matches afterExpected", UnityEngine.Random.state.Equals(afterExpected));
            Check.Equal("State.GetHashCode matches for equal states", afterExpected.GetHashCode(), UnityEngine.Random.state.GetHashCode());
        }

        private static void TestDerivedGeometricAndRotationValues()
        {
            UnityEngine.Random.InitState(2022);
            for (int i = 0; i < 16; i++)
            {
                Vector3 onSphere = UnityEngine.Random.onUnitSphere;
                Check.Equal("Random.onUnitSphere has unit magnitude step " + i, 1f, onSphere.magnitude);

                Vector3 inSphere = UnityEngine.Random.insideUnitSphere;
                Check.That("Random.insideUnitSphere has magnitude <= 1 step " + i, inSphere.magnitude <= 1.0001f);

                Vector2 inCircle = UnityEngine.Random.insideUnitCircle;
                Check.That("Random.insideUnitCircle has magnitude <= 1 step " + i, inCircle.magnitude <= 1.0001f);

                Quaternion rot = UnityEngine.Random.rotation;
                float rotNorm = Mathf.Sqrt(Quaternion.Dot(rot, rot));
                Check.Equal("Random.rotation is unit quaternion step " + i, 1f, rotNorm);

                Quaternion rotUniform = UnityEngine.Random.rotationUniform;
                float rotUniformNorm = Mathf.Sqrt(Quaternion.Dot(rotUniform, rotUniform));
                Check.Equal("Random.rotationUniform is unit quaternion step " + i, 1f, rotUniformNorm);
            }

            UnityEngine.Random.InitState(2022);
            Vector3 sphereA = UnityEngine.Random.insideUnitSphere;
            Quaternion rotA = UnityEngine.Random.rotation;

            UnityEngine.Random.InitState(2022);
            Vector3 sphereB = UnityEngine.Random.insideUnitSphere;
            Quaternion rotB = UnityEngine.Random.rotation;

            Check.That("Random.insideUnitSphere is deterministic for same seed", sphereA == sphereB);
            Check.That("Random.rotation is deterministic for same seed", rotA == rotB);
        }
    }
}
