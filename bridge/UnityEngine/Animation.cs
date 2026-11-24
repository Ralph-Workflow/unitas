using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public enum AnimationWrapMode
    {
        Once = 0,
        Loop = 1,
        PingPong = 2,
        ClampForever = 3,
    }

    public enum KeyframeInterpolationMode
    {
        Hermite = 0,
        Linear = 1,
        Stepped = 2,
    }

    public enum AnimationPropertyType
    {
        Float = 0,
        Vector3 = 1,
        Quaternion = 2,
        Color = 3,
    }

    public struct Keyframe
    {
        public Keyframe(float time, float value)
        {
            this.time = time;
            this.value = value;
            inTangent = 0f;
            outTangent = 0f;
            interpolation = KeyframeInterpolationMode.Hermite;
        }

        public Keyframe(float time, float value, float inTangent, float outTangent, KeyframeInterpolationMode interpolation)
        {
            this.time = time;
            this.value = value;
            this.inTangent = inTangent;
            this.outTangent = outTangent;
            this.interpolation = interpolation;
        }

        public float time { get; set; }

        public float value { get; set; }

        public float inTangent { get; set; }

        public float outTangent { get; set; }

        public KeyframeInterpolationMode interpolation { get; set; }
    }

    public sealed class AnimationCurve
    {
        private const float TimeEpsilon = 1e-6f;
        private readonly List<Keyframe> _keys = new List<Keyframe>();

        public AnimationCurve()
        {
        }

        public AnimationCurve(params Keyframe[] keys)
        {
            if (keys != null)
            {
                for (int index = 0; index < keys.Length; index++)
                {
                    AddKey(keys[index]);
                }
            }
        }

        /// <summary>Builds a hermite curve whose interior tangents are clamped to the neighbouring slopes.</summary>
        public static AnimationCurve ClampedAuto(params Keyframe[] keys)
        {
            AnimationCurve curve = new AnimationCurve();
            if (keys == null)
            {
                return curve;
            }

            List<Keyframe> sorted = new List<Keyframe>();
            for (int index = 0; index < keys.Length; index++)
            {
                sorted.Add(keys[index]);
            }

            sorted.Sort((left, right) => left.time.CompareTo(right.time));
            int count = sorted.Count;
            float[] tangents = new float[count];
            for (int index = 1; index < count - 1; index++)
            {
                Keyframe previous = sorted[index - 1];
                Keyframe current = sorted[index];
                Keyframe next = sorted[index + 1];
                float deltaPrevious = current.value - previous.value;
                float deltaNext = next.value - current.value;
                float timePrevious = current.time - previous.time;
                float timeNext = next.time - current.time;
                float timeTotal = next.time - previous.time;
                if (deltaPrevious * deltaNext <= 0f
                    || timePrevious <= TimeEpsilon
                    || timeNext <= TimeEpsilon
                    || timeTotal <= TimeEpsilon)
                {
                    tangents[index] = 0f;
                    continue;
                }

                float slopePrevious = deltaPrevious / timePrevious;
                float slopeNext = deltaNext / timeNext;
                float slope = (next.value - previous.value) / timeTotal;
                float limit = Math.Min(3f * Math.Abs(slopePrevious), 3f * Math.Abs(slopeNext));
                tangents[index] = Math.Max(-limit, Math.Min(limit, slope));
            }

            for (int index = 0; index < count; index++)
            {
                Keyframe key = sorted[index];
                key.inTangent = tangents[index];
                key.outTangent = tangents[index];
                key.interpolation = KeyframeInterpolationMode.Hermite;
                curve.AddKey(key);
            }

            return curve;
        }

        public int length => _keys.Count;

        public Keyframe[] keys => _keys.ToArray();

        public Keyframe this[int index] => _keys[index];

        public int AddKey(Keyframe key)
        {
            int index = _keys.Count;
            while (index > 0 && _keys[index - 1].time > key.time)
            {
                index--;
            }

            _keys.Insert(index, key);
            return index;
        }

        public int AddKey(float time, float value) => AddKey(new Keyframe(time, value));

        public void RemoveKey(int index) => _keys.RemoveAt(index);

        public void ClearKeys() => _keys.Clear();

        public float Evaluate(float time)
        {
            if (_keys.Count == 0)
            {
                return 0f;
            }

            Keyframe first = _keys[0];
            if (float.IsNaN(time) || float.IsInfinity(time) || time <= first.time)
            {
                return first.value;
            }

            Keyframe last = _keys[_keys.Count - 1];
            if (time >= last.time)
            {
                return last.value;
            }

            int segment = 0;
            for (int index = 0; index < _keys.Count - 1; index++)
            {
                if (time >= _keys[index].time && time <= _keys[index + 1].time)
                {
                    segment = index;
                    break;
                }
            }

            Keyframe start = _keys[segment];
            Keyframe end = _keys[segment + 1];
            float span = end.time - start.time;
            if (span <= TimeEpsilon || Math.Abs(time - start.time) <= TimeEpsilon)
            {
                return start.value;
            }

            if (Math.Abs(time - end.time) <= TimeEpsilon)
            {
                return end.value;
            }

            float blend = Math.Max(0f, Math.Min(1f, (time - start.time) / span));
            if (start.interpolation == KeyframeInterpolationMode.Stepped)
            {
                return start.value;
            }

            if (start.interpolation == KeyframeInterpolationMode.Linear)
            {
                return start.value + ((end.value - start.value) * blend);
            }

            float timeSquared = blend * blend;
            float timeCubed = timeSquared * blend;
            float basisStart = (2f * timeCubed) - (3f * timeSquared) + 1f;
            float basisStartTangent = timeCubed - (2f * timeSquared) + blend;
            float basisEnd = (-2f * timeCubed) + (3f * timeSquared);
            float basisEndTangent = timeCubed - timeSquared;
            return (basisStart * start.value)
                + (basisStartTangent * span * start.outTangent)
                + (basisEnd * end.value)
                + (basisEndTangent * span * end.inTangent);
        }
    }

    public sealed class AnimationCurveBinding
    {
        public AnimationCurveBinding(string objectPath, string component, string property, AnimationPropertyType propertyType, AnimationCurve curve)
        {
            this.objectPath = objectPath ?? string.Empty;
            this.component = component ?? string.Empty;
            this.property = property ?? string.Empty;
            this.propertyType = propertyType;
            this.curve = curve;
        }

        public string objectPath { get; }

        public string component { get; }

        public string property { get; }

        public AnimationPropertyType propertyType { get; }

        public AnimationCurve curve { get; }
    }

    public sealed class AnimationEvent
    {
        public AnimationEvent(float time, string functionName)
            : this(time, functionName, 0f, 0, string.Empty)
        {
        }

        public AnimationEvent(float time, string functionName, float floatParameter, int intParameter, string stringParameter)
        {
            this.time = time;
            this.functionName = functionName ?? string.Empty;
            this.floatParameter = floatParameter;
            this.intParameter = intParameter;
            this.stringParameter = stringParameter ?? string.Empty;
        }

        public float time { get; }

        public string functionName { get; }

        public float floatParameter { get; }

        public int intParameter { get; }

        public string stringParameter { get; }
    }

    public sealed class AnimationClip
    {
        private readonly List<AnimationCurveBinding> _curves = new List<AnimationCurveBinding>();
        private readonly List<AnimationEvent> _events = new List<AnimationEvent>();

        public AnimationClip(string name)
        {
            this.name = name ?? string.Empty;
            duration = 0f;
            sampleRate = 60f;
            wrapMode = AnimationWrapMode.Loop;
        }

        public string name { get; set; }

        public float duration { get; set; }

        public float sampleRate { get; set; }

        public AnimationWrapMode wrapMode { get; set; }

        public IReadOnlyList<AnimationCurveBinding> curves => _curves;

        public IReadOnlyList<AnimationEvent> events => _events;

        /// <summary>
        /// Binds a curve to a property. Vector, quaternion and color bindings replay the same curve on every channel.
        /// </summary>
        public void SetCurve(string path, AnimationPropertyType propertyType, string propertyName, AnimationCurve curve, string component = "Transform")
        {
            if (curve == null)
            {
                throw new ArgumentNullException(nameof(curve));
            }

            _curves.Add(new AnimationCurveBinding(path, component, propertyName, propertyType, curve));
        }

        public int AddEvent(float time, string functionName)
        {
            _events.Add(new AnimationEvent(time, functionName));
            return _events.Count - 1;
        }

        public int AddEvent(float time, string functionName, float floatParameter, int intParameter, string stringParameter)
        {
            _events.Add(new AnimationEvent(time, functionName, floatParameter, intParameter, stringParameter));
            return _events.Count - 1;
        }
    }
}
