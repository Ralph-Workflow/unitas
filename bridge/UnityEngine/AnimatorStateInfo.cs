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
            this.name = name ?? string.Empty;
            this.stateIndex = stateIndex;
            this.normalizedTime = normalizedTime;
            this.length = length;
            this.speed = speed;
            this.weight = weight;
            this.isInTransition = isInTransition;
            this.position = position;
        }

        public string name { get; }

        public int stateIndex { get; }

        public float normalizedTime { get; }

        public float length { get; }

        public float speed { get; }

        public float weight { get; }

        public bool isInTransition { get; }

        /// <summary>Local position of the animated transform after the latest sample.</summary>
        public Vector3 position { get; }
    }
}
