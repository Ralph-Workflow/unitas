namespace UnityEngine
{
    /// The authored trail settings the native trail keeps.
    public struct TrailSettings
    {
        public float Time;
        public float StartWidth;
        public float EndWidth;
        public float MinVertexDistance;
        public Color StartColor;
        public Color EndColor;

        public TrailSettings(
            float time,
            float startWidth,
            float endWidth,
            float minVertexDistance,
            Color startColor,
            Color endColor)
        {
            Time = time;
            StartWidth = startWidth;
            EndWidth = endWidth;
            MinVertexDistance = minVertexDistance;
            StartColor = startColor;
            EndColor = endColor;
        }
    }

    /// The native trail a managed TrailRenderer reads and writes.
    public interface ITrailProvider
    {
        bool TrySetTrailEmitting(ulong entity, bool emitting);

        bool TryGetTrailEmitting(ulong entity, out bool emitting);

        bool TrySetTrailSettings(ulong entity, TrailSettings settings);

        bool TryGetTrailSettings(ulong entity, out TrailSettings settings);
    }

    public sealed class TrailRenderer : Behaviour
    {
        private const float DefaultTime = 5f;
        private const float DefaultStartWidth = 1f;
        private const float DefaultEndWidth = 0f;
        private const float DefaultMinVertexDistance = 0f;

        /// The native trail the managed component reads and writes.
        public static ITrailProvider? Provider { get; set; }

        private bool _emitting = true;
        private TrailSettings _settings = new TrailSettings(
            DefaultTime,
            DefaultStartWidth,
            DefaultEndWidth,
            DefaultMinVertexDistance,
            Color.white,
            new Color(1f, 1f, 1f, 0f));

        public bool emitting
        {
            get
            {
                if (TryNativeEntity(out ulong entity) && Provider!.TryGetTrailEmitting(entity, out bool native))
                {
                    return native;
                }
                return _emitting;
            }
            set
            {
                _emitting = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider!.TrySetTrailEmitting(entity, value);
                }
            }
        }

        public float time
        {
            get => ReadSettings().Time;
            set
            {
                TrailSettings settings = ReadSettings();
                settings.Time = value;
                WriteSettings(settings);
            }
        }

        public float startWidth
        {
            get => ReadSettings().StartWidth;
            set
            {
                TrailSettings settings = ReadSettings();
                settings.StartWidth = value;
                WriteSettings(settings);
            }
        }

        public float endWidth
        {
            get => ReadSettings().EndWidth;
            set
            {
                TrailSettings settings = ReadSettings();
                settings.EndWidth = value;
                WriteSettings(settings);
            }
        }

        public Color startColor
        {
            get => ReadSettings().StartColor;
            set
            {
                TrailSettings settings = ReadSettings();
                settings.StartColor = value;
                WriteSettings(settings);
            }
        }

        public Color endColor
        {
            get => ReadSettings().EndColor;
            set
            {
                TrailSettings settings = ReadSettings();
                settings.EndColor = value;
                WriteSettings(settings);
            }
        }

        public float minVertexDistance
        {
            get => ReadSettings().MinVertexDistance;
            set
            {
                TrailSettings settings = ReadSettings();
                settings.MinVertexDistance = value;
                WriteSettings(settings);
            }
        }

        private TrailSettings ReadSettings()
        {
            if (TryNativeEntity(out ulong entity) && Provider!.TryGetTrailSettings(entity, out TrailSettings native))
            {
                return native;
            }
            return _settings;
        }

        private void WriteSettings(TrailSettings settings)
        {
            _settings = settings;
            if (TryNativeEntity(out ulong entity))
            {
                Provider!.TrySetTrailSettings(entity, settings);
            }
        }

        private bool TryNativeEntity(out ulong entity)
        {
            entity = 0;
            if (Provider == null)
            {
                return false;
            }
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return false;
            }
            entity = owner.NativeEntityId;
            return entity != 0;
        }
    }
}
