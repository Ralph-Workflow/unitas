using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace UnityEngine
{
    public enum RuntimePlatform
    {
        OSXEditor = 0,
        OSXPlayer = 1,
        WindowsPlayer = 2,
        WindowsEditor = 7,
        IPhonePlayer = 8,
        Android = 11,
        LinuxPlayer = 13,
        LinuxEditor = 16,
        WebGLPlayer = 17,
        PS4 = 25,
        XboxOne = 27,
        tvOS = 31,
        Switch = 32,
        PS5 = 38,
        LinuxServer = 43,
        WindowsServer = 44,
        OSXServer = 45,
    }

    public enum NetworkReachability
    {
        NotReachable = 0,
        ReachableViaCarrierDataNetwork = 1,
        ReachableViaLocalAreaNetwork = 2,
    }

    public enum SystemLanguage
    {
        Afrikaans = 0,
        Arabic = 1,
        Basque = 2,
        Belarusian = 3,
        Bulgarian = 4,
        Catalan = 5,
        Chinese = 6,
        Czech = 7,
        Danish = 8,
        Dutch = 9,
        English = 10,
        Estonian = 11,
        Faroese = 12,
        Finnish = 13,
        French = 14,
        German = 15,
        Greek = 16,
        Hebrew = 17,
        Hungarian = 18,
        Icelandic = 19,
        Indonesian = 20,
        Italian = 21,
        Japanese = 22,
        Korean = 23,
        Latvian = 24,
        Lithuanian = 25,
        Norwegian = 26,
        Polish = 27,
        Portuguese = 28,
        Romanian = 29,
        Russian = 30,
        SerboCroatian = 31,
        Slovak = 32,
        Slovenian = 33,
        Spanish = 34,
        Swedish = 35,
        Thai = 36,
        Turkish = 37,
        Ukrainian = 38,
        Vietnamese = 39,
        ChineseSimplified = 40,
        ChineseTraditional = 41,
        Unknown = 42,
    }

    public enum ApplicationBuildKind
    {
        Editor = 0,
        DevelopmentPlayer = 1,
        ReleasePlayer = 2,
    }

    public interface IApplicationHost
    {
        bool IsPlaying { get; }

        bool IsFocused { get; }

        bool IsPaused { get; }

        ApplicationBuildKind BuildKind { get; }

        string DataPath { get; }

        string StreamingAssetsPath { get; }

        string CompanyName { get; }

        string ProductName { get; }

        string Version { get; }

        void EndSession();
    }

    public static class Application
    {
        private const string DefaultCompanyName = "DefaultCompany";
        private const string DefaultProductName = "UnitasProduct";
        private const string DefaultVersion = "0.1";
        private const string DefaultUnityVersion = "2022.3.0f1";

        private static string? _persistentDataPathOverride;
        private static string? _unityVersionOverride;
        private static RuntimePlatform? _platformOverride;
        private static int _targetFrameRate = -1;
        private static bool _runInBackground = true;
        private static bool _ignoreQuitInPlayMode;
        private static int _quitRequestCount;
        private static int _ignoredPlayModeQuitCount;

        static Application()
        {
            PlayerPrefs.PersistentDataPathResolver = () => persistentDataPath;
        }

        public static IApplicationHost? Host { get; set; }

        public static string? PersistentDataPathOverride
        {
            get => _persistentDataPathOverride;
            set => _persistentDataPathOverride = value;
        }

        public static string? UnityVersionOverride
        {
            get => _unityVersionOverride;
            set => _unityVersionOverride = value;
        }

        public static RuntimePlatform? PlatformOverride
        {
            get => _platformOverride;
            set => _platformOverride = value;
        }

        public static bool IgnoreQuitInPlayMode
        {
            get => _ignoreQuitInPlayMode;
            set => _ignoreQuitInPlayMode = value;
        }

        public static int QuitRequestCount => _quitRequestCount;

        public static int IgnoredPlayModeQuitCount => _ignoredPlayModeQuitCount;

        public static event Func<bool>? wantsToQuit;

        public static event Action? quitting;

        public static event Action<bool>? focusChanged;

        public static event Action<bool>? pauseChanged;

        public static event Action<string>? deepLinkActivated;

        public static bool isPlaying => Host?.IsPlaying ?? false;

        public static bool isEditor => Host != null && Host.BuildKind == ApplicationBuildKind.Editor;

        public static bool isFocused => Host?.IsFocused ?? false;

        public static bool isPaused => Host?.IsPaused ?? false;

        public static bool isBatchMode => true;

        public static RuntimePlatform platform
        {
            get
            {
                if (_platformOverride.HasValue)
                {
                    return _platformOverride.Value;
                }

                bool editor = Host == null || Host.BuildKind == ApplicationBuildKind.Editor;
                if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
                {
                    return editor ? RuntimePlatform.OSXEditor : RuntimePlatform.OSXPlayer;
                }

                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                {
                    return editor ? RuntimePlatform.WindowsEditor : RuntimePlatform.WindowsPlayer;
                }

                return editor ? RuntimePlatform.LinuxEditor : RuntimePlatform.LinuxPlayer;
            }
        }

        public static int targetFrameRate
        {
            get => _targetFrameRate;
            set => _targetFrameRate = value;
        }

        public static bool runInBackground
        {
            get => _runInBackground;
            set => _runInBackground = value;
        }

        public static NetworkReachability internetReachability => NetworkReachability.ReachableViaLocalAreaNetwork;

        public static SystemLanguage systemLanguage => SystemLanguage.English;

        public static bool isMobilePlatform =>
            platform == RuntimePlatform.IPhonePlayer || platform == RuntimePlatform.Android;

        public static string absoluteURL => string.Empty;

        public static ApplicationBuildKind buildKind => Host?.BuildKind ?? ApplicationBuildKind.ReleasePlayer;

        public static bool isDevelopmentBuild
        {
            get
            {
                IApplicationHost? host = Host;
                if (host == null)
                {
                    return false;
                }

                return host.BuildKind == ApplicationBuildKind.Editor
                    || host.BuildKind == ApplicationBuildKind.DevelopmentPlayer;
            }
        }

        public static string companyName
        {
            get
            {
                string? name = Host?.CompanyName;
                return string.IsNullOrEmpty(name) ? DefaultCompanyName : name;
            }
        }

        public static string productName
        {
            get
            {
                string? name = Host?.ProductName;
                return string.IsNullOrEmpty(name) ? DefaultProductName : name;
            }
        }

        public static string version
        {
            get
            {
                string? ver = Host?.Version;
                return string.IsNullOrEmpty(ver) ? DefaultVersion : ver;
            }
        }

        public static string unityVersion
        {
            get
            {
                if (!string.IsNullOrEmpty(_unityVersionOverride))
                {
                    return _unityVersionOverride!;
                }

                string? resolvedFromProject = TryResolveUnityVersionFromProjectSettings();
                if (!string.IsNullOrEmpty(resolvedFromProject))
                {
                    return resolvedFromProject!;
                }

                return DefaultUnityVersion;
            }
        }

        public static string identifier
        {
            get
            {
                string companyToken = NormalizeIdentifierToken(companyName, "defaultcompany");
                string productToken = NormalizeIdentifierToken(productName, "unitasproduct");
                string baseId = "com." + companyToken + "." + productToken;
                IApplicationHost? host = Host;
                if (host == null)
                {
                    return baseId;
                }

                if (host.BuildKind == ApplicationBuildKind.Editor)
                {
                    return baseId + ".editor";
                }

                if (host.BuildKind == ApplicationBuildKind.DevelopmentPlayer)
                {
                    return baseId + ".dev";
                }

                return baseId;
            }
        }

        public static string dataPath
        {
            get
            {
                string? hostPath = Host?.DataPath;
                if (!string.IsNullOrEmpty(hostPath))
                {
                    return hostPath;
                }

                return Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "Assets"));
            }
        }

        public static string streamingAssetsPath
        {
            get
            {
                string? hostPath = Host?.StreamingAssetsPath;
                if (!string.IsNullOrEmpty(hostPath))
                {
                    return hostPath;
                }

                return Path.Combine(dataPath, "StreamingAssets");
            }
        }

        public static string persistentDataPath
        {
            get
            {
                if (!string.IsNullOrEmpty(_persistentDataPathOverride))
                {
                    string overridePath = Path.GetFullPath(_persistentDataPathOverride);
                    Directory.CreateDirectory(overridePath);
                    return overridePath;
                }

                string resolved = ResolvePersistentDataPathForTarget(platform, companyName, productName, ResolveCurrentUserHomeDirectory());
                if (platform != RuntimePlatform.WebGLPlayer)
                {
                    string fullResolved = Path.GetFullPath(resolved);
                    Directory.CreateDirectory(fullResolved);
                    return fullResolved;
                }

                return resolved;
            }
        }

        public static string ResolvePersistentDataPathForTarget(
            RuntimePlatform targetPlatform,
            string? targetCompanyName,
            string? targetProductName,
            string? userHomeDirectory = null)
        {
            string companyToken = SanitizeFolderToken(targetCompanyName ?? string.Empty, DefaultCompanyName);
            string productToken = SanitizeFolderToken(targetProductName ?? string.Empty, DefaultProductName);
            string home = string.IsNullOrEmpty(userHomeDirectory)
                ? ResolveCurrentUserHomeDirectory()
                : userHomeDirectory!.TrimEnd('/', '\\');

            switch (targetPlatform)
            {
                case RuntimePlatform.OSXEditor:
                case RuntimePlatform.OSXPlayer:
                case RuntimePlatform.OSXServer:
                    return CombineUnixPath(home, "Library", "Application Support", companyToken, productToken);

                case RuntimePlatform.WindowsEditor:
                case RuntimePlatform.WindowsPlayer:
                case RuntimePlatform.WindowsServer:
                    return CombineWindowsPath(home, "AppData", "LocalLow", companyToken, productToken);

                case RuntimePlatform.LinuxEditor:
                case RuntimePlatform.LinuxPlayer:
                case RuntimePlatform.LinuxServer:
                    return CombineUnixPath(home, ".config", "unity3d", companyToken, productToken);

                case RuntimePlatform.WebGLPlayer:
                    return "/idbfs/" + companyToken + "/" + productToken;

                case RuntimePlatform.IPhonePlayer:
                case RuntimePlatform.tvOS:
                    return CombineUnixPath(home, "Documents", companyToken, productToken);

                case RuntimePlatform.Android:
                    return "/storage/emulated/0/Android/data/"
                        + "com."
                        + NormalizeIdentifierToken(companyToken, "defaultcompany")
                        + "."
                        + NormalizeIdentifierToken(productToken, "unitasproduct")
                        + "/files";

                default:
                    return CombineUnixPath(home, "unitas", companyToken + "-" + productToken);
            }
        }

        public static string ResolveUnityVersionForProject(string? projectOrDataPath)
        {
            string? version = TryReadUnityVersionFromPath(projectOrDataPath);
            return string.IsNullOrEmpty(version) ? DefaultUnityVersion : version!;
        }

        public static string temporaryCachePath
        {
            get
            {
                string folderName = SanitizeFolderToken(companyName, DefaultCompanyName)
                    + "-"
                    + SanitizeFolderToken(productName, DefaultProductName);
                string resolved = Path.GetFullPath(Path.Combine(Path.GetTempPath(), "unitas-cache", folderName));
                Directory.CreateDirectory(resolved);
                return resolved;
            }
        }

        public static void Quit()
        {
            Quit(0);
        }

        public static void Quit(int exitCode)
        {
            _ = exitCode;
            _quitRequestCount++;
            if (_ignoreQuitInPlayMode && isEditor && isPlaying)
            {
                _ignoredPlayModeQuitCount++;
                Debug.Log("Application.Quit is ignored in the Editor during Play Mode.");
                return;
            }

            if (!EvaluateWantsToQuit())
            {
                return;
            }

            IApplicationHost? host = Host;
            if (host != null)
            {
                host.EndSession();
            }
            else
            {
                RaiseQuitting();
            }
        }

        public static void OpenURL(string url)
        {
            _ = url;
        }

        internal static void RaiseDeepLinkActivated(string url)
        {
            Action<string>? handlers = deepLinkActivated;
            if (handlers == null)
            {
                return;
            }

            try
            {
                handlers.Invoke(url ?? string.Empty);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        public static void ClearEvents()
        {
            wantsToQuit = null;
            quitting = null;
            focusChanged = null;
            pauseChanged = null;
            deepLinkActivated = null;
            _ignoreQuitInPlayMode = false;
            _quitRequestCount = 0;
            _ignoredPlayModeQuitCount = 0;
            _platformOverride = null;
            _unityVersionOverride = null;
        }

        internal static bool EvaluateWantsToQuit()
        {
            Func<bool>? handlers = wantsToQuit;
            if (handlers == null)
            {
                return true;
            }

            Delegate[] invocationList = handlers.GetInvocationList();
            for (int index = 0; index < invocationList.Length; index++)
            {
                if (invocationList[index] is Func<bool> callback)
                {
                    bool allowed;
                    try
                    {
                        allowed = callback();
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex);
                        allowed = true;
                    }

                    if (!allowed)
                    {
                        return false;
                    }
                }
            }

            return true;
        }

        internal static void RaiseQuitting()
        {
            Action? handlers = quitting;
            if (handlers == null)
            {
                return;
            }

            Delegate[] invocationList = handlers.GetInvocationList();
            for (int index = 0; index < invocationList.Length; index++)
            {
                if (invocationList[index] is Action callback)
                {
                    try
                    {
                        callback();
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex);
                    }
                }
            }
        }

        internal static void RaiseFocusChanged(bool hasFocus)
        {
            Action<bool>? handlers = focusChanged;
            if (handlers == null)
            {
                return;
            }

            Delegate[] invocationList = handlers.GetInvocationList();
            for (int index = 0; index < invocationList.Length; index++)
            {
                if (invocationList[index] is Action<bool> callback)
                {
                    try
                    {
                        callback(hasFocus);
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex);
                    }
                }
            }
        }

        internal static void RaisePauseChanged(bool isPaused)
        {
            Action<bool>? handlers = pauseChanged;
            if (handlers == null)
            {
                return;
            }

            Delegate[] invocationList = handlers.GetInvocationList();
            for (int index = 0; index < invocationList.Length; index++)
            {
                if (invocationList[index] is Action<bool> callback)
                {
                    try
                    {
                        callback(isPaused);
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        Debug.LogException(ex);
                    }
                }
            }
        }

        private static string NormalizeIdentifierToken(string value, string fallback)
        {
            if (string.IsNullOrEmpty(value))
            {
                return fallback;
            }

            StringBuilder builder = new StringBuilder(value.Length);
            for (int i = 0; i < value.Length; i++)
            {
                char ch = value[i];
                if (ch >= 'A' && ch <= 'Z')
                {
                    builder.Append((char)(ch - 'A' + 'a'));
                }
                else if ((ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9'))
                {
                    builder.Append(ch);
                }
            }

            return builder.Length == 0 ? fallback : builder.ToString();
        }

        private static string SanitizeFolderToken(string value, string fallback)
        {
            if (string.IsNullOrEmpty(value))
            {
                return fallback;
            }

            char[] invalid = Path.GetInvalidFileNameChars();
            StringBuilder builder = new StringBuilder(value.Length);
            for (int i = 0; i < value.Length; i++)
            {
                char ch = value[i];
                if (Array.IndexOf(invalid, ch) >= 0 || ch == Path.DirectorySeparatorChar || ch == Path.AltDirectorySeparatorChar)
                {
                    builder.Append('_');
                }
                else
                {
                    builder.Append(ch);
                }
            }

            string sanitized = builder.ToString().Trim();
            return sanitized.Length == 0 ? fallback : sanitized;
        }

        private static string ResolveCurrentUserHomeDirectory()
        {
            string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            if (!string.IsNullOrEmpty(home))
            {
                return home.TrimEnd('/', '\\');
            }

            string localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            if (!string.IsNullOrEmpty(localAppData))
            {
                return localAppData.TrimEnd('/', '\\');
            }

            return Path.GetTempPath().TrimEnd('/', '\\');
        }

        private static string CombineUnixPath(string root, params string[] segments)
        {
            StringBuilder builder = new StringBuilder(root.Replace('\\', '/').TrimEnd('/'));
            for (int i = 0; i < segments.Length; i++)
            {
                string segment = segments[i].Replace('\\', '/').Trim('/');
                if (segment.Length > 0)
                {
                    builder.Append('/').Append(segment);
                }
            }

            return builder.ToString();
        }

        private static string CombineWindowsPath(string root, params string[] segments)
        {
            StringBuilder builder = new StringBuilder(root.Replace('/', '\\').TrimEnd('\\'));
            for (int i = 0; i < segments.Length; i++)
            {
                string segment = segments[i].Replace('/', '\\').Trim('\\');
                if (segment.Length > 0)
                {
                    builder.Append('\\').Append(segment);
                }
            }

            return builder.ToString();
        }

        private static string? TryResolveUnityVersionFromProjectSettings()
        {
            string? hostDataPath = Host?.DataPath;
            if (!string.IsNullOrEmpty(hostDataPath))
            {
                string? fromHost = TryReadUnityVersionFromPath(hostDataPath);
                if (!string.IsNullOrEmpty(fromHost))
                {
                    return fromHost;
                }
            }

            return null;
        }

        private static string? TryReadUnityVersionFromPath(string? startPath)
        {
            if (string.IsNullOrEmpty(startPath))
            {
                return null;
            }

            string? current = File.Exists(startPath)
                ? Path.GetDirectoryName(Path.GetFullPath(startPath))
                : Path.GetFullPath(startPath);

            int depth = 0;
            while (!string.IsNullOrEmpty(current) && depth < 6)
            {
                string candidate = Path.Combine(current, "ProjectSettings", "ProjectVersion.txt");
                if (File.Exists(candidate))
                {
                    string? parsed = TryParseEditorVersionFile(candidate);
                    if (!string.IsNullOrEmpty(parsed))
                    {
                        return parsed;
                    }
                }

                current = Path.GetDirectoryName(current);
                depth++;
            }

            return null;
        }

        private static string? TryParseEditorVersionFile(string filePath)
        {
            string[] lines;
            try
            {
                lines = File.ReadAllLines(filePath, Encoding.UTF8);
            }
            catch (IOException)
            {
                return null;
            }
            catch (UnauthorizedAccessException)
            {
                return null;
            }

            for (int i = 0; i < lines.Length; i++)
            {
                string trimmed = lines[i].Trim();
                if (trimmed.StartsWith("m_EditorVersion:", StringComparison.Ordinal))
                {
                    string rawVersion = trimmed.Substring("m_EditorVersion:".Length).Trim();
                    if (rawVersion.StartsWith("2022.3.", StringComparison.Ordinal))
                    {
                        return rawVersion;
                    }

                    if (rawVersion.Length > 0)
                    {
                        return DefaultUnityVersion;
                    }
                }
            }

            return null;
        }
    }
}
