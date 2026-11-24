namespace UnityEngine
{
    public sealed class AudioListener : Behaviour
    {
        public static float volume { get; set; } = 1.0f;

        public static bool pause { get; set; }

        public Vector3 forward { get; set; } = new Vector3(0f, 0f, 1f);

        public Vector3 Velocity => Vector3.zero;
    }
}
