using System;

namespace UnityEngine
{
    public static class Random
    {
        private const uint MtMultiplier = 1812433253U;
        private const uint FloatMantissaMask = 0x007FFFFFU;
        private const float FloatNormalizer = 1.0f / 8388607.0f;

        private static State _state = CreateStateFromSeed(123456789);

        [Serializable]
        public struct State : IEquatable<State>
        {
            public int s0;
            public int s1;
            public int s2;
            public int s3;

            public State(int s0, int s1, int s2, int s3)
            {
                this.s0 = s0;
                this.s1 = s1;
                this.s2 = s2;
                this.s3 = s3;
            }

            public static bool operator ==(State lhs, State rhs) => lhs.Equals(rhs);

            public static bool operator !=(State lhs, State rhs) => !lhs.Equals(rhs);

            public bool Equals(State other) =>
                s0 == other.s0 && s1 == other.s1 && s2 == other.s2 && s3 == other.s3;

            public override bool Equals(object? obj) => obj is State other && Equals(other);

            public override int GetHashCode() => HashCode.Combine(s0, s1, s2, s3);
        }

        public static State state
        {
            get => _state;
            set => _state = value;
        }

        public static void InitState(int seed)
        {
            _state = CreateStateFromSeed(seed);
        }

        public static float value => NextFloat01();

        public static int Range(int minInclusive, int maxExclusive)
        {
            if (minInclusive == maxExclusive)
            {
                return minInclusive;
            }

            if (minInclusive > maxExclusive)
            {
                int swapped = minInclusive;
                minInclusive = maxExclusive;
                maxExclusive = swapped;
            }

            uint span = (uint)(maxExclusive - minInclusive);
            uint sample = NextUInt() % span;
            return (int)((uint)minInclusive + sample);
        }

        public static float Range(float minInclusive, float maxInclusive)
        {
            float t = NextFloat01();
            return (t * minInclusive) + ((1.0f - t) * maxInclusive);
        }

        public static Vector3 onUnitSphere
        {
            get
            {
                float z = Range(-1.0f, 1.0f);
                float theta = Range(0.0f, 2.0f * Mathf.PI);
                float radius = Mathf.Sqrt(Mathf.Max(0.0f, 1.0f - (z * z)));
                float x = radius * Mathf.Cos(theta);
                float y = radius * Mathf.Sin(theta);
                return new Vector3(x, y, z);
            }
        }

        public static Vector3 insideUnitSphere
        {
            get
            {
                Vector3 dir = onUnitSphere;
                float r = Mathf.Pow(value, 1.0f / 3.0f);
                return dir * r;
            }
        }

        public static Vector2 insideUnitCircle
        {
            get
            {
                float angle = Range(0.0f, 2.0f * Mathf.PI);
                float r = Mathf.Sqrt(value);
                return new Vector2(r * Mathf.Cos(angle), r * Mathf.Sin(angle));
            }
        }

        public static Quaternion rotation => rotationUniform;

        public static Quaternion rotationUniform
        {
            get
            {
                float u1 = value;
                float u2 = Range(0.0f, 2.0f * Mathf.PI);
                float u3 = Range(0.0f, 2.0f * Mathf.PI);
                float sqrt1MinusU1 = Mathf.Sqrt(Mathf.Max(0.0f, 1.0f - u1));
                float sqrtU1 = Mathf.Sqrt(Mathf.Max(0.0f, u1));
                return new Quaternion(
                    sqrt1MinusU1 * Mathf.Sin(u2),
                    sqrt1MinusU1 * Mathf.Cos(u2),
                    sqrtU1 * Mathf.Sin(u3),
                    sqrtU1 * Mathf.Cos(u3));
            }
        }

        private static State CreateStateFromSeed(int seed)
        {
            uint u0 = (uint)seed;
            uint u1 = (MtMultiplier * u0) + 1U;
            uint u2 = (MtMultiplier * u1) + 1U;
            uint u3 = (MtMultiplier * u2) + 1U;
            if (u0 == 0U && u1 == 0U && u2 == 0U && u3 == 0U)
            {
                u0 = 1U;
            }

            return new State((int)u0, (int)u1, (int)u2, (int)u3);
        }

        private static uint NextUInt()
        {
            uint x = (uint)_state.s0;
            uint y = (uint)_state.s1;
            uint z = (uint)_state.s2;
            uint w = (uint)_state.s3;
            if (x == 0U && y == 0U && z == 0U && w == 0U)
            {
                x = 1U;
            }

            uint t = x ^ (x << 11);
            _state.s0 = (int)y;
            _state.s1 = (int)z;
            _state.s2 = (int)w;
            uint nextW = w ^ (w >> 19) ^ t ^ (t >> 8);
            _state.s3 = (int)nextW;
            return nextW;
        }

        private static float NextFloat01()
        {
            uint sample = NextUInt();
            return (sample & FloatMantissaMask) * FloatNormalizer;
        }
    }
}
