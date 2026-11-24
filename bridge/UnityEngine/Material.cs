using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Color : IEquatable<Color>
    {
        public float r;

        public float g;

        public float b;

        public float a;

        public Color(float r, float g, float b, float a)
        {
            this.r = r;
            this.g = g;
            this.b = b;
            this.a = a;
        }

        public static Color white => new Color(1f, 1f, 1f, 1f);

        public static Color black => new Color(0f, 0f, 0f, 1f);

        public static Color clear => new Color(0f, 0f, 0f, 0f);

        public bool Equals(Color other) => r == other.r && g == other.g && b == other.b && a == other.a;

        public override int GetHashCode() => HashCode.Combine(r, g, b, a);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "RGBA({0:F3}, {1:F3}, {2:F3}, {3:F3})", r, g, b, a);
    }

    public sealed class Material : Object
    {
        private const string DefaultShader = "Unitas/Standard";

        private ulong _nativeMaterialHandle;
        private Color _color = Color.white;
        private string _shader = DefaultShader;

        public static IRenderProvider? Provider { get; set; }

        public Color color
        {
            get => _color;
            set => SetColor(value);
        }

        public string shader
        {
            get => _shader;
            set => _shader = value ?? string.Empty;
        }

        public void SetColor(Color value)
        {
            bool existed = _nativeMaterialHandle != 0UL;
            _color = value;
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong materialHandle = EnsureNativeMaterial();
            if (materialHandle == 0UL)
            {
                return;
            }

            if (existed)
            {
                provider.SetMaterialColor(materialHandle, value);
            }
        }

        internal ulong NativeMaterialHandle => EnsureNativeMaterial();

        private ulong EnsureNativeMaterial()
        {
            if (_nativeMaterialHandle != 0UL)
            {
                return _nativeMaterialHandle;
            }

            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            _nativeMaterialHandle = provider.CreateMaterial();
            if (_nativeMaterialHandle != 0UL)
            {
                provider.SetMaterialColor(_nativeMaterialHandle, _color);
            }

            return _nativeMaterialHandle;
        }
    }
}
