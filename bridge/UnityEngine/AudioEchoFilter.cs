using System;

namespace UnityEngine
{
    public sealed class AudioEchoFilter : Behaviour
    {
        private float _echoDelay = 0.5f;
        private float _echoDryLevel;
        private float _echoFeedback = -10f;
        private float _echoWetLevel;

        public float echoDelay
        {
            get => _echoDelay;
            set
            {
                _echoDelay = Math.Clamp(value, 0.02f, 1f);
                ApplyFilter();
            }
        }

        public float echoDryLevel
        {
            get => _echoDryLevel;
            set
            {
                _echoDryLevel = Math.Clamp(value, -60f, 0f);
                ApplyFilter();
            }
        }

        public float echoFeedback
        {
            get => _echoFeedback;
            set
            {
                _echoFeedback = Math.Clamp(value, -60f, 20f);
                ApplyFilter();
            }
        }

        public float echoWetLevel
        {
            get => _echoWetLevel;
            set
            {
                _echoWetLevel = Math.Clamp(value, -60f, 0f);
                ApplyFilter();
            }
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            ApplyFilter();
        }

        internal void ApplyFilter()
        {
            IAudioProvider? provider = AudioSource.Provider;
            if (provider == null)
            {
                return;
            }
            ulong entityId = AudioFilterBinding.ResolveEntityId(this);
            if (entityId == 0UL)
            {
                return;
            }
            provider.SetEchoFilter(
                entityId,
                enabled,
                _echoDelay,
                _echoDryLevel,
                _echoFeedback,
                _echoWetLevel);
        }
    }
}
