using System;
using UnityEngine;

namespace UnityEngine.Video
{
    public enum VideoRenderMode
    {
        CameraFarPlane = 0,
        CameraNearPlane = 1,
        RenderTexture = 2,
        MaterialOverride = 3,
        APIOnly = 4,
    }

    public enum VideoSource
    {
        VideoClip = 0,
        Url = 1,
    }

    public readonly struct VideoPlayerNativeState
    {
        public VideoPlayerNativeState(
            uint width,
            uint height,
            ulong frameCount,
            double frameRate,
            double lengthSeconds,
            long currentFrame,
            double currentTime,
            float playbackSpeed,
            bool isPrepared,
            bool isPlaying,
            bool isPaused,
            bool isLooping,
            bool sendFrameReadyEvents,
            uint eventBits,
            string? errorMessage)
        {
            Width = width;
            Height = height;
            FrameCount = frameCount;
            FrameRate = frameRate;
            LengthSeconds = lengthSeconds;
            CurrentFrame = currentFrame;
            CurrentTime = currentTime;
            PlaybackSpeed = playbackSpeed;
            IsPrepared = isPrepared;
            IsPlaying = isPlaying;
            IsPaused = isPaused;
            IsLooping = isLooping;
            SendFrameReadyEvents = sendFrameReadyEvents;
            EventBits = eventBits;
            ErrorMessage = errorMessage;
        }

        public uint Width { get; }

        public uint Height { get; }

        public ulong FrameCount { get; }

        public double FrameRate { get; }

        public double LengthSeconds { get; }

        public long CurrentFrame { get; }

        public double CurrentTime { get; }

        public float PlaybackSpeed { get; }

        public bool IsPrepared { get; }

        public bool IsPlaying { get; }

        public bool IsPaused { get; }

        public bool IsLooping { get; }

        public bool SendFrameReadyEvents { get; }

        public uint EventBits { get; }

        public string? ErrorMessage { get; }
    }

    public interface IVideoPlayerProvider
    {
        ulong CreatePlayer(ulong entityId);

        void DestroyPlayer(ulong playerHandle);

        bool LoadClip(ulong playerHandle, string path);

        bool Prepare(ulong playerHandle, bool waitForFirstFrame);

        bool Play(ulong playerHandle, bool waitForFirstFrame);

        void Pause(ulong playerHandle);

        void Stop(ulong playerHandle);

        void StepForward(ulong playerHandle);

        void SetTime(ulong playerHandle, double seconds);

        void SetFrame(ulong playerHandle, long frameIndex);

        void SetLooping(ulong playerHandle, bool isLooping);

        void SetPlaybackSpeed(ulong playerHandle, float speed);

        void SetSendFrameReadyEvents(ulong playerHandle, bool enabled);

        void SetTargetEntity(ulong playerHandle, ulong targetEntityId);

        void Advance(ulong playerHandle, double deltaSeconds);

        bool TryGetState(ulong playerHandle, out VideoPlayerNativeState state);

        byte[] GetFrameRgba(ulong playerHandle);
    }

    public sealed class VideoPlayer : Behaviour
    {
        public delegate void EventHandler(VideoPlayer source);

        public delegate void FrameReadyEventHandler(VideoPlayer source, long frameIdx);

        public delegate void ErrorEventHandler(VideoPlayer source, string message);

        private const uint EventBitPrepareCompleted = 1u << 0;
        private const uint EventBitStarted = 1u << 1;
        private const uint EventBitFrameReady = 1u << 2;
        private const uint EventBitLoopPointReached = 1u << 3;
        private const uint EventBitSeekCompleted = 1u << 4;
        private const uint EventBitErrorReceived = 1u << 5;

        private static IVideoPlayerProvider? s_provider;
        private static Func<IVideoPlayerProvider?>? s_defaultProviderFactory;

        private VideoSource _source = VideoSource.VideoClip;
        private VideoClip? _clip;
        private string _url = string.Empty;
        private VideoRenderMode _renderMode = VideoRenderMode.CameraFarPlane;
        private Camera? _targetCamera;
        private Component? _targetMaterialRenderer;
        private string _targetMaterialProperty = "_MainTex";
        private bool _isLooping;
        private bool _playOnAwake = true;
        private bool _waitForFirstFrame = true;
        private float _playbackSpeed = 1f;
        private bool _skipOnDrop = true;
        private bool _sendFrameReadyEvents;
        private ulong _nativePlayerHandle;
        private ulong _nativeEntityId;
        private string _loadedPath = string.Empty;
        private VideoPlayerNativeState _cachedState;

        public static IVideoPlayerProvider? Provider
        {
            get => s_provider ?? s_defaultProviderFactory?.Invoke();
            set => s_provider = value;
        }

        public static void RegisterDefaultProviderFactory(Func<IVideoPlayerProvider?>? factory)
        {
            s_defaultProviderFactory = factory;
        }

        public event EventHandler? prepareCompleted;

        public event EventHandler? started;

        public event FrameReadyEventHandler? frameReady;

        public event EventHandler? loopPointReached;

        public event EventHandler? seekCompleted;

        public event ErrorEventHandler? errorReceived;

