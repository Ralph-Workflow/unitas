using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public enum AnimatorParameterType
    {
        Float = 0,
        Int = 1,
        Bool = 2,
        Trigger = 3,
    }

    public enum AnimatorConditionMode
    {
        If = 0,
        IfNot = 1,
        Greater = 2,
        Less = 3,
        Equals = 4,
        NotEqual = 5,
    }

    public struct AnimatorCondition
    {
        public AnimatorCondition(string parameter, AnimatorConditionMode mode)
            : this(parameter, mode, 0f, 0)
        {
        }

        public AnimatorCondition(string parameter, AnimatorConditionMode mode, float floatValue)
            : this(parameter, mode, floatValue, 0)
        {
        }

        public AnimatorCondition(string parameter, AnimatorConditionMode mode, int intValue)
            : this(parameter, mode, 0f, intValue)
        {
        }

        public AnimatorCondition(string parameter, AnimatorConditionMode mode, float floatValue, int intValue)
        {
            this.parameter = parameter ?? string.Empty;
            this.mode = mode;
            this.floatValue = floatValue;
            this.intValue = intValue;
        }

        public string parameter { get; set; }

        public AnimatorConditionMode mode { get; set; }

        public float floatValue { get; set; }

        public int intValue { get; set; }
    }

    public sealed class AnimatorControllerParameter
    {
        public AnimatorControllerParameter(string name, AnimatorParameterType type, float floatValue, int intValue, bool boolValue)
        {
            this.name = name ?? string.Empty;
            this.type = type;
            this.floatValue = floatValue;
            this.intValue = intValue;
            this.boolValue = boolValue;
        }

        public string name { get; }

        public AnimatorParameterType type { get; }

        public float floatValue { get; }

        public int intValue { get; }

        public bool boolValue { get; }
    }

    public sealed class AnimatorControllerState
    {
        public AnimatorControllerState(string name, AnimationClip? clip, float speed, bool isExit)
        {
            this.name = name ?? string.Empty;
            this.clip = clip;
            this.speed = speed;
            this.isExit = isExit;
        }

        public string name { get; }

        public AnimationClip? clip { get; }

        public float speed { get; }

        public bool isExit { get; }
    }

    public sealed class AnimatorControllerTransition
    {
        public AnimatorControllerTransition(string sourceState, string targetState, float duration, bool hasExitTime, float exitTime, AnimatorCondition[] conditions)
        {
            this.sourceState = sourceState ?? string.Empty;
            this.targetState = targetState ?? string.Empty;
            this.duration = duration;
            this.hasExitTime = hasExitTime;
            this.exitTime = exitTime;
            this.conditions = conditions ?? Array.Empty<AnimatorCondition>();
        }

        public string sourceState { get; }

        public string targetState { get; }

        public float duration { get; }

        public bool hasExitTime { get; }

        public float exitTime { get; }

        public IReadOnlyList<AnimatorCondition> conditions { get; }
    }

    public sealed class AnimatorController
    {
        private readonly List<AnimatorControllerParameter> _parameters = new List<AnimatorControllerParameter>();
        private readonly List<AnimatorControllerState> _states = new List<AnimatorControllerState>();
        private readonly List<AnimatorControllerTransition> _transitions = new List<AnimatorControllerTransition>();
        private string _entryState = string.Empty;

        public AnimatorController(string name)
        {
            this.name = name ?? string.Empty;
        }

        public string name { get; set; }

        public IReadOnlyList<AnimatorControllerParameter> parameters => _parameters;

        public IReadOnlyList<AnimatorControllerState> states => _states;

        public IReadOnlyList<AnimatorControllerTransition> transitions => _transitions;

        public string entryState => _entryState;

        public void AddParameter(string name, float defaultValue)
        {
            _parameters.Add(new AnimatorControllerParameter(name, AnimatorParameterType.Float, defaultValue, 0, false));
        }

        public void AddParameter(string name, int defaultValue)
        {
            _parameters.Add(new AnimatorControllerParameter(name, AnimatorParameterType.Int, 0f, defaultValue, false));
        }

        public void AddParameter(string name, bool defaultValue)
        {
            _parameters.Add(new AnimatorControllerParameter(name, AnimatorParameterType.Bool, 0f, 0, defaultValue));
        }

        public void AddTriggerParameter(string name)
        {
            _parameters.Add(new AnimatorControllerParameter(name, AnimatorParameterType.Trigger, 0f, 0, false));
        }

        public void AddState(string name, AnimationClip? clip)
        {
            _states.Add(new AnimatorControllerState(name, clip, 1f, false));
        }

        public void AddTransition(string fromState, string toState, params AnimatorCondition[] conditions)
        {
            AddTransition(fromState, toState, 0f, false, 0f, conditions);
        }

        public void AddTransition(string fromState, string toState, float duration, params AnimatorCondition[] conditions)
        {
            AddTransition(fromState, toState, duration, false, 0f, conditions);
        }

        public void AddTransition(string fromState, string toState, float duration, bool hasExitTime, float exitTime, params AnimatorCondition[] conditions)
        {
            if (!ContainsState(fromState))
            {
                throw new ArgumentException("Unknown source state: " + fromState, nameof(fromState));
            }

            if (!ContainsState(toState))
            {
                throw new ArgumentException("Unknown target state: " + toState, nameof(toState));
            }

            _transitions.Add(new AnimatorControllerTransition(fromState, toState, duration, hasExitTime, exitTime, conditions));
        }

        public void SetEntryState(string name)
        {
            if (!ContainsState(name))
            {
                throw new ArgumentException("Unknown state: " + name, nameof(name));
            }

            _entryState = name;
        }

        public int IndexOfState(string name)
        {
            for (int index = 0; index < _states.Count; index++)
            {
                if (string.Equals(_states[index].name, name, StringComparison.Ordinal))
                {
                    return index;
                }
            }

            return -1;
        }

        private bool ContainsState(string name)
        {
            return IndexOfState(name) >= 0;
        }
    }
}
