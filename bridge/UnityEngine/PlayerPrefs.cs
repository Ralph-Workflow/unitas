using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace UnityEngine
{
    public sealed class PlayerPrefsException : Exception
    {
        public PlayerPrefsException()
        {
        }

        public PlayerPrefsException(string message) : base(message)
        {
        }

        public PlayerPrefsException(string message, Exception innerException) : base(message, innerException)
        {
        }
    }

    public enum PlayerPrefsLoadState
    {
        Absent = 0,
        Loaded = 1,
        Corrupt = 2,
        IoError = 3
    }

    public enum StorageFailureReason
    {
        None = 0,
        QuotaExceeded = 1,
        PermissionDenied = 2,
        IoError = 3
    }

    public static class PlayerPrefs
    {
        private const string HeaderLine = "UNITAS_PLAYERPREFS_V1";
        private const char TypeInt = 'I';
        private const char TypeFloat = 'F';
        private const char TypeString = 'S';

        private static readonly Dictionary<string, PrefEntry> Cache = new Dictionary<string, PrefEntry>(StringComparer.Ordinal);
        private static bool _isLoaded;
        private static string? _loadedFilePath;
        private static string? _storageDirectoryOverride;
        private static PlayerPrefsLoadState _lastLoadState = PlayerPrefsLoadState.Absent;
        private static string? _lastLoadError;
        private static StorageFailureReason _lastSaveFailureReason = StorageFailureReason.None;
        private static string? _lastSaveError;
        private static long _simulatedQuotaBytes = -1;
        private static int _simulatedInterruptionAfterBytes = -1;

        public static PlayerPrefsLoadState LastLoadState
        {
            get
            {
                EnsureLoaded();
                return _lastLoadState;
            }
        }

        public static bool IsCorrupt => LastLoadState == PlayerPrefsLoadState.Corrupt;

        public static string? LastLoadError
        {
            get
            {
                EnsureLoaded();
                return _lastLoadError;
            }
        }

        public static StorageFailureReason LastSaveFailureReason => _lastSaveFailureReason;

        public static string? LastSaveError => _lastSaveError;

        public static long SimulatedQuotaBytes
        {
            get => _simulatedQuotaBytes;
            set => _simulatedQuotaBytes = value;
        }

        public static int SimulatedInterruptionAfterBytes
        {
            get => _simulatedInterruptionAfterBytes;
            set => _simulatedInterruptionAfterBytes = value;
        }

        public static long CurrentCommittedByteSize
        {
            get
            {
                string path = StorageFilePath;
                if (!File.Exists(path))
                {
                    return 0L;
                }

                return new FileInfo(path).Length;
            }
        }

        public static void ResetSimulationControls()
        {
            _simulatedQuotaBytes = -1;
            _simulatedInterruptionAfterBytes = -1;
            _lastSaveFailureReason = StorageFailureReason.None;
            _lastSaveError = null;
        }

        internal static string? StorageDirectoryOverride
        {
            get => _storageDirectoryOverride;
            set
            {
                if (!string.Equals(_storageDirectoryOverride, value, StringComparison.Ordinal))
                {
                    _storageDirectoryOverride = value;
                    _isLoaded = false;
                }
            }
        }

        internal static string StorageFilePath => Path.Combine(ResolveStorageDirectory(), "PlayerPrefs.ini");

        internal static void Reload()
        {
            _isLoaded = false;
            EnsureLoaded();
        }

        public static int GetInt(string key)
        {
            return GetInt(key, 0);
        }

        public static int GetInt(string key, int defaultValue)
        {
            if (key == null)
            {
                return defaultValue;
            }

            EnsureLoaded();
            if (Cache.TryGetValue(key, out PrefEntry entry) && entry.Kind == TypeInt)
            {
                return entry.IntValue;
            }

            return defaultValue;
        }

        public static void SetInt(string key, int value)
        {
            if (key == null)
            {
                throw new ArgumentNullException(nameof(key));
            }

            EnsureLoaded();
            Cache[key] = PrefEntry.FromInt(value);
        }

        public static float GetFloat(string key)
        {
            return GetFloat(key, 0.0f);
        }

        public static float GetFloat(string key, float defaultValue)
        {
            if (key == null)
            {
                return defaultValue;
            }

            EnsureLoaded();
            if (Cache.TryGetValue(key, out PrefEntry entry) && entry.Kind == TypeFloat)
            {
                return entry.FloatValue;
            }

            return defaultValue;
        }

        public static void SetFloat(string key, float value)
        {
            if (key == null)
            {
                throw new ArgumentNullException(nameof(key));
            }

            EnsureLoaded();
            Cache[key] = PrefEntry.FromFloat(value);
        }

        public static string GetString(string key)
        {
            return GetString(key, string.Empty);
        }

        public static string GetString(string key, string defaultValue)
        {
            if (key == null)
            {
                return defaultValue;
            }

            EnsureLoaded();
            if (Cache.TryGetValue(key, out PrefEntry entry) && entry.Kind == TypeString)
            {
                return entry.StringValue;
            }

            return defaultValue;
        }

        public static void SetString(string key, string value)
        {
            if (key == null)
            {
                throw new ArgumentNullException(nameof(key));
            }

            EnsureLoaded();
            Cache[key] = PrefEntry.FromString(value ?? string.Empty);
        }

        public static bool HasKey(string key)
        {
            if (key == null)
            {
                return false;
            }

            EnsureLoaded();
            return Cache.ContainsKey(key);
        }

        public static void DeleteKey(string key)
        {
            if (key == null)
            {
                return;
            }

            EnsureLoaded();
            Cache.Remove(key);
        }

        public static void DeleteAll()
        {
            EnsureLoaded();
            Cache.Clear();
        }

        public static void Save()
        {
            EnsureLoaded();
            string targetPath = StorageFilePath;
            string directory = Path.GetDirectoryName(targetPath) ?? ResolveStorageDirectory();
            string tempPath = Path.Combine(directory, "PlayerPrefs.ini.tmp." + Guid.NewGuid().ToString("N"));
            bool preserveInterruptedTemp = false;
            try
            {
                Directory.CreateDirectory(directory);
                string serialized = SerializeCache();
                byte[] utf8Bytes = new UTF8Encoding(false).GetBytes(serialized);

                if (_simulatedQuotaBytes >= 0 && utf8Bytes.LongLength > _simulatedQuotaBytes)
                {
                    _lastSaveFailureReason = StorageFailureReason.QuotaExceeded;
                    string quotaMessage = string.Concat(
                        "Could not save PlayerPrefs to '",
                        targetPath,
                        "': storage quota exceeded (required ",
                        utf8Bytes.LongLength.ToString(CultureInfo.InvariantCulture),
                        " bytes, quota ",
                        _simulatedQuotaBytes.ToString(CultureInfo.InvariantCulture),
                        " bytes).");
                    _lastSaveError = quotaMessage;
                    Debug.LogError(quotaMessage);
                    throw new PlayerPrefsException(quotaMessage, new IOException(quotaMessage));
                }

                if (_simulatedInterruptionAfterBytes >= 0 && _simulatedInterruptionAfterBytes < utf8Bytes.Length)
                {
                    preserveInterruptedTemp = true;
                    _lastSaveFailureReason = StorageFailureReason.IoError;
                    byte[] partialBytes = new byte[_simulatedInterruptionAfterBytes];
                    Buffer.BlockCopy(utf8Bytes, 0, partialBytes, 0, _simulatedInterruptionAfterBytes);
                    File.WriteAllBytes(tempPath, partialBytes);
                    string interruptedMessage = string.Concat(
                        "Could not save PlayerPrefs to '",
                        targetPath,
                        "': simulated write interruption after ",
                        _simulatedInterruptionAfterBytes.ToString(CultureInfo.InvariantCulture),
                        " of ",
                        utf8Bytes.Length.ToString(CultureInfo.InvariantCulture),
                        " bytes before atomic replace.");
                    _lastSaveError = interruptedMessage;
                    Debug.LogError(interruptedMessage);
                    throw new PlayerPrefsException(interruptedMessage, new IOException(interruptedMessage));
                }

                File.WriteAllBytes(tempPath, utf8Bytes);
                if (File.Exists(targetPath))
                {
                    File.Replace(tempPath, targetPath, null);
                }
                else
                {
                    File.Move(tempPath, targetPath);
                }
                _loadedFilePath = targetPath;
                _lastLoadState = PlayerPrefsLoadState.Loaded;
                _lastLoadError = null;
                _lastSaveFailureReason = StorageFailureReason.None;
                _lastSaveError = null;
            }
            catch (PlayerPrefsException)
            {
                throw;
            }
            catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException || ex is NotSupportedException)
            {
                _lastSaveFailureReason = ex is UnauthorizedAccessException
                    ? StorageFailureReason.PermissionDenied
                    : StorageFailureReason.IoError;
                string errorMessage = "Could not save PlayerPrefs to '" + targetPath + "': " + ex.Message;
                _lastSaveError = errorMessage;
                Debug.LogError(errorMessage);
                throw new PlayerPrefsException(errorMessage, ex);
            }
            finally
            {
                if (!preserveInterruptedTemp)
                {
                    try
                    {
                        if (File.Exists(tempPath))
                        {
                            File.Delete(tempPath);
                        }
                    }
                    catch (IOException)
                    {
                    }
                    catch (UnauthorizedAccessException)
                    {
                    }
                }
            }
        }

        private static string ResolveStorageDirectory()
        {
            string? armedRoot = Application.PersistentDataPathOverride;
            if (!string.IsNullOrEmpty(armedRoot))
            {
                return Path.Combine(Path.GetFullPath(armedRoot), "prefs");
            }

            if (!string.IsNullOrEmpty(_storageDirectoryOverride))
            {
                return Path.GetFullPath(_storageDirectoryOverride);
            }

            string preferencesRoot = Application.ResolvePlayerPrefsPathForTarget(
                Application.platform,
                Application.companyName,
                Application.productName);
            if (!string.IsNullOrEmpty(preferencesRoot))
            {
                return Path.GetFullPath(preferencesRoot);
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

            return Path.GetFullPath(Path.Combine(baseFolder, "unitas", "DefaultCompany-UnitasProduct"));
        }

        private static void EnsureLoaded()
        {
            string currentPath = StorageFilePath;
            if (_isLoaded && string.Equals(_loadedFilePath, currentPath, StringComparison.Ordinal))
            {
                return;
            }

            Cache.Clear();
            _loadedFilePath = currentPath;
            _isLoaded = true;
            _lastLoadState = PlayerPrefsLoadState.Absent;
            _lastLoadError = null;

            if (!File.Exists(currentPath))
            {
                return;
            }

            string content;
            try
            {
                content = File.ReadAllText(currentPath, Encoding.UTF8);
            }
            catch (IOException ex)
            {
                string msg = "PlayerPrefs failed to read '" + currentPath + "': " + ex.Message;
                _lastLoadState = PlayerPrefsLoadState.IoError;
                _lastLoadError = msg;
                Debug.LogError(msg);
                Debug.LogWarning(msg);
                return;
            }
            catch (UnauthorizedAccessException ex)
            {
                string msg = "PlayerPrefs failed to read '" + currentPath + "': " + ex.Message;
                _lastLoadState = PlayerPrefsLoadState.IoError;
                _lastLoadError = msg;
                Debug.LogError(msg);
                Debug.LogWarning(msg);
                return;
            }

            if (!TryDeserialize(content, out Dictionary<string, PrefEntry> parsed, out string reason))
            {
                string msg = "PlayerPrefs corrupt file '" + currentPath + "': " + reason;
                _lastLoadState = PlayerPrefsLoadState.Corrupt;
                _lastLoadError = msg;
                Debug.LogError(msg);
                Debug.LogWarning(msg);
                return;
            }

            _lastLoadState = PlayerPrefsLoadState.Loaded;
            foreach (KeyValuePair<string, PrefEntry> pair in parsed)
            {
                Cache[pair.Key] = pair.Value;
            }
        }

        private static string SerializeCache()
        {
            List<string> keys = new List<string>(Cache.Keys);
            keys.Sort(StringComparer.Ordinal);

            StringBuilder builder = new StringBuilder();
            builder.Append(HeaderLine).Append('\n');
            foreach (string key in keys)
            {
                PrefEntry entry = Cache[key];
                builder.Append(EscapeField(key)).Append('|').Append(entry.Kind).Append('|');
                if (entry.Kind == TypeInt)
                {
                    builder.Append(entry.IntValue.ToString(CultureInfo.InvariantCulture));
                }
                else if (entry.Kind == TypeFloat)
                {
                    builder.Append(entry.FloatValue.ToString("R", CultureInfo.InvariantCulture));
                }
                else
                {
                    builder.Append(EscapeField(entry.StringValue));
                }

                builder.Append('\n');
            }

            return builder.ToString();
        }

        private static bool TryDeserialize(string content, out Dictionary<string, PrefEntry> entries, out string reason)
        {
            entries = new Dictionary<string, PrefEntry>(StringComparer.Ordinal);
            reason = string.Empty;

            string normalized = content.Replace("\r\n", "\n", StringComparison.Ordinal);
            string[] lines = normalized.Split('\n');
            if (lines.Length == 0 || !string.Equals(lines[0], HeaderLine, StringComparison.Ordinal))
            {
                reason = "missing or invalid header";
                return false;
            }

            for (int i = 1; i < lines.Length; i++)
            {
                string line = lines[i];
                if (line.Length == 0)
                {
                    if (i == lines.Length - 1)
                    {
                        continue;
                    }

                    reason = "unexpected blank record at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                    return false;
                }

                int firstSep = line.IndexOf('|', StringComparison.Ordinal);
                int secondSep = firstSep >= 0 ? line.IndexOf('|', firstSep + 1) : -1;
                if (firstSep < 0 || secondSep < 0 || line.IndexOf('|', secondSep + 1) >= 0)
                {
                    reason = "malformed record delimiter at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                    return false;
                }

                string rawKey = line.Substring(0, firstSep);
                string rawType = line.Substring(firstSep + 1, secondSep - firstSep - 1);
                string rawValue = line.Substring(secondSep + 1);

                if (rawType.Length != 1)
                {
                    reason = "invalid type tag at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                    return false;
                }

                if (!TryUnescapeField(rawKey, out string key))
                {
                    reason = "invalid key escape at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                    return false;
                }

                char kind = rawType[0];
                if (kind == TypeInt)
                {
                    if (!int.TryParse(rawValue, NumberStyles.Integer, CultureInfo.InvariantCulture, out int intVal))
                    {
                        reason = "invalid integer value at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                        return false;
                    }

                    entries[key] = PrefEntry.FromInt(intVal);
                }
                else if (kind == TypeFloat)
                {
                    if (!float.TryParse(rawValue, NumberStyles.Float | NumberStyles.AllowThousands, CultureInfo.InvariantCulture, out float floatVal) || !float.IsFinite(floatVal))
                    {
                        reason = "invalid float value at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                        return false;
                    }

                    entries[key] = PrefEntry.FromFloat(floatVal);
                }
                else if (kind == TypeString)
                {
                    if (!TryUnescapeField(rawValue, out string strVal))
                    {
                        reason = "invalid string escape at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                        return false;
                    }

                    entries[key] = PrefEntry.FromString(strVal);
                }
                else
                {
                    reason = "unknown type tag at line " + (i + 1).ToString(CultureInfo.InvariantCulture);
                    return false;
                }
            }

            return true;
        }

        private static string EscapeField(string input)
        {
            StringBuilder builder = new StringBuilder(input.Length);
            for (int i = 0; i < input.Length; i++)
            {
                char ch = input[i];
                if (ch == '\\')
                {
                    builder.Append("\\\\");
                }
                else if (ch == '\n')
                {
                    builder.Append("\\n");
                }
                else if (ch == '\r')
                {
                    builder.Append("\\r");
                }
                else if (ch == '|')
                {
                    builder.Append("\\p");
                }
                else
                {
                    builder.Append(ch);
                }
            }

            return builder.ToString();
        }

        private static bool TryUnescapeField(string input, out string output)
        {
            StringBuilder builder = new StringBuilder(input.Length);
            for (int i = 0; i < input.Length; i++)
            {
                char ch = input[i];
                if (ch != '\\')
                {
                    builder.Append(ch);
                    continue;
                }

                if (i + 1 >= input.Length)
                {
                    output = string.Empty;
                    return false;
                }

                char next = input[++i];
                if (next == '\\')
                {
                    builder.Append('\\');
                }
                else if (next == 'n')
                {
                    builder.Append('\n');
                }
                else if (next == 'r')
                {
                    builder.Append('\r');
                }
                else if (next == 'p')
                {
                    builder.Append('|');
                }
                else
                {
                    output = string.Empty;
                    return false;
                }
            }

            output = builder.ToString();
            return true;
        }

        private readonly struct PrefEntry
        {
            public PrefEntry(char kind, int intValue, float floatValue, string stringValue)
            {
                Kind = kind;
                IntValue = intValue;
                FloatValue = floatValue;
                StringValue = stringValue;
            }

            public char Kind { get; }

            public int IntValue { get; }

            public float FloatValue { get; }

            public string StringValue { get; }

            public static PrefEntry FromInt(int value)
            {
                return new PrefEntry(TypeInt, value, 0f, string.Empty);
            }

            public static PrefEntry FromFloat(float value)
            {
                return new PrefEntry(TypeFloat, 0, value, string.Empty);
            }

            public static PrefEntry FromString(string value)
            {
                return new PrefEntry(TypeString, 0, 0f, value);
            }
        }
    }
}