        public VideoSource source
        {
            get => _source;
            set => _source = value;
        }

        public VideoClip? clip
        {
            get => _clip;
            set
            {
                _clip = value;
                if (value != null)
                {
                    _source = VideoSource.VideoClip;
                    _loadedPath = string.Empty;
                }
            }
        }

        public string url
        {
            get => _url;
            set
            {
                _url = value ?? string.Empty;
                if (!string.IsNullOrEmpty(_url))
                {
                    _source = VideoSource.Url;
                    _loadedPath = string.Empty;
                }
            }
        }

        public VideoRenderMode renderMode
        {
            get => _renderMode;
            set
            {
                _renderMode = value;
                SyncTargetEntity();
            }
        }

        public Camera? targetCamera
        {
            get => _targetCamera;
            set
            {
                _targetCamera = value;
                SyncTargetEntity();
            }
        }

        public Component? targetMaterialRenderer
        {
            get => _targetMaterialRenderer;
            set
            {
                _targetMaterialRenderer = value;
                SyncTargetEntity();
            }
        }

        public string targetMaterialProperty
        {
            get => _targetMaterialProperty;
            set => _targetMaterialProperty = value ?? "_MainTex";
        }

        public bool isLooping
        {
            get => _isLooping;
            set
            {
                _isLooping = value;
                ulong handle = EnsureNativePlayer();
                if (handle != 0UL)
                {
                    Provider?.SetLooping(handle, value);
                }
            }
        }

        public bool playOnAwake
        {
            get => _playOnAwake;
            set => _playOnAwake = value;
        }

        public bool waitForFirstFrame
        {
            get => _waitForFirstFrame;
            set => _waitForFirstFrame = value;
        }

        public float playbackSpeed
        {
            get => _playbackSpeed;
            set
            {
                _playbackSpeed = value < 0f ? 0f : value;
                ulong handle = EnsureNativePlayer();
                if (handle != 0UL)
                {
                    Provider?.SetPlaybackSpeed(handle, _playbackSpeed);
                }
            }
        }

        public bool skipOnDrop
        {
            get => _skipOnDrop;
            set => _skipOnDrop = value;
        }

        public bool sendFrameReadyEvents
        {
            get => _sendFrameReadyEvents;
            set
            {
                _sendFrameReadyEvents = value;
                ulong handle = EnsureNativePlayer();
                if (handle != 0UL)
                {
                    Provider?.SetSendFrameReadyEvents(handle, value);
                }
            }
        }

        public bool canSetTime => true;

        public bool canSetPlaybackSpeed => true;

        public bool canStep => true;

        public bool canSetSkipOnDrop => true;

        public bool isPrepared
        {
            get
            {
                PollNativeState();
                return _cachedState.IsPrepared;
            }
        }

        public bool isPlaying
        {
            get
            {
                PollNativeState();
                return _cachedState.IsPlaying;
            }
        }

        public bool isPaused
        {
            get
            {
                PollNativeState();
                return _cachedState.IsPaused;
            }
        }

        public double time
        {
            get
            {
                PollNativeState();
                return _cachedState.CurrentTime;
            }
            set
            {
                ulong handle = EnsureNativePlayer();
                if (handle != 0UL)
                {
                    Provider?.SetTime(handle, value);
                    PollNativeState();
                }
            }
        }

        public long frame
        {
            get
            {
                PollNativeState();
                return _cachedState.CurrentFrame;
            }
            set
            {
                ulong handle = EnsureNativePlayer();
                if (handle != 0UL)
                {
                    Provider?.SetFrame(handle, value);
                    PollNativeState();
                }
            }
        }

        public ulong frameCount
        {
            get
            {
                PollNativeState();
                if (_cachedState.FrameCount > 0)
                {
                    return _cachedState.FrameCount;
                }

                return _clip?.frameCount ?? 0UL;
            }
        }

        public float frameRate
        {
            get
            {
                PollNativeState();
                if (_cachedState.FrameRate > 0.0)
                {
                    return (float)_cachedState.FrameRate;
                }

                return (float)(_clip?.frameRate ?? 0.0);
            }
        }

        public double length
        {
            get
            {
                PollNativeState();
                if (_cachedState.LengthSeconds > 0.0)
                {
                    return _cachedState.LengthSeconds;
                }

                return _clip?.length ?? 0.0;
            }
        }

        public uint width
        {
            get
            {
                PollNativeState();
                if (_cachedState.Width > 0)
                {
                    return _cachedState.Width;
                }

                return _clip?.width ?? 0u;
            }
        }

        public uint height
        {
            get
            {
                PollNativeState();
                if (_cachedState.Height > 0)
                {
                    return _cachedState.Height;
                }

                return _clip?.height ?? 0u;
            }
        }

        public ulong NativePlayerHandle => EnsureNativePlayer();

        public void Prepare()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong handle = EnsureNativePlayer();
            if (handle == 0UL || !EnsureClipLoaded(provider, handle))
            {
                PollNativeState();
                return;
            }

            provider.Prepare(handle, _waitForFirstFrame);
            PollNativeState();
        }

