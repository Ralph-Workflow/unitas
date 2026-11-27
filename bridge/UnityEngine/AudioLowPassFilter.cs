using System;

namespace UnityEngine
{
    public sealed class AudioLowPassFilter : Behaviour
    {
        private float _cutoffFrequency = 5000f;
        private float _lowpassResonanceQ = 1f;

        public float cutoffFrequency
        {
            get => _cutoffFrequency;
            set
            {
                _cutoffFrequency = Math.Clamp(value, 10f, 22000f);
                ApplyFilter();
            }
        }

        public float lowpassResonanceQ
        {
            get => _lowpassResonanceQ;
            set
            {
                _lowpassResonanceQ = Math.Clamp(value, 1f, 10f);
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
            provider.SetLowPassFilter(entityId, enabled, _cutoffFrequency, _lowpassResonanceQ);
        }
    }
}
