using System;
using UnityEngine;

namespace UnityEngine.Rendering.Universal
{
    public interface ILight2DProvider
    {
        ulong CreateLight2D(ulong entityId);

        void DestroyLight2D(ulong lightHandle);

        void SetLight2DKind(ulong lightHandle, Light2D.LightType kind);

        void SetLight2DColor(ulong lightHandle, Color color);

        void SetLight2DIntensity(ulong lightHandle, float intensity);

        void SetLight2DRadii(ulong lightHandle, float innerRadius, float outerRadius);

        void SetLight2DAngles(ulong lightHandle, float innerAngle, float outerAngle);

        void SetLight2DPosition(ulong lightHandle, Vector2 position);

        void SetLight2DDirection(ulong lightHandle, Vector2 direction);

        void SetLight2DEnabled(ulong lightHandle, bool enabled);
    }

    public sealed class Light2D : MonoBehaviour
    {
        public enum LightType
        {
            Global = 0,
            Point = 1,
            Spot = 2,
        }

        private LightType _lightType = LightType.Point;
        private Color _color = Color.white;
        private float _intensity = 1f;
        private float _pointLightInnerRadius;
        private float _pointLightOuterRadius = 1f;
        private float _pointLightInnerAngle = 360f;
        private float _pointLightOuterAngle = 360f;
        private Vector2 _direction = Vector2.up;
        private bool _hasDirectionOverride;
        private Vector2 _position;
        private bool _hasPositionOverride;
        private ulong _nativeLightHandle;
        private ulong _nativeEntityId;

        private static ILight2DProvider? s_provider;
        private static Func<ILight2DProvider?>? s_defaultProviderFactory;

        public static ILight2DProvider? Provider
        {
            get => s_provider ?? s_defaultProviderFactory?.Invoke();
            set => s_provider = value;
        }

        public static void RegisterDefaultProviderFactory(Func<ILight2DProvider?>? factory)
        {
            s_defaultProviderFactory = factory;
        }

        public LightType lightType
        {
            get => _lightType;
            set
            {
                _lightType = value;
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

        public float pointLightInnerRadius
        {
            get => _pointLightInnerRadius;
            set
            {
                _pointLightInnerRadius = value;
                PushSettings();
            }
        }

        public float pointLightOuterRadius
        {
            get => _pointLightOuterRadius;
            set
            {
                _pointLightOuterRadius = value;
                PushSettings();
            }
        }

        public float pointLightInnerAngle
        {
            get => _pointLightInnerAngle;
            set
            {
                _pointLightInnerAngle = value;
                PushSettings();
            }
        }

        public float pointLightOuterAngle
        {
            get => _pointLightOuterAngle;
            set
            {
                _pointLightOuterAngle = value;
                PushSettings();
            }
        }

        public float innerSpotAngle
        {
            get => _pointLightInnerAngle * 0.5f;
            set => pointLightInnerAngle = value * 2f;
        }

        public float outerSpotAngle
        {
            get => _pointLightOuterAngle * 0.5f;
            set => pointLightOuterAngle = value * 2f;
        }

        public Vector2 direction
        {
            get
            {
                if (_hasDirectionOverride)
                {
                    return _direction;
                }

                GameObject? owner = OwnerOrNull;
                if (owner != null && !owner.IsDestroyed)
                {
                    Vector3 up = owner.transform.up;
                    return new Vector2(up.x, up.y);
                }

                return _direction;
            }
            set
            {
                _direction = value;
                _hasDirectionOverride = true;
                PushSettings();
            }
        }

        public Vector2 position
        {
            get
            {
                if (_hasPositionOverride)
                {
                    return _position;
                }

                GameObject? owner = OwnerOrNull;
                if (owner != null && !owner.IsDestroyed)
                {
                    Vector3 pos = owner.transform.position;
                    return new Vector2(pos.x, pos.y);
                }

                return _position;
            }
            set
            {
                _position = value;
                _hasPositionOverride = true;
                PushSettings();
            }
        }

        public ulong NativeLightHandle => EnsureNativeLight();

        public override void OnEnable()
        {
            base.OnEnable();
            PushSettings();
        }

        public override void OnDisable()
        {
            base.OnDisable();
            ILight2DProvider? provider = Provider;
            if (provider != null && _nativeLightHandle != 0UL)
            {
                provider.SetLight2DEnabled(_nativeLightHandle, false);
            }
        }

        public override void OnDestroy()
        {
            base.OnDestroy();
            ILight2DProvider? provider = Provider;
            if (provider != null && _nativeLightHandle != 0UL)
            {
                provider.DestroyLight2D(_nativeLightHandle);
            }

            _nativeLightHandle = 0UL;
            _nativeEntityId = 0UL;
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            PushSettings();
        }

        private void PushSettings()
        {
            ILight2DProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong handle = EnsureNativeLight();
            if (handle == 0UL)
            {
                return;
            }

            provider.SetLight2DKind(handle, _lightType);
            provider.SetLight2DColor(handle, _color);
            provider.SetLight2DIntensity(handle, _intensity);
            provider.SetLight2DRadii(handle, _pointLightInnerRadius, _pointLightOuterRadius);
            provider.SetLight2DAngles(handle, _pointLightInnerAngle * 0.5f, _pointLightOuterAngle * 0.5f);
            if (_hasPositionOverride)
            {
                provider.SetLight2DPosition(handle, _position);
            }

            if (_hasDirectionOverride)
            {
                provider.SetLight2DDirection(handle, _direction);
            }

            provider.SetLight2DEnabled(handle, isActiveAndEnabled);
        }

        private ulong EnsureNativeLight()
        {
            GameObject? owner = OwnerOrNull;
            ulong entityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            if (_nativeLightHandle != 0UL && _nativeEntityId == entityId)
            {
                return _nativeLightHandle;
            }

            ILight2DProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            if (_nativeLightHandle != 0UL)
            {
                provider.DestroyLight2D(_nativeLightHandle);
            }

            _nativeEntityId = entityId;
            _nativeLightHandle = provider.CreateLight2D(entityId);
            if (_nativeLightHandle != 0UL)
            {
                PushSettings();
            }

            return _nativeLightHandle;
        }
    }
}
