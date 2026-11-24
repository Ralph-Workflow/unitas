using System;

namespace UnityEngine
{
    public sealed class AudioClip : Object
    {
        public AudioClip()
        {
            name = string.Empty;
            length = 0f;
            channels = 1;
            frequency = 44100;
            samples = 0;
            NativeClipId = 0UL;
        }

        public AudioClip(string clipName, float lengthSeconds, int channelCount, int sampleRateHz, ulong nativeClipId = 0UL)
        {
            name = clipName ?? string.Empty;
            float clampedLength = Math.Max(0f, lengthSeconds);
            int clampedChannels = Math.Max(1, channelCount);
            int clampedRate = Math.Max(1, sampleRateHz);
            length = clampedLength;
            channels = clampedChannels;
            frequency = clampedRate;
            samples = (int)Math.Round(clampedLength * clampedRate);
            NativeClipId = nativeClipId;
        }

        public ulong NativeClipId { get; internal set; }

        public float length { get; private set; }

        public int channels { get; private set; }

        public int frequency { get; private set; }

        public int sampleRate => frequency;

        public int samples { get; private set; }

        public static AudioClip Create(string name, int lengthSamples, int channels, int frequency, bool stream = false)
        {
            int clampedSamples = Math.Max(0, lengthSamples);
            int clampedChannels = Math.Max(1, channels);
            int clampedFrequency = Math.Max(1, frequency);
            IAudioProvider? provider = AudioSource.Provider;
            if (provider != null)
            {
                return provider.CreateClip(name ?? string.Empty, clampedSamples, clampedChannels, clampedFrequency);
            }

            float duration = (float)clampedSamples / clampedFrequency;
            AudioClip clip = new AudioClip(name ?? string.Empty, duration, clampedChannels, clampedFrequency, 0UL)
            {
                samples = clampedSamples,
            };
            return clip;
        }

        public static AudioClip? LoadFromFile(string path)
        {
            return AudioSource.Provider?.LoadClip(path);
        }
    }
}
