using System;

namespace UnityEngine
{
    public enum AudioSourceState
    {
        Scheduled = 0,
        Loading = 1,
        Playing = 2,
        Paused = 3,
        Virtualized = 4,
        Stopped = 5,
        Completed = 6,
        Failed = 7,
    }

    public enum AudioSilenceReason
    {
        None = -1,
        ZeroVolume = 0,
        Muted = 1,
        Paused = 2,
        Virtualized = 3,
        OutOfRange = 4,
        NoListener = 5,
        UnsupportedClip = 6,
        Stopped = 7,
        Completed = 8,
    }

    public readonly struct AudioSourceConfig
    {
        public AudioSourceConfig(
            float volume,
            float pitch,
            float spatialBlend,
            float minDistance,
            float maxDistance,
            ulong clipId,
            double clipDuration,
            int priority,
            bool looping,
            bool playOnAwake,
            bool mute,
            bool enabled)
        {
            Volume = volume;
            Pitch = pitch;
            SpatialBlend = spatialBlend;
            MinDistance = minDistance;
            MaxDistance = maxDistance;
            ClipId = clipId;
            ClipDuration = clipDuration;
            Priority = priority;
            Looping = looping;
            PlayOnAwake = playOnAwake;
            Mute = mute;
            Enabled = enabled;
        }

        public float Volume { get; }

        public float Pitch { get; }

        public float SpatialBlend { get; }

        public float MinDistance { get; }

        public float MaxDistance { get; }

        public ulong ClipId { get; }

        public double ClipDuration { get; }

        public int Priority { get; }

        public bool Looping { get; }

        public bool PlayOnAwake { get; }

        public bool Mute { get; }

        public bool Enabled { get; }
    }

    public interface IAudioProvider
    {
        ulong CreateSource(ulong entityId);

        void DestroySource(ulong entityId);

        void Play(ulong entityId, bool oneShot);

        void PlayScheduled(ulong entityId, double dspTime);

        void PlayDelayed(ulong entityId, float delaySeconds);

        double GetDspTime();

        void Pause(ulong entityId);

        void UnPause(ulong entityId);

        void Stop(ulong entityId);

        void SetVolume(ulong entityId, float volume);

        void SetPitch(ulong entityId, float pitch);

        void SetMute(ulong entityId, bool mute);

        void SetLoop(ulong entityId, bool loop);

        void SetPriority(ulong entityId, int priority);

        void SetSpatialBlend(ulong entityId, float spatialBlend);

        void SetMinMaxDistance(ulong entityId, float minDistance, float maxDistance);

        void SetTime(ulong entityId, double time);

        void SetClip(ulong entityId, ulong clipId, double fallbackDuration);

        void SetPosition(ulong entityId, Vector3 position);

        AudioSourceState GetState(ulong entityId);

        double GetTime(ulong entityId);

        AudioSilenceReason GetSilenceReason(ulong entityId);

        AudioClip CreateClip(string name, int lengthSamples, int channels, int frequency);

        AudioClip? LoadClip(string path);
    }

    public sealed class AudioSource : Behaviour
    {
        private AudioClip? _clip;
        private float _volume = 1.0f;
        private float _pitch = 1.0f;
        private bool _mute;
        private bool _loop;
        private bool _playOnAwake = true;
        private int _priority = 128;
        private float _spatialBlend;
        private float _minDistance = 1.0f;
        private float _maxDistance = 500.0f;
        private float _time;
        private ulong _nativeEntityId;
        private AudioSourceState _localState = AudioSourceState.Stopped;

        public static IAudioProvider? Provider { get; set; }

        public static double dspTime
        {
            get
            {
                IAudioProvider? provider = Provider;
                return provider != null ? provider.GetDspTime() : 0.0;
            }
        }

