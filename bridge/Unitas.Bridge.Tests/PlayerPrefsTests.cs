using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class PlayerPrefsTests
    {
        public static void Run()
        {
            TestCrudIntFloatStringAndDefaults();
            TestTypeMismatchReturnsSuppliedDefault();
            TestSpecialCharactersInKeysAndValuesRoundTrip();
            TestSaveAndReloadAcrossProcessRestartSimulation();
            TestCorruptFileLogsWarningReturnsDefaultsAndSaveRecovers();
            TestSaveFailureThrowsPlayerPrefsExceptionAndPreservesPriorValidFile();
            TestStoragePathUnderPersistentDataPathAndNotProjectContent();
        }

        private static void TestCrudIntFloatStringAndDefaults()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            LogRecorder.Clear();
            try
            {
                Check.Equal("Absent HasKey is false", false, PlayerPrefs.HasKey("volume"));
                Check.Equal("Absent GetInt default is 0", 0, PlayerPrefs.GetInt("score"));
                Check.Equal("Absent GetInt supplied default", 42, PlayerPrefs.GetInt("score", 42));
                Check.Equal("Absent GetFloat default is 0", 0f, PlayerPrefs.GetFloat("volume"));
                Check.Equal("Absent GetFloat supplied default", 0.75f, PlayerPrefs.GetFloat("volume", 0.75f));
                Check.Equal("Absent GetString default is empty", string.Empty, PlayerPrefs.GetString("player"));
                Check.Equal("Absent GetString supplied default", "Guest", PlayerPrefs.GetString("player", "Guest"));
                Check.Equal("Absent file logs zero warnings", 0, LogRecorder.OfSeverity(LogSeverity.Warning).Count);

                PlayerPrefs.SetInt("score", 1200);
                PlayerPrefs.SetFloat("volume", 0.65f);
                PlayerPrefs.SetString("player", "Aria");

                Check.Equal("HasKey score is true after SetInt", true, PlayerPrefs.HasKey("score"));
                Check.Equal("HasKey volume is true after SetFloat", true, PlayerPrefs.HasKey("volume"));
                Check.Equal("HasKey player is true after SetString", true, PlayerPrefs.HasKey("player"));
                Check.Equal("GetInt reads stored int", 1200, PlayerPrefs.GetInt("score"));
                Check.Equal("GetFloat reads stored float", 0.65f, PlayerPrefs.GetFloat("volume"));
                Check.Equal("GetString reads stored string", "Aria", PlayerPrefs.GetString("player"));

                PlayerPrefs.SetInt("score", -99);
                Check.Equal("SetInt updates existing key", -99, PlayerPrefs.GetInt("score"));

                PlayerPrefs.DeleteKey("volume");
                Check.Equal("HasKey volume is false after DeleteKey", false, PlayerPrefs.HasKey("volume"));
                Check.Equal("GetFloat returns default after DeleteKey", 0.5f, PlayerPrefs.GetFloat("volume", 0.5f));
                Check.Equal("Other keys remain after DeleteKey", -99, PlayerPrefs.GetInt("score"));

                PlayerPrefs.DeleteAll();
                Check.Equal("HasKey score is false after DeleteAll", false, PlayerPrefs.HasKey("score"));
                Check.Equal("HasKey player is false after DeleteAll", false, PlayerPrefs.HasKey("player"));
                Check.Equal("GetInt returns default after DeleteAll", 0, PlayerPrefs.GetInt("score"));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestTypeMismatchReturnsSuppliedDefault()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                PlayerPrefs.SetInt("int_key", 7);
                PlayerPrefs.SetFloat("float_key", 2.5f);
                PlayerPrefs.SetString("string_key", "123");

                Check.Equal("GetFloat on int key returns default", 1.5f, PlayerPrefs.GetFloat("int_key", 1.5f));
                Check.Equal("GetString on int key returns default", "fallback", PlayerPrefs.GetString("int_key", "fallback"));
                Check.Equal("GetInt on float key returns default", 99, PlayerPrefs.GetInt("float_key", 99));
                Check.Equal("GetString on float key returns default", "none", PlayerPrefs.GetString("float_key", "none"));
                Check.Equal("GetInt on string key returns default", 11, PlayerPrefs.GetInt("string_key", 11));
                Check.Equal("GetFloat on string key returns default", 3.25f, PlayerPrefs.GetFloat("string_key", 3.25f));

                PlayerPrefs.SetString("int_key", "converted");
                Check.Equal("Overwriting int key with string updates type", "converted", PlayerPrefs.GetString("int_key"));
                Check.Equal("GetInt on overwritten key returns default", 0, PlayerPrefs.GetInt("int_key"));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestSpecialCharactersInKeysAndValuesRoundTrip()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string complexKey = "section|sub\\key\nline2\r\t=";
                string complexVal = "line1\nline2\r\npipe|backslash\\empty:";
                PlayerPrefs.SetString(complexKey, complexVal);
                PlayerPrefs.SetString("empty_val", string.Empty);
                PlayerPrefs.Save();

                PlayerPrefs.Reload();
                Check.Equal("Complex key exists after Save and Reload", true, PlayerPrefs.HasKey(complexKey));
                Check.Equal("Complex string value round-trips", complexVal, PlayerPrefs.GetString(complexKey, "wrong"));
                Check.Equal("Empty string value exists after Save and Reload", true, PlayerPrefs.HasKey("empty_val"));
                Check.Equal("Empty string value round-trips", string.Empty, PlayerPrefs.GetString("empty_val", "nonempty"));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestSaveAndReloadAcrossProcessRestartSimulation()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                PlayerPrefs.SetInt("difficulty", 3);
                PlayerPrefs.SetFloat("gamma", 1.2f);
                PlayerPrefs.SetString("locale", "ja-JP");
                PlayerPrefs.Save();

                string filePath = PlayerPrefs.StorageFilePath;
                Check.That("Save writes the PlayerPrefs storage file", File.Exists(filePath));

                PlayerPrefs.SetInt("unsaved", 555);
                PlayerPrefs.Reload();

                Check.Equal("Saved int survives Reload", 3, PlayerPrefs.GetInt("difficulty"));
                Check.Equal("Saved float survives Reload", 1.2f, PlayerPrefs.GetFloat("gamma"));
                Check.Equal("Saved string survives Reload", "ja-JP", PlayerPrefs.GetString("locale"));
                Check.Equal("Unsaved mutation is discarded on Reload", false, PlayerPrefs.HasKey("unsaved"));

                PlayerPrefs.DeleteKey("gamma");
                PlayerPrefs.Save();
                PlayerPrefs.Reload();
                Check.Equal("Deleted key stays deleted after Save and Reload", false, PlayerPrefs.HasKey("gamma"));
                Check.Equal("Remaining key stays intact after Save and Reload", 3, PlayerPrefs.GetInt("difficulty"));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestCorruptFileLogsWarningReturnsDefaultsAndSaveRecovers()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string filePath = PlayerPrefs.StorageFilePath;
                string? parent = Path.GetDirectoryName(filePath);
                if (!string.IsNullOrEmpty(parent))
                {
                    Directory.CreateDirectory(parent);
                }

                File.WriteAllText(filePath, "CORRUPT_GARBAGE_HEADER\nnot|a|valid|line\n");
                LogRecorder.Clear();
                PlayerPrefs.Reload();

                Check.Equal("Corrupt file reads return default int", 10, PlayerPrefs.GetInt("any", 10));
                Check.Equal("Corrupt file HasKey is false", false, PlayerPrefs.HasKey("any"));
                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                Check.Equal("Corrupt file logs one warning", 1, warnings.Count);
                Check.That(
                    "Corrupt file warning mentions file path",
                    warnings.Count == 1 && warnings[0].Message.Contains(filePath, StringComparison.Ordinal));

                File.WriteAllText(filePath, "UNITAS_PLAYERPREFS_V1\nkey|I|not_an_int\n");
                LogRecorder.Clear();
                PlayerPrefs.Reload();

                Check.Equal("Corrupt int record returns default", 0, PlayerPrefs.GetInt("key"));
                Check.Equal("Corrupt int record logs one warning", 1, LogRecorder.OfSeverity(LogSeverity.Warning).Count);

                PlayerPrefs.SetInt("recovered", 88);
                PlayerPrefs.Save();
                LogRecorder.Clear();
                PlayerPrefs.Reload();

                Check.Equal("Recovered file logs zero warnings", 0, LogRecorder.OfSeverity(LogSeverity.Warning).Count);
                Check.Equal("Recovered file reads saved value", 88, PlayerPrefs.GetInt("recovered"));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestSaveFailureThrowsPlayerPrefsExceptionAndPreservesPriorValidFile()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                PlayerPrefs.SetInt("checkpoint", 10);
                PlayerPrefs.Save();

                string validFile = PlayerPrefs.StorageFilePath;
                Check.That("Prior valid PlayerPrefs file exists", File.Exists(validFile));

                string blockedRootFile = Path.Combine(tempDir, "blocked_not_a_directory");
                File.WriteAllText(blockedRootFile, "occupied by file");

                PlayerPrefs.StorageDirectoryOverride = blockedRootFile;
                PlayerPrefs.SetInt("checkpoint", 99);

                bool threwPrefsException = false;
                try
                {
                    PlayerPrefs.Save();
                }
                catch (PlayerPrefsException ex)
                {
                    threwPrefsException = !string.IsNullOrEmpty(ex.Message) && ex.InnerException != null;
                }

                Check.That("Save into an invalid directory throws PlayerPrefsException", threwPrefsException);

                PlayerPrefs.StorageDirectoryOverride = tempDir;
                PlayerPrefs.Reload();
                Check.Equal("Prior valid PlayerPrefs file remains intact after failed save", 10, PlayerPrefs.GetInt("checkpoint"));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStoragePathUnderPersistentDataPathAndNotProjectContent()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            PlayerPrefs.StorageDirectoryOverride = null;
            Application.PersistentDataPathOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string filePath = Path.GetFullPath(PlayerPrefs.StorageFilePath);
                string persistentPath = Path.GetFullPath(Application.persistentDataPath);
                string dataPath = Path.GetFullPath(Application.dataPath);
                string repoRoot = Path.GetFullPath(".");
                Check.That("Default StorageFilePath is rooted", Path.IsPathRooted(filePath));
                Check.Equal(
                    "Default StorageFilePath lives under Application.persistentDataPath",
                    Path.Combine(persistentPath, "PlayerPrefs.ini"),
                    filePath);
                Check.That(
                    "Default StorageFilePath is not under Application.dataPath",
                    !filePath.StartsWith(dataPath + Path.DirectorySeparatorChar, StringComparison.Ordinal));
                Check.That(
                    "Default StorageFilePath is outside repository project content",
                    !filePath.StartsWith(repoRoot + Path.DirectorySeparatorChar, StringComparison.Ordinal));
            }
            finally
            {
                Application.PersistentDataPathOverride = savedAppOverride;
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static string CreateTempRoot()
        {
            string path = Path.Combine(Path.GetTempPath(), "unitas_playerprefs_test_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(path);
            return path;
        }

        private static void CleanupTempRoot(string path)
        {
            if (Directory.Exists(path))
            {
                Directory.Delete(path, true);
            }
        }
    }
}
