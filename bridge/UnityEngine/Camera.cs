using System;
using System.IO;

namespace UnityEngine
{
    public interface IRenderProvider
    {
        ulong CreateMesh(Vector3[] vertices, Vector3[] normals, int[] triangles);

        void DestroyMesh(ulong meshHandle);

        ulong CreateMaterial();

        void SetMaterialColor(ulong materialHandle, Color color);

        ulong CreateCamera(ulong entityId);

        void UpdateCamera(ulong cameraHandle, float orthographicSize, float fieldOfView, float nearClipPlane, float farClipPlane, Color backgroundColor);

        Vector3 WorldToScreenPoint(ulong cameraHandle, Vector3 worldPoint);

        Ray ScreenPointToRay(ulong cameraHandle, Vector3 screenPoint);

        ulong CreateRenderer(ulong entityId, ulong meshHandle, ulong materialHandle);

        void SetRendererEnabled(ulong rendererHandle, bool enabled);

        ulong CreateLight(ulong entityId);

        void UpdateLight(ulong lightHandle, Vector3 direction, Color color, float intensity);

        byte[] RenderFrame(ulong cameraHandle, uint width, uint height);
    }

    public sealed class Camera : Behaviour
    {
        private const float DefaultFieldOfView = 60f;
        private const float DefaultNearClipPlane = 0.3f;
        private const float DefaultFarClipPlane = 1000f;
        private const float DefaultOrthographicSize = 5f;
        private const float MinimumOrthographicSize = 0.0001f;
        private const int DefaultPixelWidth = 256;
        private const int DefaultPixelHeight = 256;

        private ulong _nativeCameraHandle;
        private bool _orthographic;
        private float _orthographicSize = DefaultOrthographicSize;
        private float _fieldOfView = DefaultFieldOfView;
        private float _nearClipPlane = DefaultNearClipPlane;
        private float _farClipPlane = DefaultFarClipPlane;
        private Color _backgroundColor = Color.black;
        private int _pixelWidth = DefaultPixelWidth;
        private int _pixelHeight = DefaultPixelHeight;

        public static IRenderProvider? Provider { get; set; }

        public bool orthographic
        {
            get => _orthographic;
            set
            {
                _orthographic = value;
                PushSettings();
            }
        }

        public float orthographicSize
        {
            get => _orthographicSize;
            set
            {
                _orthographicSize = value;
                PushSettings();
            }
        }

        public float fieldOfView
        {
            get => _fieldOfView;
            set
            {
                _fieldOfView = value;
                PushSettings();
            }
        }

        public float nearClipPlane
        {
            get => _nearClipPlane;
            set
            {
                _nearClipPlane = value;
                PushSettings();
            }
        }

        public float farClipPlane
        {
            get => _farClipPlane;
            set
            {
                _farClipPlane = value;
                PushSettings();
            }
        }

        public Color backgroundColor
        {
            get => _backgroundColor;
            set
            {
                _backgroundColor = value;
                PushSettings();
            }
        }

        public int pixelWidth
        {
            get => _pixelWidth;
            set
            {
                _pixelWidth = value < 1 ? 1 : value;
            }
        }

        public int pixelHeight
        {
            get => _pixelHeight;
            set
            {
                _pixelHeight = value < 1 ? 1 : value;
            }
        }

        public Vector3 WorldToScreenPoint(Vector3 worldPoint)
        {
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return Vector3.zero;
            }

            ulong cameraHandle = EnsureNativeCamera();
            if (cameraHandle == 0UL)
            {
                return Vector3.zero;
            }

            return provider.WorldToScreenPoint(cameraHandle, worldPoint);
        }

        public Ray ScreenPointToRay(Vector3 screenPoint)
        {
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return default;
            }

            ulong cameraHandle = EnsureNativeCamera();
            if (cameraHandle == 0UL)
            {
                return default;
            }

            return provider.ScreenPointToRay(cameraHandle, screenPoint);
        }

        public Vector3 ViewportToWorldPoint(Vector3 viewportPoint)
        {
            Vector3 screenPoint = new Vector3(viewportPoint.x * _pixelWidth, viewportPoint.y * _pixelHeight, 0f);
            Ray ray = ScreenPointToRay(screenPoint);
            return ray.origin + (ray.direction * viewportPoint.z);
        }

        public void CaptureToFile(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                throw new ArgumentException("Capture path must be non-empty.", nameof(path));
            }

            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong cameraHandle = EnsureNativeCamera();
            if (cameraHandle == 0UL)
            {
                return;
            }

            byte[] frame = provider.RenderFrame(cameraHandle, (uint)_pixelWidth, (uint)_pixelHeight);
            if (frame.Length == 0)
            {
                return;
            }

            File.WriteAllBytes(path, frame);
        }

        private void PushSettings()
        {
            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }

            ulong cameraHandle = EnsureNativeCamera();
            if (cameraHandle == 0UL)
            {
                return;
            }

            provider.UpdateCamera(
                cameraHandle,
                _orthographic ? Math.Max(_orthographicSize, MinimumOrthographicSize) : 0f,
                _fieldOfView,
                _nearClipPlane,
                _farClipPlane,
                _backgroundColor);
        }

        private ulong EnsureNativeCamera()
        {
            if (_nativeCameraHandle != 0UL)
            {
                return _nativeCameraHandle;
            }

            IRenderProvider? provider = Provider;
            if (provider == null)
            {
                return 0UL;
            }

            GameObject? owner = gameObject;
            ulong entityId = owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            _nativeCameraHandle = provider.CreateCamera(entityId);
            if (_nativeCameraHandle != 0UL)
            {
                PushSettings();
            }

            return _nativeCameraHandle;
        }
    }
}