        public AudioClip? clip
        {
            get => _clip;
            set
            {
                _clip = value;
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetClip(entityId, value?.NativeClipId ?? 0UL, value?.length ?? 0.0);
                    }
                }
            }
        }

        public float volume
        {
            get => _volume;
            set
            {
                _volume = Math.Clamp(value, 0f, 1f);
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetVolume(entityId, _volume);
                    }
                }
            }
        }

        public float pitch
        {
            get => _pitch;
            set
            {
                _pitch = value;
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetPitch(entityId, _pitch);
                    }
                }
            }
        }

        public bool mute
        {
            get => _mute;
            set
            {
                _mute = value;
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetMute(entityId, _mute);
                    }
                }
            }
        }

        public bool loop
        {
            get => _loop;
            set
            {
                _loop = value;
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetLoop(entityId, _loop);
                    }
                }
            }
        }

        public bool playOnAwake
        {
            get => _playOnAwake;
            set => _playOnAwake = value;
        }

        public int priority
        {
            get => _priority;
            set
            {
                _priority = Math.Clamp(value, 0, 255);
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetPriority(entityId, _priority);
                    }
                }
            }
        }

        public float spatialBlend
        {
            get => _spatialBlend;
            set
            {
                _spatialBlend = Math.Clamp(value, 0f, 1f);
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetSpatialBlend(entityId, _spatialBlend);
                    }
                }
            }
        }

        public float minDistance
        {
            get => _minDistance;
            set
            {
                _minDistance = Math.Max(0f, value);
                if (_maxDistance < _minDistance)
                {
                    _maxDistance = _minDistance;
                }
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetMinMaxDistance(entityId, _minDistance, _maxDistance);
                    }
                }
            }
        }

        public float maxDistance
        {
            get => _maxDistance;
            set
            {
                _maxDistance = Math.Max(_minDistance, Math.Max(0f, value));
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetMinMaxDistance(entityId, _minDistance, _maxDistance);
                    }
                }
            }
        }

        public float time
        {
            get => Provider != null && TryGetEntityId(out ulong id) ? (float)Provider.GetTime(id) : _time;
            set
            {
                _time = Math.Max(0f, value);
                IAudioProvider? provider = Provider;
                if (provider != null)
                {
                    ulong entityId = EnsureNativeEntityId();
                    if (entityId != 0UL)
                    {
                        provider.SetTime(entityId, _time);
                    }
                }
            }
        }

        public bool isPlaying
        {
            get
            {
                AudioSourceState currentState = state;
                return currentState == AudioSourceState.Playing || currentState == AudioSourceState.Virtualized;
            }
        }

        public AudioSourceState state => Provider != null && TryGetEntityId(out ulong id) ? Provider.GetState(id) : _localState;

        public AudioSilenceReason silenceReason => Provider != null && TryGetEntityId(out ulong id)
            ? Provider.GetSilenceReason(id)
            : (_mute ? AudioSilenceReason.Muted : (_volume <= 0f ? AudioSilenceReason.ZeroVolume : AudioSilenceReason.Stopped));

        public void Play()
        {
            if (!isActiveAndEnabled)
            {
                return;
            }

            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    if (gameObject != null && !gameObject.IsDestroyed)
                    {
                        provider.SetPosition(entityId, transform.position);
                    }
                    if (_clip != null)
                    {
                        provider.SetClip(entityId, _clip.NativeClipId, _clip.length);
                    }
                    provider.Play(entityId, false);
                }
            }

            _localState = AudioSourceState.Playing;
        }

        public void PlayOneShot(AudioClip clip, float volumeScale = 1.0f)
        {
            if (clip == null || !isActiveAndEnabled)
            {
                return;
            }

            float scaledVolume = Math.Clamp(_volume * Math.Max(0f, volumeScale), 0f, 1f);
            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    if (gameObject != null && !gameObject.IsDestroyed)
                    {
                        provider.SetPosition(entityId, transform.position);
                    }
                    provider.SetClip(entityId, clip.NativeClipId, clip.length);
                    if (Math.Abs(volumeScale - 1.0f) > 1e-6f)
                    {
                        provider.SetVolume(entityId, scaledVolume);
                    }
                    provider.Play(entityId, true);
                }
            }

            if (_clip == null)
            {
                _clip = clip;
            }

            _localState = AudioSourceState.Playing;
        }

        public void PlayScheduled(double dspTime)
        {
            if (!isActiveAndEnabled)
            {
                return;
            }

            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    if (gameObject != null && !gameObject.IsDestroyed)
                    {
                        provider.SetPosition(entityId, transform.position);
                    }
                    if (_clip != null)
                    {
                        provider.SetClip(entityId, _clip.NativeClipId, _clip.length);
                    }
                    provider.PlayScheduled(entityId, dspTime);
                }
            }

            _localState = AudioSourceState.Scheduled;
        }

        public void PlayDelayed(float seconds)
        {
            if (!isActiveAndEnabled)
            {
                return;
            }

            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    if (gameObject != null && !gameObject.IsDestroyed)
                    {
                        provider.SetPosition(entityId, transform.position);
                    }
                    if (_clip != null)
                    {
                        provider.SetClip(entityId, _clip.NativeClipId, _clip.length);
                    }
                    provider.PlayDelayed(entityId, seconds);
                }
            }

            _localState = AudioSourceState.Scheduled;
        }

        public void Pause()
        {
            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    provider.Pause(entityId);
                }
            }

            _localState = AudioSourceState.Paused;
        }

        public void UnPause()
        {
            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    provider.UnPause(entityId);
                }
            }

            _localState = AudioSourceState.Playing;
        }

        public void Stop()
        {
            IAudioProvider? provider = Provider;
            if (provider != null)
            {
                ulong entityId = EnsureNativeEntityId();
                if (entityId != 0UL)
                {
                    provider.Stop(entityId);
                }
            }

            _localState = AudioSourceState.Stopped;
            _time = 0f;
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            if (!isEnabled)
            {
                Stop();
            }
        }

        internal void OnComponentDestroyed()
        {
            IAudioProvider? provider = Provider;
            if (provider != null && TryGetEntityId(out ulong entityId) && entityId != 0UL)
            {
                provider.Stop(entityId);
                provider.DestroySource(entityId);
            }

            _localState = AudioSourceState.Stopped;
            _time = 0f;
        }

        internal void ApplyNativeConfig(ulong entityId, AudioSourceConfig config, AudioClip? nativeClip)
        {
            _nativeEntityId = entityId;
            _volume = Math.Clamp(config.Volume, 0f, 1f);
            _pitch = config.Pitch;
            _spatialBlend = Math.Clamp(config.SpatialBlend, 0f, 1f);
            _minDistance = Math.Max(0f, config.MinDistance);
            _maxDistance = Math.Max(_minDistance, Math.Max(0f, config.MaxDistance));
            _priority = Math.Clamp(config.Priority, 0, 255);
            _loop = config.Looping;
            _playOnAwake = config.PlayOnAwake;
            _mute = config.Mute;
            enabled = config.Enabled;
            _clip = nativeClip;
        }

        internal bool TryGetEntityId(out ulong entityId)
        {
            if (_nativeEntityId != 0UL)
            {
                entityId = _nativeEntityId;
                return true;
            }

            GameObject? owner = gameObject;
            if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0UL)
            {
                entityId = owner.NativeEntityId;
                return true;
            }

            entityId = 0UL;
            return false;
        }

        internal ulong EnsureNativeEntityId()
        {
            if (_nativeEntityId != 0UL)
            {
                return _nativeEntityId;
            }

            GameObject? owner = gameObject;
            ulong ownerEntityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            IAudioProvider? provider = Provider;
            if (provider == null)
            {
                return ownerEntityId;
            }

            _nativeEntityId = provider.CreateSource(ownerEntityId);
            if (owner != null && !owner.IsDestroyed && owner.NativeEntityId == 0UL && _nativeEntityId != 0UL)
            {
                owner.AssignNativeEntityId(_nativeEntityId);
            }

            if (_nativeEntityId != 0UL)
            {
                provider.SetVolume(_nativeEntityId, _volume);
                provider.SetPitch(_nativeEntityId, _pitch);
                provider.SetMute(_nativeEntityId, _mute);
                provider.SetLoop(_nativeEntityId, _loop);
                provider.SetPriority(_nativeEntityId, _priority);
                provider.SetSpatialBlend(_nativeEntityId, _spatialBlend);
                provider.SetMinMaxDistance(_nativeEntityId, _minDistance, _maxDistance);
                if (_clip != null)
                {
                    provider.SetClip(_nativeEntityId, _clip.NativeClipId, _clip.length);
                }
                if (owner != null && !owner.IsDestroyed)
                {
                    provider.SetPosition(_nativeEntityId, owner.transform.position);
                }
            }

            return _nativeEntityId;
        }
    }
}
