namespace UnityEngine
{
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
