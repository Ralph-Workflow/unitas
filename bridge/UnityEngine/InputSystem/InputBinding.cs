using System;

namespace UnityEngine.InputSystem
{
    public enum InputActionType
    {
        Value = 0,
        Button = 1,
        PassThrough = 2
    }

    public enum InputActionPhase
    {
        Disabled = 0,
        Waiting = 1,
        Started = 2,
        Performed = 3,
        Canceled = 4
    }

    public struct InputBinding : IEquatable<InputBinding>
    {
        public InputBinding(
            string path,
            string? action = null,
            string? groups = null,
            string? processors = null,
            string? interactions = null,
            string? name = null,
            bool isComposite = false,
            bool isPartOfComposite = false)
        {
            this.path = path ?? string.Empty;
            this.action = action ?? string.Empty;
            this.groups = groups ?? string.Empty;
            this.processors = processors ?? string.Empty;
            this.interactions = interactions ?? string.Empty;
            this.name = name ?? string.Empty;
            this.isComposite = isComposite;
            this.isPartOfComposite = isPartOfComposite;
            overridePath = string.Empty;
            id = Guid.NewGuid().ToString("D");
        }

        public string id { get; }

        public string name { get; }

        public string path { get; }

        public string action { get; }

        public string groups { get; }

        public string interactions { get; }

        public string processors { get; }

        public bool isComposite { get; }

        public bool isPartOfComposite { get; }

        /// <summary>Path that replaces <see cref="path"/> while a rebind override is applied.</summary>
        public string overridePath { get; internal set; }

        public string effectivePath => string.IsNullOrEmpty(overridePath) ? path : overridePath;

        public static implicit operator InputBinding(string path)
        {
            return new InputBinding(path);
        }

        public bool Equals(InputBinding other)
        {
            return string.Equals(path, other.path, StringComparison.Ordinal)
                && string.Equals(name, other.name, StringComparison.Ordinal)
                && string.Equals(action, other.action, StringComparison.Ordinal)
                && string.Equals(groups, other.groups, StringComparison.Ordinal)
                && string.Equals(interactions, other.interactions, StringComparison.Ordinal)
                && string.Equals(processors, other.processors, StringComparison.Ordinal)
                && isComposite == other.isComposite
                && isPartOfComposite == other.isPartOfComposite;
        }

        public override bool Equals(object? obj)
        {
            return obj is InputBinding other && Equals(other);
        }

        public override int GetHashCode()
        {
            return HashCode.Combine(path, name, action, groups, interactions, processors, isComposite, isPartOfComposite);
        }

        public static bool operator ==(InputBinding left, InputBinding right)
        {
            return left.Equals(right);
        }

        public static bool operator !=(InputBinding left, InputBinding right)
        {
            return !left.Equals(right);
        }
    }

    public sealed class InputValue
    {
        private readonly float _floatValue;
        private readonly Vector2 _vector2Value;
        private readonly bool _isVector2;
        private readonly float _pressPoint;

        public InputValue()
            : this(0f, Vector2.zero, false, 0.5f)
        {
        }

        internal InputValue(float floatValue, Vector2 vector2Value, bool isVector2, float pressPoint)
        {
            _floatValue = floatValue;
            _vector2Value = vector2Value;
            _isVector2 = isVector2;
            _pressPoint = pressPoint;
        }

        public bool isPressed
        {
            get
            {
                float magnitude = _isVector2 ? _vector2Value.magnitude : MathF.Abs(_floatValue);
                return magnitude >= _pressPoint;
            }
        }

        public TValue Get<TValue>() where TValue : struct
        {
            if (typeof(TValue) == typeof(float))
            {
                float scalar = _isVector2 ? _vector2Value.magnitude : _floatValue;
                if (scalar is TValue typedFloat)
                {
                    return typedFloat;
                }
            }

            if (typeof(TValue) == typeof(Vector2))
            {
                Vector2 vec = _isVector2 ? _vector2Value : new Vector2(_floatValue, 0f);
                if (vec is TValue typedVec)
                {
                    return typedVec;
                }
            }

            if (typeof(TValue) == typeof(bool))
            {
                bool pressed = isPressed;
                if (pressed is TValue typedBool)
                {
                    return typedBool;
                }
            }

            return default;
        }
    }
}
