using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class PlayerPrefsPersistenceTests
    {
        public static void Run()
        {
            TestPersistenceAcrossSimulatedHostRestartAtPerTargetLocation();
            TestAtomicWriteThenReplaceLeavesNoTmpAndPreservesLastCompleteSaveAfterCrash();
            TestCorruptPlayerPrefsFileLogsDiagnosticNamingFilePathAndReasonAndFallsBackToDefaults();
            TestDeleteAllAndPersistentDataPathSaveFilesRemainMutuallyIsolated();
        }

        public static void RunAll() => Run();

        private static void TestPersistenceAcrossSimulatedHostRestartAtPerTargetLocation()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            PlayerPrefs.StorageDirectoryOverride = null;
            Application.PersistentDataPathOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string expectedIniPath = Path.Combine(Path.GetFullPath(tempDir), "PlayerPrefs.ini");
                Check.Equal(
                    "PlayerPrefs per-target storage file resolves to PlayerPrefs.ini under persistentDataPath",
                    expectedIniPath,
                    Path.GetFullPath(PlayerPrefs.StorageFilePath));
                Check.Equal("Initial load state before first save is Absent", (int)PlayerPrefsLoadState.Absent, (int)PlayerPrefs.LastLoadState);

                PlayerPrefs.SetInt("master_volume", 85);
                PlayerPrefs.SetFloat("mouse_sensitivity", 1.75f);
                PlayerPrefs.SetString("active_profile", "Commander_Shepard");
                PlayerPrefs.Save();

                Check.That("PlayerPrefs.ini exists on disk after Save", File.Exists(expectedIniPath));
                Check.Equal("Load state after Save is Loaded", (int)PlayerPrefsLoadState.Loaded, (int)PlayerPrefs.LastLoadState);

                PlayerPrefs.SetInt("master_volume", 10);
                PlayerPrefs.SetString("uncommitted_flag", "ephemeral");

                PlayerPrefs.Reload();

                Check.Equal("Reload after simulated host restart restores committed int", 85, PlayerPrefs.GetInt("master_volume"));
                Check.Equal("Reload after simulated host restart restores committed float", 1.75f, PlayerPrefs.GetFloat("mouse_sensitivity"));
                Check.Equal("Reload after simulated host restart restores committed string", "Commander_Shepard", PlayerPrefs.GetString("active_profile"));
                Check.Equal("Uncommitted key before simulated host restart is not persisted", false, PlayerPrefs.HasKey("uncommitted_flag"));
                Check.Equal("Load state after simulated host restart is Loaded", (int)PlayerPrefsLoadState.Loaded, (int)PlayerPrefs.LastLoadState);
                Check.Equal("IsCorrupt after clean reload is false", false, PlayerPrefs.IsCorrupt);
            }
            finally
            {
                Application.PersistentDataPathOverride = savedAppOverride;
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestAtomicWriteThenReplaceLeavesNoTmpAndPreservesLastCompleteSaveAfterCrash()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                PlayerPrefs.SetInt("checkpoint_stage", 4);
                PlayerPrefs.SetFloat("brightness", 0.9f);
                PlayerPrefs.SetString("save_owner", "PlayerOne");
                PlayerPrefs.Save();

                string[] leftoverTmpAfterSave = Directory.GetFiles(tempDir, "*.tmp*");
                Check.Equal("Atomic Save leaves zero .tmp files behind", 0, leftoverTmpAfterSave.Length);

                string crashedTmpPath = Path.Combine(tempDir, "PlayerPrefs.ini.tmp.interrupted");
                File.WriteAllText(
                    crashedTmpPath,
                    "UNITAS_PLAYERPREFS_V1\ncheckpoint_stage|I|999\nsave_owner|S|Partial");

                LogRecorder.Clear();
                PlayerPrefs.Reload();

                Check.Equal("Reload with leftover .tmp preserves last complete int value", 4, PlayerPrefs.GetInt("checkpoint_stage"));
                Check.Equal("Reload with leftover .tmp preserves last complete float value", 0.9f, PlayerPrefs.GetFloat("brightness"));
                Check.Equal("Reload with leftover .tmp preserves last complete string value", "PlayerOne", PlayerPrefs.GetString("save_owner"));
                Check.Equal("Reload with leftover .tmp reports Loaded state", (int)PlayerPrefsLoadState.Loaded, (int)PlayerPrefs.LastLoadState);
                Check.Equal("Reload with leftover .tmp logs zero warnings", 0, LogRecorder.OfSeverity(LogSeverity.Warning).Count);
                Check.Equal("Reload with leftover .tmp logs zero errors", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestCorruptPlayerPrefsFileLogsDiagnosticNamingFilePathAndReasonAndFallsBackToDefaults()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string iniPath = PlayerPrefs.StorageFilePath;
                Directory.CreateDirectory(tempDir);
                File.WriteAllText(iniPath, "UNITAS_PLAYERPREFS_V1\nbad_line\n");

                LogRecorder.Clear();
                PlayerPrefs.Reload();

                Check.Equal("Corrupt PlayerPrefs.ini falls back to default int without throwing", 25, PlayerPrefs.GetInt("audio_level", 25));
                Check.Equal("Corrupt PlayerPrefs.ini falls back to default float without throwing", 0.5f, PlayerPrefs.GetFloat("sfx_level", 0.5f));
                Check.Equal("Corrupt PlayerPrefs.ini falls back to default string without throwing", "default_user", PlayerPrefs.GetString("user_id", "default_user"));
                Check.Equal("Corrupt PlayerPrefs.ini HasKey returns false", false, PlayerPrefs.HasKey("audio_level"));
                Check.Equal("Corrupt PlayerPrefs.ini sets LastLoadState to Corrupt", (int)PlayerPrefsLoadState.Corrupt, (int)PlayerPrefs.LastLoadState);
                Check.Equal("Corrupt PlayerPrefs.ini sets IsCorrupt to true", true, PlayerPrefs.IsCorrupt);

                string? loadError = PlayerPrefs.LastLoadError;
                Check.That(
                    "LastLoadError names the corrupt file path and reason",
                    !string.IsNullOrEmpty(loadError)
                    && loadError.Contains(iniPath, StringComparison.Ordinal)
                    && loadError.Contains("malformed record delimiter", StringComparison.Ordinal));

                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Corrupt PlayerPrefs.ini logs warning diagnostic", 1, warnings.Count);
                Check.Equal("Corrupt PlayerPrefs.ini logs error diagnostic", 1, errors.Count);
                Check.That(
                    "Warning diagnostic names corrupt file path and delimiter reason",
                    warnings.Count == 1
                    && warnings[0].Message.Contains(iniPath, StringComparison.Ordinal)
                    && warnings[0].Message.Contains("malformed record delimiter", StringComparison.Ordinal));
                Check.That(
                    "Error diagnostic names corrupt file path and delimiter reason",
                    errors.Count == 1
                    && errors[0].Message.Contains(iniPath, StringComparison.Ordinal)
                    && errors[0].Message.Contains("malformed record delimiter", StringComparison.Ordinal));
            }
            finally
            {
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestDeleteAllAndPersistentDataPathSaveFilesRemainMutuallyIsolated()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            PlayerPrefs.StorageDirectoryOverride = null;
            Application.PersistentDataPathOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string saveSlotPath = Path.Combine(Application.persistentDataPath, "campaign_slot_01.json");
                string saveBackupPath = Path.Combine(Application.persistentDataPath, "saves", "autosave.json");
                Directory.CreateDirectory(Path.Combine(Application.persistentDataPath, "saves"));
                File.WriteAllText(saveSlotPath, "{\"version\":2,\"chapter\":5}");
                File.WriteAllText(saveBackupPath, "{\"version\":2,\"chapter\":4}");

                PlayerPrefs.SetInt("graphics_quality", 3);
                PlayerPrefs.SetString("control_scheme", "gamepad");
                PlayerPrefs.Save();

                PlayerPrefs.DeleteAll();
                PlayerPrefs.Save();
                PlayerPrefs.Reload();

                Check.Equal("PlayerPrefs key graphics_quality is removed after DeleteAll", false, PlayerPrefs.HasKey("graphics_quality"));
                Check.Equal("PlayerPrefs key control_scheme is removed after DeleteAll", false, PlayerPrefs.HasKey("control_scheme"));
                Check.That("Campaign save file under persistentDataPath survives PlayerPrefs.DeleteAll", File.Exists(saveSlotPath));
                Check.Equal("Campaign save file payload is unchanged after PlayerPrefs.DeleteAll", "{\"version\":2,\"chapter\":5}", File.ReadAllText(saveSlotPath));
                Check.That("Autosave file under persistentDataPath survives PlayerPrefs.DeleteAll", File.Exists(saveBackupPath));

                PlayerPrefs.SetInt("graphics_quality", 2);
                PlayerPrefs.SetString("control_scheme", "keyboard_mouse");
                PlayerPrefs.Save();

                File.Delete(saveSlotPath);
                File.Delete(saveBackupPath);
                Check.Equal("Campaign save file was deleted from persistentDataPath", false, File.Exists(saveSlotPath));
                Check.Equal("Autosave file was deleted from persistentDataPath", false, File.Exists(saveBackupPath));

                PlayerPrefs.Reload();
                Check.Equal("Deleting save files under persistentDataPath leaves PlayerPrefs int intact", 2, PlayerPrefs.GetInt("graphics_quality"));
                Check.Equal("Deleting save files under persistentDataPath leaves PlayerPrefs string intact", "keyboard_mouse", PlayerPrefs.GetString("control_scheme"));
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
            string path = Path.Combine(Path.GetTempPath(), "unitas_prefs_test_" + Guid.NewGuid().ToString("N"));
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
