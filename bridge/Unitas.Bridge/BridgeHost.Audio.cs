using System;
using System.IO;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IDisposable, IPhysicsProvider, IPhysics2DProvider, ICharacterControllerProvider, IInputProvider, ISceneHost, IApplicationHost, IAudioProvider, IRuntimeObjectProvider
    {
        public ulong CreateSource(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0)
            {
                return 0UL;
            }
            return RuntimeInterop.unitas_runtime_audio_create(_runtimeHandle, entityId);
        }

        public void DestroySource(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_destroy(_runtimeHandle, entityId);
        }

        public void Play(ulong entityId, bool oneShot)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_play(_runtimeHandle, entityId, oneShot ? 1U : 0U);
        }

        public void Pause(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_pause(_runtimeHandle, entityId);
        }

        public void UnPause(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_unpause(_runtimeHandle, entityId);
        }

        public void Stop(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_stop(_runtimeHandle, entityId);
        }

        public void SetVolume(ulong entityId, float volume)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_volume(_runtimeHandle, entityId, volume);
        }

        public void SetPitch(ulong entityId, float pitch)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_pitch(_runtimeHandle, entityId, pitch);
        }

        public void SetMute(ulong entityId, bool mute)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_mute(_runtimeHandle, entityId, mute ? 1U : 0U);
        }

        public void SetLoop(ulong entityId, bool loop)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_loop(_runtimeHandle, entityId, loop ? 1U : 0U);
        }

        public void SetPriority(ulong entityId, int priority)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            uint clamped = (uint)Math.Clamp(priority, 0, 255);
            RuntimeInterop.unitas_runtime_audio_set_priority(_runtimeHandle, entityId, clamped);
        }

        public void SetSpatialBlend(ulong entityId, float spatialBlend)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_spatial_blend(_runtimeHandle, entityId, spatialBlend);
        }

        public void SetMinMaxDistance(ulong entityId, float minDistance, float maxDistance)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_min_max_distance(_runtimeHandle, entityId, minDistance, maxDistance);
        }

        public void SetTime(ulong entityId, double time)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_time(_runtimeHandle, entityId, time);
        }

        public void SetClip(ulong entityId, ulong clipId, double fallbackDuration)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_clip(_runtimeHandle, entityId, clipId);
            if (clipId == 0UL && fallbackDuration > 0.0)
            {
                RuntimeInterop.unitas_runtime_audio_set_custom_duration(_runtimeHandle, entityId, fallbackDuration);
            }
        }

        public void SetPosition(ulong entityId, Vector3 position)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return;
            }
            RuntimeInterop.unitas_runtime_audio_set_position(_runtimeHandle, entityId, position.x, position.y, position.z);
        }

        public bool SetAudioGroup(ulong entityId, uint group)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_audio_set_group(_runtimeHandle, entityId, group) == 0;
        }

        public int GetAudioGroup(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return -1;
            }
            return RuntimeInterop.unitas_runtime_audio_group(_runtimeHandle, entityId);
        }

        public AudioSourceState GetState(ulong entityId)
        {
            return GetAudioState(entityId);
        }

        public double GetTime(ulong entityId)
        {
            return GetAudioTime(entityId);
        }

        public AudioSilenceReason GetSilenceReason(ulong entityId)
        {
            return GetAudioSilenceReason(entityId);
        }

        public AudioClip CreateClip(string name, int lengthSamples, int channels, int frequency)
        {
            int clampedSamples = Math.Max(0, lengthSamples);
            int clampedChannels = Math.Max(1, channels);
            int clampedFrequency = Math.Max(1, frequency);
            float duration = (float)clampedSamples / clampedFrequency;
            ulong clipId = 0UL;
            if (!_isDisposed && _runtimeHandle != 0 && clampedSamples > 0)
            {
                ulong totalSamples = (ulong)clampedSamples * (ulong)clampedChannels;
                ulong byteLength = totalSamples * 4UL;
                IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, byteLength);
                if (buffer != IntPtr.Zero)
                {
                    byte[] sampleBytes = new byte[(int)byteLength];
                    for (int frame = 0; frame < clampedSamples; frame++)
                    {
                        float sample = (float)Math.Sin(2.0 * Math.PI * 440.0 * frame / clampedFrequency) * 0.5f;
                        byte[] floatBytes = BitConverter.GetBytes(sample);
                        for (int ch = 0; ch < clampedChannels; ch++)
                        {
                            int byteOffset = ((frame * clampedChannels) + ch) * 4;
                            Buffer.BlockCopy(floatBytes, 0, sampleBytes, byteOffset, 4);
                        }
                    }
                    Marshal.Copy(sampleBytes, 0, buffer, sampleBytes.Length);
                    clipId = RuntimeInterop.unitas_runtime_audio_register_clip_samples(
                        _runtimeHandle,
                        (uint)clampedChannels,
                        (uint)clampedFrequency,
                        (ulong)clampedSamples,
                        (ulong)buffer.ToInt64());
                }
            }

            return new AudioClip(name ?? string.Empty, duration, clampedChannels, clampedFrequency, clipId);
        }

        public AudioClip? LoadClip(string path)
        {
            if (string.IsNullOrEmpty(path) || _isDisposed || _runtimeHandle == 0)
            {
                return null;
            }
            ulong clipId = LoadAudioClip(path);
            if (clipId == 0UL)
            {
                return null;
            }
            string clipName = Path.GetFileNameWithoutExtension(path);
            if (TryGetAudioClipInfo(clipId, out uint channels, out uint sampleRate, out ulong frameCount, out double durationSeconds))
            {
                return new AudioClip(clipName, (float)durationSeconds, (int)channels, (int)sampleRate, clipId);
            }
            return new AudioClip(clipName, 0f, 1, 44100, clipId);
        }

        public ulong LoadAudioClip(string path)
        {
            ThrowIfDisposed();
            if (!TryWriteStringToPathBuffer(path, out IntPtr buffer, out ulong length))
            {
                return 0UL;
            }
            return RuntimeInterop.unitas_runtime_audio_load_clip(_runtimeHandle, (ulong)buffer.ToInt64(), length);
        }

        public AudioSourceState GetAudioState(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return AudioSourceState.Stopped;
            }
            int raw = RuntimeInterop.unitas_runtime_audio_state(_runtimeHandle, entityId);
            if (raw < 0 || raw > (int)AudioSourceState.Failed)
            {
                return AudioSourceState.Stopped;
            }
            return (AudioSourceState)raw;
        }

        public AudioSilenceReason GetAudioSilenceReason(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return AudioSilenceReason.Stopped;
            }
            int raw = RuntimeInterop.unitas_runtime_audio_silence_reason(_runtimeHandle, entityId);
            if (raw < -1 || raw > (int)AudioSilenceReason.Completed)
            {
                return AudioSilenceReason.None;
            }
            return (AudioSilenceReason)raw;
        }

        public double GetAudioTime(ulong entityId)
        {
            if (_isDisposed || _runtimeHandle == 0 || entityId == 0)
            {
                return 0.0;
            }
            double time = RuntimeInterop.unitas_runtime_audio_time(_runtimeHandle, entityId);
            return time >= 0.0 ? time : 0.0;
        }

        public float[] MixAudio(int frames)
        {
            ThrowIfDisposed();
            if (frames <= 0)
            {
                return Array.Empty<float>();
            }
            ulong byteCount = (ulong)frames * 2UL * 4UL;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, byteCount);
            if (buffer == IntPtr.Zero)
            {
                return Array.Empty<float>();
            }
            ulong mixedFrames = RuntimeInterop.unitas_runtime_audio_mix(_runtimeHandle, (ulong)frames, (ulong)buffer.ToInt64());
            int totalSamples = (int)mixedFrames * 2;
            if (totalSamples <= 0)
            {
                return Array.Empty<float>();
            }
            float[] output = new float[totalSamples];
            Marshal.Copy(buffer, output, 0, totalSamples);
            return output;
        }

        public bool TryGetAudioClipInfo(ulong clipId, out uint channels, out uint sampleRate, out ulong frameCount, out double durationSeconds)
        {
            channels = 0;
            sampleRate = 0;
            frameCount = 0;
            durationSeconds = 0.0;
            if (_isDisposed || _runtimeHandle == 0 || clipId == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 16);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_audio_clip_info(_runtimeHandle, clipId, (ulong)buffer.ToInt64()) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[16];
            Marshal.Copy(buffer, bytes, 0, 16);
            durationSeconds = BitConverter.ToDouble(bytes, 0);
            sampleRate = BitConverter.ToUInt32(bytes, 8);
            channels = BitConverter.ToUInt32(bytes, 12);
            frameCount = sampleRate > 0 && durationSeconds > 0.0
                ? (ulong)Math.Round(durationSeconds * sampleRate)
                : 0UL;
            return true;
        }

        public bool TryGetAudioSourceConfig(ulong entity, out AudioSourceConfig config)
        {
            config = default;
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 44);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_audio_source_config(_runtimeHandle, entity, (ulong)buffer.ToInt64()) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[44];
            Marshal.Copy(buffer, bytes, 0, 44);
            float volume = BitConverter.ToSingle(bytes, 0);
            float pitch = BitConverter.ToSingle(bytes, 4);
            float spatialBlend = BitConverter.ToSingle(bytes, 8);
            float minDistance = BitConverter.ToSingle(bytes, 12);
            float maxDistance = BitConverter.ToSingle(bytes, 16);
            ulong clipId = BitConverter.ToUInt64(bytes, 20);
            double clipDuration = BitConverter.ToDouble(bytes, 28);
            int priority = bytes[36];
            bool looping = bytes[37] != 0;
            bool playOnAwake = bytes[38] != 0;
            bool mute = bytes[39] != 0;
            bool enabled = bytes[40] != 0;
            config = new AudioSourceConfig(
                volume,
                pitch,
                spatialBlend,
                minDistance,
                maxDistance,
                clipId,
                clipDuration,
                priority,
                looping,
                playOnAwake,
                mute,
                enabled);
            return true;
        }

        private bool _hasManagedAudioListener;

        private void SyncAudioBeforeTick()
        {
            if (_isDisposed || _runtimeHandle == 0)
            {
                return;
            }

            if (AudioListener.pause && !_isPaused)
            {
                RuntimeInterop.unitas_runtime_audio_set_paused(_runtimeHandle, 1U);
            }
            else if (!AudioListener.pause && !_isPaused)
            {
                RuntimeInterop.unitas_runtime_audio_set_paused(_runtimeHandle, 0U);
            }

            bool foundListener = false;
            AudioListener[] listeners = UnityEngine.Object.FindObjectsOfType<AudioListener>();
            for (int index = 0; index < listeners.Length; index++)
            {
                AudioListener listener = listeners[index];
                if (listener != null && listener.isActiveAndEnabled)
                {
                    Vector3 pos = listener.transform.position;
                    Vector3 fwd = listener.forward;
                    RuntimeInterop.unitas_runtime_audio_set_listener(_runtimeHandle, pos.x, pos.y, pos.z, fwd.x, fwd.y, fwd.z);
                    foundListener = true;
                    _hasManagedAudioListener = true;
                    break;
                }
            }

            if (!foundListener && _hasManagedAudioListener)
            {
                RuntimeInterop.unitas_runtime_audio_clear_listener(_runtimeHandle);
            }

            SyncAudioSourceTransforms();
        }

        private void ApplySceneAudioSourceComponent(ulong entity, GameObject owner)
        {
            ulong nativeEntity = entity;
            bool hasParsed = _sceneAudioSources.TryGetValue(entity, out SceneAudioSourceMetadata parsed);
            if (!TryGetAudioSourceConfig(nativeEntity, out AudioSourceConfig audioConfig))
            {
                if (!hasParsed)
                {
                    return;
                }

                if (_alignSceneEntityBase && TryGetAudioSourceConfig(entity + 1UL, out audioConfig))
                {
                    nativeEntity = entity + 1UL;
                }
                else
                {
                    audioConfig = parsed.ToAudioSourceConfig();
                }
            }

            AudioSource audioSource = owner.GetComponent<AudioSource>() ?? owner.AddComponent<AudioSource>();
            string clipName = hasParsed && !string.IsNullOrEmpty(parsed.ClipName)
                ? parsed.ClipName
                : "SceneAudioClip";
            AudioClip? loadedClip = null;
            // "scene" binds the clip the runtime imported for the serialized
            // reference; "path" re-loads the asset and registers a second copy.
            string binding = "none";
            if (audioConfig.ClipId != 0UL)
            {
                binding = "scene";
                if (TryGetAudioClipInfo(audioConfig.ClipId, out uint channels, out uint sampleRate, out ulong _, out double durationSeconds))
                {
                    loadedClip = new AudioClip(clipName, (float)durationSeconds, (int)channels, (int)sampleRate, audioConfig.ClipId);
                }
                else
                {
                    loadedClip = new AudioClip(clipName, (float)audioConfig.ClipDuration, 1, 44100, audioConfig.ClipId);
                }
            }
            else if (hasParsed && !string.IsNullOrEmpty(parsed.ClipPath))
            {
                binding = "path";
                loadedClip = LoadClip(parsed.ClipPath);
                if (loadedClip != null && loadedClip.NativeClipId != 0UL)
                {
                    audioConfig = new AudioSourceConfig(
                        audioConfig.Volume,
                        audioConfig.Pitch,
                        audioConfig.SpatialBlend,
                        audioConfig.MinDistance,
                        audioConfig.MaxDistance,
                        loadedClip.NativeClipId,
                        loadedClip.length,
                        audioConfig.Priority,
                        audioConfig.Looping,
                        audioConfig.PlayOnAwake,
                        audioConfig.Mute,
                        audioConfig.Enabled);
                }
            }

            audioSource.ApplyNativeConfig(nativeEntity, audioConfig, loadedClip);
            if (loadedClip != null && loadedClip.NativeClipId != 0UL)
            {
                Debug.Log($"[scene-audio] source={owner.name} clip={loadedClip.name} binding={binding} native_clip_id={loadedClip.NativeClipId} samples={loadedClip.samples} channels={loadedClip.channels} sample_rate={loadedClip.frequency}");
            }
        }

        private readonly struct SceneAudioSourceMetadata
        {
            public SceneAudioSourceMetadata(
                string clipGuid,
                string clipPath,
                string clipName,
                float volume,
                float pitch,
                float spatialBlend,
                float minDistance,
                float maxDistance,
                int priority,
                bool looping,
                bool playOnAwake,
                bool mute,
                bool enabled)
            {
                ClipGuid = clipGuid;
                ClipPath = clipPath;
                ClipName = clipName;
                Volume = volume;
                Pitch = pitch;
                SpatialBlend = spatialBlend;
                MinDistance = minDistance;
                MaxDistance = maxDistance;
                Priority = priority;
                Looping = looping;
                PlayOnAwake = playOnAwake;
                Mute = mute;
                Enabled = enabled;
            }

            public string ClipGuid { get; }

            public string ClipPath { get; }

            public string ClipName { get; }

            public float Volume { get; }

            public float Pitch { get; }

            public float SpatialBlend { get; }

            public float MinDistance { get; }

            public float MaxDistance { get; }

            public int Priority { get; }

            public bool Looping { get; }

            public bool PlayOnAwake { get; }

            public bool Mute { get; }

            public bool Enabled { get; }

            public AudioSourceConfig ToAudioSourceConfig()
            {
                return new AudioSourceConfig(
                    Volume,
                    Pitch,
                    SpatialBlend,
                    MinDistance,
                    MaxDistance,
                    0UL,
                    0.0,
                    Priority,
                    Looping,
                    PlayOnAwake,
                    Mute,
                    Enabled);
            }
        }
    }
}
