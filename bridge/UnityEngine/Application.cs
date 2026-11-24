using System;
using System.IO;
using System.Text;

namespace UnityEngine
{
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

        public static event Func<bool>? wantsToQuit;

        public static event Action? quitting;

        public static event Action<bool>? focusChanged;

        public static event Action<bool>? pauseChanged;

        public static bool isPlaying => Host?.IsPlaying ?? false;

        public static bool isEditor => Host != null && Host.BuildKind == ApplicationBuildKind.Editor;

        public static bool isFocused => Host?.IsFocused ?? false;

        public static bool isPaused => Host?.IsPaused ?? false;

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

        public static string unityVersion => DefaultUnityVersion;

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

                string baseFolder = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                if (string.IsNullOrEmpty(baseFolder))
                {
                    baseFolder = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
                }

                if (string.IsNullOrEmpty(baseFolder))
                {
                    baseFolder = Path.GetTempPath();
                }

                string folderName = SanitizeFolderToken(companyName, DefaultCompanyName)
                    + "-"
                    + SanitizeFolderToken(productName, DefaultProductName);
                string resolved = Path.GetFullPath(Path.Combine(baseFolder, "unitas", folderName));
                Directory.CreateDirectory(resolved);
                return resolved;
            }
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

        public static void ClearEvents()
        {
            wantsToQuit = null;
            quitting = null;
            focusChanged = null;
            pauseChanged = null;
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
    }
}