        public void Play()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong handle = EnsureNativePlayer();
            if (handle == 0UL || !EnsureClipLoaded(provider, handle))
            {
                PollNativeState();
                return;
            }

            provider.Play(handle, _waitForFirstFrame);
            PollNativeState();
        }

        public void Pause()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            provider.Pause(_nativePlayerHandle);
            PollNativeState();
        }

        public void Stop()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            provider.Stop(_nativePlayerHandle);
            PollNativeState();
        }

        public void StepForward()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            provider.StepForward(_nativePlayerHandle);
            PollNativeState();
        }

        public void Advance(double deltaSeconds)
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            provider.Advance(_nativePlayerHandle, deltaSeconds);
            PollNativeState();
        }

        public byte[] GetCurrentFrameRgba()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return Array.Empty<byte>();
            }

            return provider.GetFrameRgba(_nativePlayerHandle);
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            if (!isEnabled && _nativePlayerHandle != 0UL)
            {
                Provider?.Pause(_nativePlayerHandle);
                PollNativeState();
            }
        }

        private string EffectiveSourcePath()
        {
            if (_source == VideoSource.VideoClip && _clip != null)
            {
                return _clip.originalPath;
            }

            if (!string.IsNullOrEmpty(_url))
            {
                return _url;
            }

            return _clip?.originalPath ?? string.Empty;
        }

        private bool EnsureClipLoaded(IVideoPlayerProvider provider, ulong handle)
        {
            string sourcePath = EffectiveSourcePath();
            if (string.IsNullOrEmpty(sourcePath))
            {
                errorReceived?.Invoke(this, "VideoPlayer has no VideoClip or URL assigned.");
                return false;
            }

            if (_loadedPath == sourcePath)
            {
                return true;
            }

            bool loaded = provider.LoadClip(handle, sourcePath);
            if (loaded)
            {
                _loadedPath = sourcePath;
                SyncClipMetadata();
            }

            return loaded;
        }

        private void SyncClipMetadata()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            if (provider.TryGetState(_nativePlayerHandle, out VideoPlayerNativeState state))
            {
                _cachedState = state;
                if (_clip != null && state.Width > 0 && state.Height > 0)
                {
                    _clip.width = state.Width;
                    _clip.height = state.Height;
                    _clip.frameCount = state.FrameCount;
                    _clip.frameRate = state.FrameRate;
                    _clip.length = state.LengthSeconds;
                }
            }
        }

        private void SyncTargetEntity()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            ulong targetId = ResolveTargetEntityId();
            provider.SetTargetEntity(_nativePlayerHandle, targetId);
        }

        private ulong ResolveTargetEntityId()
        {
            if (_renderMode == VideoRenderMode.MaterialOverride && _targetMaterialRenderer != null)
            {
                GameObject? matOwner = _targetMaterialRenderer.OwnerOrNull;
                if (matOwner != null && !matOwner.IsDestroyed && matOwner.NativeEntityId != 0UL)
                {
                    return matOwner.NativeEntityId;
                }
            }

            GameObject? owner = OwnerOrNull;
            return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
        }

        private ulong EnsureNativePlayer()
        {
            GameObject? owner = OwnerOrNull;
            ulong entityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            if (_nativePlayerHandle != 0UL && _nativeEntityId == entityId)
            {
                return _nativePlayerHandle;
            }

            IVideoPlayerProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            if (_nativePlayerHandle != 0UL)
            {
                provider.DestroyPlayer(_nativePlayerHandle);
            }

            _nativeEntityId = entityId;
            _nativePlayerHandle = provider.CreatePlayer(entityId);
            _loadedPath = string.Empty;
            if (_nativePlayerHandle != 0UL)
            {
                provider.SetLooping(_nativePlayerHandle, _isLooping);
                provider.SetPlaybackSpeed(_nativePlayerHandle, _playbackSpeed);
                provider.SetSendFrameReadyEvents(_nativePlayerHandle, _sendFrameReadyEvents);
                SyncTargetEntity();
            }

            return _nativePlayerHandle;
        }

        private void PollNativeState()
        {
            IVideoPlayerProvider? provider = Provider;
            if (provider == null || _nativePlayerHandle == 0UL)
            {
                return;
            }

            if (!provider.TryGetState(_nativePlayerHandle, out VideoPlayerNativeState state))
            {
                return;
            }

            _cachedState = state;
            uint bits = state.EventBits;
            if ((bits & EventBitPrepareCompleted) != 0)
            {
                prepareCompleted?.Invoke(this);
            }

            if ((bits & EventBitStarted) != 0)
            {
                started?.Invoke(this);
            }

            if ((bits & EventBitFrameReady) != 0 && _sendFrameReadyEvents)
            {
                frameReady?.Invoke(this, state.CurrentFrame);
            }

            if ((bits & EventBitLoopPointReached) != 0)
            {
                loopPointReached?.Invoke(this);
            }

            if ((bits & EventBitSeekCompleted) != 0)
            {
                seekCompleted?.Invoke(this);
            }

            if ((bits & EventBitErrorReceived) != 0)
            {
                errorReceived?.Invoke(this, state.ErrorMessage ?? "VideoPlayer playback error");
            }
        }
    }
}
