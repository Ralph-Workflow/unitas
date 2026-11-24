using System;

namespace UnityEngine
{
    public enum LightType
    {
        Directional,
        Point,
        Spot,
        Area
    }

    public sealed class Light : Behaviour
    {
        private LightType _type = LightType.Directional;
        private Color _color = Color.white;
        private float _intensity = 1f;
        private Vector3 _direction = new Vector3(0f, 1f, 0f);
        private ulong _nativeLightHandle;

        public static IRenderProvider? Provider { get; set; }

        public LightType type
        {
            get => _type;
            set
            {
                if (value != LightType.Directional)
                {
                    throw new NotSupportedException("Only directional lights are supported by the unitas renderer (unsupported type: " + value + ").");
                }

                _type = value;
                PushSettings();
            }
        }

        public Color color
        {
            get => _color;
            set
            {
                _color = value;
                PushSettings();
            }
        }

        public float intensity
        {
            get => _intensity;
            set
            {
                _intensity = value;
                PushSettings();
            }
        }

        public Vector3 direction
        {
            get => _direction;
            set
            {
                _direction = value;
                PushSettings();
            }
        }

        internal ulong NativeLightHandle => EnsureNativeLight();

        private void PushSettings()
        {
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong lightHandle = EnsureNativeLight();
            if (lightHandle == 0UL)
            {
                return;
            }

            provider.UpdateLight(lightHandle, _direction, _color, _intensity);
        }

        private ulong EnsureNativeLight()
        {
            if (_nativeLightHandle != 0UL)
            {
                return _nativeLightHandle;
            }

            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            GameObject? owner = gameObject;
            ulong entityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            _nativeLightHandle = provider.CreateLight(entityId);
            if (_nativeLightHandle != 0UL)
            {
                PushSettings();
            }

            return _nativeLightHandle;
        }
    }
}
