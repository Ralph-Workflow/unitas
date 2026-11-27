using System;
using System.Globalization;

namespace UnityEngine.Rendering
{
    public enum GraphicsDeviceType
    {
        OpenGL2 = 0,
        Direct3D9 = 1,
        Direct3D11 = 2,
        PlayStation3 = 3,
        Null = 4,
        Xbox360 = 6,
        OpenGLES2 = 8,
        OpenGLES3 = 11,
        PlayStationVita = 12,
        PlayStation4 = 13,
        XboxOne = 14,
        PlayStationMobile = 15,
        Metal = 16,
        OpenGLCore = 17,
        Direct3D12 = 18,
        N3DS = 19,
        Vulkan = 21,
        Switch = 22,
        XboxOneD3D12 = 23,
        GameCoreXboxOne = 24,
        GameCoreScarlett = 25,
        PlayStation5 = 26,
        PlayStation5NGGC = 27,
    }
}

namespace UnityEngine
{
    public enum ScreenOrientation
    {
        Unknown = 0,
        Portrait = 1,
        PortraitUpsideDown = 2,
        LandscapeLeft = 3,
        Landscape = 3,
        LandscapeRight = 4,
        AutoRotation = 5,
    }

    public enum FullScreenMode
    {
        ExclusiveFullScreen = 0,
        FullScreenWindow = 1,
        MaximizedWindow = 2,
        Windowed = 3,
    }

    public static class SleepTimeout
    {
        public const int NeverSleep = -1;
        public const int SystemSetting = -2;
    }

    public struct Resolution : IEquatable<Resolution>
    {
        public int width { get; set; }

        public int height { get; set; }

        public int refreshRate { get; set; }

        public bool Equals(Resolution other) =>
            width == other.width && height == other.height && refreshRate == other.refreshRate;

        public override bool Equals(object? obj) => obj is Resolution other && Equals(other);

        public override int GetHashCode() => HashCode.Combine(width, height, refreshRate);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "{0} x {1} @ {2}Hz", width, height, refreshRate);

        public static bool operator ==(Resolution left, Resolution right) => left.Equals(right);

        public static bool operator !=(Resolution left, Resolution right) => !left.Equals(right);
    }

    public static class Screen
    {
        private const int DefaultWidth = 1280;
        private const int DefaultHeight = 720;
        private const int DefaultRefreshRate = 60;
        private const float DefaultDpi = 96f;

        private static int _width = DefaultWidth;
        private static int _height = DefaultHeight;
        private static int _refreshRate = DefaultRefreshRate;
        private static float _dpi = DefaultDpi;
        private static bool _fullScreen;
        private static FullScreenMode _fullScreenMode = FullScreenMode.Windowed;
        private static ScreenOrientation _orientation = ScreenOrientation.LandscapeLeft;
        private static int _sleepTimeout = SleepTimeout.SystemSetting;

        public static Action<int, int, bool>? ResolutionChangeHandler { get; set; }

        public static Action<bool>? FullScreenChangeHandler { get; set; }

        public static int width
        {
            get => _width;
            set => _width = value > 0 ? value : DefaultWidth;
        }

        public static int height
        {
            get => _height;
            set => _height = value > 0 ? value : DefaultHeight;
        }

        public static float dpi
        {
            get => _dpi;
            set => _dpi = value > 0f && float.IsFinite(value) ? value : DefaultDpi;
        }

        public static bool fullScreen
        {
            get => _fullScreen;
            set
            {
                _fullScreen = value;
                _fullScreenMode = value ? FullScreenMode.FullScreenWindow : FullScreenMode.Windowed;
                FullScreenChangeHandler?.Invoke(_fullScreen);
            }
        }

        public static FullScreenMode fullScreenMode
        {
            get => _fullScreenMode;
            set
            {
                _fullScreenMode = value;
                _fullScreen = value != FullScreenMode.Windowed;
                FullScreenChangeHandler?.Invoke(_fullScreen);
            }
        }

        public static ScreenOrientation orientation
        {
            get => _orientation;
            set => _orientation = value;
        }

        public static int sleepTimeout
        {
            get => _sleepTimeout;
            set => _sleepTimeout = value;
        }

        public static Resolution currentResolution => new Resolution
        {
            width = _width,
            height = _height,
            refreshRate = _refreshRate,
        };

        public static Resolution[] resolutions => new Resolution[] { currentResolution };

        public static void SetResolution(int width, int height, bool fullscreen)
        {
            SetResolution(width, height, fullscreen ? FullScreenMode.FullScreenWindow : FullScreenMode.Windowed, _refreshRate);
        }

        public static void SetResolution(int width, int height, bool fullscreen, int preferredRefreshRate)
        {
            SetResolution(width, height, fullscreen ? FullScreenMode.FullScreenWindow : FullScreenMode.Windowed, preferredRefreshRate);
        }

        public static void SetResolution(int width, int height, FullScreenMode fullscreenMode)
        {
            SetResolution(width, height, fullscreenMode, _refreshRate);
        }

        public static void SetResolution(int width, int height, FullScreenMode fullscreenMode, int preferredRefreshRate)
        {
            _width = width > 0 ? width : DefaultWidth;
            _height = height > 0 ? height : DefaultHeight;
            _refreshRate = preferredRefreshRate > 0 ? preferredRefreshRate : DefaultRefreshRate;
            _fullScreenMode = fullscreenMode;
            _fullScreen = fullscreenMode != FullScreenMode.Windowed;
            ResolutionChangeHandler?.Invoke(_width, _height, _fullScreen);
        }

        public static void ResetToDefaults()
        {
            _width = DefaultWidth;
            _height = DefaultHeight;
            _refreshRate = DefaultRefreshRate;
            _dpi = DefaultDpi;
            _fullScreen = false;
            _fullScreenMode = FullScreenMode.Windowed;
            _orientation = ScreenOrientation.LandscapeLeft;
            _sleepTimeout = SleepTimeout.SystemSetting;
        }
    }

    public static class SystemInfo
    {
        private const string NullGraphicsDeviceName = "Null Device";
        private const string NullGraphicsDeviceVendor = "Unity Technologies";
        private const string NullGraphicsDeviceVersion = "NULL 1.0 [1.0]";

        public static Rendering.GraphicsDeviceType graphicsDeviceType { get; set; } =
            Rendering.GraphicsDeviceType.Null;

        public static string graphicsDeviceName => NullGraphicsDeviceName;

        public static string graphicsDeviceVendor => NullGraphicsDeviceVendor;

        public static string graphicsDeviceVersion => NullGraphicsDeviceVersion;

        public static int processorCount => Math.Max(1, Environment.ProcessorCount);

        public static string operatingSystem => Environment.OSVersion.ToString();

        public static string deviceModel => "Headless Host";

        public static string deviceName => Environment.MachineName;
    }
}
