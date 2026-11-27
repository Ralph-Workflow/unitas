namespace Cinemachine
{
    public enum CinemachineBlendStyle
    {
        Cut = 0,
        Linear = 1,
        EaseIn = 2,
        EaseOut = 3,
        EaseInOut = 4,
    }

    public readonly struct CinemachineBlendDefinition
    {
        public CinemachineBlendDefinition(CinemachineBlendStyle style, float duration)
        {
            Style = style;
            Duration = duration;
        }

        public CinemachineBlendStyle Style { get; }

        public float Duration { get; }

        public static CinemachineBlendDefinition Cut =>
            new CinemachineBlendDefinition(CinemachineBlendStyle.Cut, 0f);

        public static CinemachineBlendDefinition Linear(float duration) =>
            new CinemachineBlendDefinition(CinemachineBlendStyle.Linear, duration);

        public static CinemachineBlendDefinition EaseInOut(float duration) =>
            new CinemachineBlendDefinition(CinemachineBlendStyle.EaseInOut, duration);
    }
}
