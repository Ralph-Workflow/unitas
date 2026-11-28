using System;

namespace UnityEngine
{
    public readonly struct AnimatorStateInfo
    {
        public static readonly AnimatorStateInfo Empty = new AnimatorStateInfo(
            string.Empty,
            0,
            0f,
            0f,
            1f,
            1f,
            false,
            Vector3.zero);

        public AnimatorStateInfo(
            string name,
            int stateIndex,
            float normalizedTime,
            float length,
            float speed,
            float weight,
            bool isInTransition,
            Vector3 position)
        {
            string resolvedName = name ?? string.Empty;
            this.name = resolvedName;
            this.stateIndex = stateIndex;
            this.normalizedTime = normalizedTime;
            this.length = length;
            this.speed = speed;
            this.weight = weight;
            this.isInTransition = isInTransition;
            this.position = position;
            shortNameHash = Animator.StringToHash(resolvedName);
            fullPathHash = resolvedName.Length == 0
                ? 0
                : Animator.StringToHash("Base Layer." + resolvedName);
        }

        public string name { get; }

        public int stateIndex { get; }

        public float normalizedTime { get; }

        public float length { get; }

        public float speed { get; }

        public float weight { get; }

        public bool isInTransition { get; }

        public Vector3 position { get; }

        public int shortNameHash { get; }

        public int fullPathHash { get; }

        public bool IsName(string stateName)
        {
            if (string.IsNullOrEmpty(stateName) || string.IsNullOrEmpty(name))
            {
                return false;
            }

            if (string.Equals(name, stateName, StringComparison.Ordinal))
            {
                return true;
            }

            if (string.Equals("Base Layer." + name, stateName, StringComparison.Ordinal))
            {
                return true;
            }

            int hash = Animator.StringToHash(stateName);
            return hash == shortNameHash || hash == fullPathHash;
        }
    }
}
