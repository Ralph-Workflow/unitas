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
            TestPlayerPrefsResolvesPerTargetPreferencesLocation();
            TestPreferencesLandUnderArmedOverrideAndStayDisjointFromSaveData();
            TestAtomicWriteThenReplaceLeavesNoTmpAndPreservesLastCompleteSaveAfterCrash();
            TestStory7QuotaExhaustionAndInterruptedSaveFileWriteKeepThePriorValidSavePerTarget();
            TestCorruptPlayerPrefsFileLogsDiagnosticNamingFilePathAndReasonAndFallsBackToDefaults();
            TestDeleteAllAndPersistentDataPathSaveFilesRemainMutuallyIsolated();
            TestDeleteKeyAndDeleteAllLeaveSaveDataUnderTheArmedSaveDirectoryUntouched();
            TestPlayerPrefsSaveFailureReportsReasonAndErrorForQuotaAndInterruption();
            TestArmedPersistentDataPathComposesThePreferencesDirectoryApartFromSaveData();
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
                string expectedIniPath = Path.Combine(Path.GetFullPath(tempDir), "prefs", "PlayerPrefs.ini");
                Check.Equal(
                    "PlayerPrefs per-target storage file resolves to PlayerPrefs.ini under the armed preferences folder",
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

        private static void TestPlayerPrefsResolvesPerTargetPreferencesLocation()
        {
            string unixHome = "/home/builder";
            string windowsHome = "C:\\Users\\Builder";

            Check.Equal(
                "PlayerPrefs resolves the macOS player preferences location to Library/Preferences/unity.<company>.<product>",
                "/home/builder/Library/Preferences/unity.Unitas Studios.Unitas Game",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.OSXPlayer, "Unitas Studios", "Unitas Game", unixHome));
            Check.Equal(
                "PlayerPrefs resolves the macOS editor preferences location to the same macOS location",
                "/home/builder/Library/Preferences/unity.Unitas Studios.Unitas Game",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.OSXEditor, "Unitas Studios", "Unitas Game", unixHome));
            Check.Equal(
                "PlayerPrefs resolves the Windows player preferences location to AppData/LocalLow/<company>/<product>/prefs",
                "C:\\Users\\Builder\\AppData\\LocalLow\\Unitas Studios\\Unitas Game\\prefs",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.WindowsPlayer, "Unitas Studios", "Unitas Game", windowsHome));
            Check.Equal(
                "PlayerPrefs resolves the Windows editor preferences location to the same Windows location",
                "C:\\Users\\Builder\\AppData\\LocalLow\\Unitas Studios\\Unitas Game\\prefs",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.WindowsEditor, "Unitas Studios", "Unitas Game", windowsHome));
            Check.Equal(
                "PlayerPrefs resolves the Linux player preferences location to .config/unity3d/<company>/<product>/prefs",
                "/home/builder/.config/unity3d/Unitas Studios/Unitas Game/prefs",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.LinuxPlayer, "Unitas Studios", "Unitas Game", unixHome));
            Check.Equal(
                "PlayerPrefs resolves the Linux editor preferences location to the same Linux location",
                "/home/builder/.config/unity3d/Unitas Studios/Unitas Game/prefs",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.LinuxEditor, "Unitas Studios", "Unitas Game", unixHome));
            Check.Equal(
                "PlayerPrefs falls back to the default company and product tokens",
                "/home/builder/Library/Preferences/unity.DefaultCompany.UnitasProduct",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.OSXPlayer, null, null, unixHome));
            Check.That(
                "PlayerPrefs resolves against the current user home when no home folder is supplied",
                Application.ResolvePlayerPrefsPathForTarget(RuntimePlatform.OSXPlayer, "Unitas Studios", "Unitas Game")
                    .EndsWith("/Library/Preferences/unity.Unitas Studios.Unitas Game", StringComparison.Ordinal));

            RuntimePlatform? savedPlatform = Application.PlatformOverride;
            string? savedStorageRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            Application.PlatformOverride = RuntimePlatform.WebGLPlayer;
            Application.PersistentDataPathOverride = null;
            PlayerPrefs.StorageDirectoryOverride = null;
            PlayerPrefs.Reload();
            try
            {
                string fallbackPath = Path.GetFullPath(PlayerPrefs.StorageFilePath);
                Check.That(
                    "A target without a file-backed preferences location falls back to the local application data folder",
                    fallbackPath.EndsWith(Path.Combine("unitas", "DefaultCompany-UnitasProduct", "PlayerPrefs.ini"), StringComparison.Ordinal));
            }
            finally
            {
                Application.PlatformOverride = savedPlatform;
                Application.PersistentDataPathOverride = savedAppOverride;
                PlayerPrefs.StorageDirectoryOverride = savedStorageRoot;
                PlayerPrefs.Reload();
            }
        }

        private static void TestPreferencesLandUnderArmedOverrideAndStayDisjointFromSaveData()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            PlayerPrefs.StorageDirectoryOverride = null;
            Application.PersistentDataPathOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string saveRoot = Path.GetFullPath(Application.persistentDataPath);
                string preferencesDirectory = Path.Combine(saveRoot, "prefs");
                string preferencesFile = Path.Combine(preferencesDirectory, "PlayerPrefs.ini");

                Check.Equal(
                    "Preferences land under <armed root>/prefs when persistence is armed",
                    preferencesFile,
                    Path.GetFullPath(PlayerPrefs.StorageFilePath));
                Check.That(
                    "The preferences directory differs from the save-data directory",
                    !string.Equals(preferencesDirectory, saveRoot, StringComparison.Ordinal));
                Check.That(
                    "The preferences file is not the save-data directory's PlayerPrefs.ini",
                    !string.Equals(preferencesFile, Path.Combine(saveRoot, "PlayerPrefs.ini"), StringComparison.Ordinal));

                string savePath = Path.Combine(saveRoot, "campaign_slot_01.json");
                File.WriteAllText(savePath, "{\"slot\":1,\"level\":7}");

                PlayerPrefs.SetInt("master_volume", 42);
                PlayerPrefs.SetString("locale", "en-US");
                PlayerPrefs.Save();

                Check.That("PlayerPrefs.ini is written under <armed root>/prefs", File.Exists(preferencesFile));
                Check.That("No preferences file is written next to the save data", !File.Exists(Path.Combine(saveRoot, "PlayerPrefs.ini")));

                PlayerPrefs.Reload();
                Check.Equal("A saved int survives a fresh load", 42, PlayerPrefs.GetInt("master_volume"));
                Check.Equal("A saved string survives a fresh load", "en-US", PlayerPrefs.GetString("locale"));

                PlayerPrefs.DeleteAll();
                PlayerPrefs.Save();
                PlayerPrefs.Reload();

                Check.Equal("Preferences are cleared after DeleteAll", false, PlayerPrefs.HasKey("master_volume"));
                Check.That("Save data survives PlayerPrefs.DeleteAll", File.Exists(savePath));
                Check.Equal("Save data is unchanged after PlayerPrefs.DeleteAll", "{\"slot\":1,\"level\":7}", File.ReadAllText(savePath));
                Check.That("The preferences file stays under <armed root>/prefs after DeleteAll", File.Exists(preferencesFile));
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

        private static void TestStory7QuotaExhaustionAndInterruptedSaveFileWriteKeepThePriorValidSavePerTarget()
        {
            // The shim simulates the storage outcomes; real device quotas and real device
            // paths belong to the target workstream, so no device quota is asserted here.
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                PlayerPrefs.SetInt("checkpoint_stage", 4);
                PlayerPrefs.SetString("save_owner", "PlayerOne");
                PlayerPrefs.Save();

                string iniPath = PlayerPrefs.StorageFilePath;
                string priorIni = File.ReadAllText(iniPath);
                Check.That("Story 7 a prior valid save exists before the storage outcomes", priorIni.Length > 0);

                PlayerPrefs.SetInt("checkpoint_stage", 9);
                PlayerPrefs.SimulatedQuotaBytes = 1;
                StorageFailureReason quotaReason = StorageFailureReason.None;
                string? quotaError = null;
                bool quotaFailed = false;
                try
                {
                    PlayerPrefs.Save();
                }
                catch (PlayerPrefsException ex)
                {
                    quotaFailed = true;
                    quotaReason = PlayerPrefs.LastSaveFailureReason;
                    quotaError = PlayerPrefs.LastSaveError;
                    Check.That("Story 7 the quota failure names the target file", ex.Message.Contains(iniPath, StringComparison.Ordinal));
                }
                finally
                {
                    PlayerPrefs.ResetSimulationControls();
                }

                Check.That("Story 7 a quota-exhausted PlayerPrefs save fails", quotaFailed);
                Check.Equal("Story 7 the quota-exhausted save reports the QuotaExceeded outcome", (int)StorageFailureReason.QuotaExceeded, (int)quotaReason);
                Check.That("Story 7 the quota failure names the target file and the quota", quotaError != null && quotaError.Contains(iniPath, StringComparison.Ordinal) && quotaError.Contains("quota", StringComparison.Ordinal));
                Check.Equal("Story 7 the quota-exhausted save leaves the prior valid save untouched", priorIni, File.ReadAllText(iniPath));
                Check.Equal("Story 7 the quota-exhausted save leaves no temporary file behind", 0, Directory.GetFiles(tempDir, "*.tmp*").Length);

                PlayerPrefs.Reload();
                Check.Equal("Story 7 the quota-exhausted save does not commit the pending value", 4, PlayerPrefs.GetInt("checkpoint_stage"));
                Check.Equal("Story 7 the prior valid save survives the quota-exhausted save", "PlayerOne", PlayerPrefs.GetString("save_owner"));

                string saveFilePath = Path.Combine(tempDir, "campaign_slot_01.json");
                string priorPayload = "{\"version\":3,\"chapter\":7}";
                File.WriteAllText(saveFilePath, priorPayload);

                SaveMigration.SimulatedQuotaBytes = 8;
                bool fileQuotaFailed = false;
                string? fileQuotaMessage = null;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(saveFilePath, "{\"version\":3,\"chapter\":9}");
                }
                catch (SaveStorageQuotaException ex)
                {
                    fileQuotaFailed = true;
                    fileQuotaMessage = ex.Message;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 a quota-exhausted save file write fails with the quota outcome", fileQuotaFailed);
                Check.That("Story 7 the quota failure names the save file", fileQuotaMessage != null && fileQuotaMessage.Contains(saveFilePath, StringComparison.Ordinal));
                Check.Equal("Story 7 the quota-exhausted save file write leaves the prior valid save untouched", priorPayload, File.ReadAllText(saveFilePath));

                SaveMigration.SimulatedInterruptionAfterBytes = 4;
                bool interrupted = false;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(saveFilePath, "{\"version\":3,\"chapter\":9}");
                }
                catch (SaveWriteInterruptionException)
                {
                    interrupted = true;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 a save file write interrupted by termination fails with the interruption outcome", interrupted);
                Check.Equal("Story 7 the interrupted save file write leaves the prior valid save untouched", priorPayload, File.ReadAllText(saveFilePath));

                IReadOnlyList<PersistenceDurabilityContract> contracts = SaveMigration.GetAllTargetDurabilityContracts();
                Check.That("Story 7 the per-target product knowledge covers the documented targets", contracts.Count >= 4);
                foreach (PersistenceDurabilityContract contract in contracts)
                {
                    string target = contract.Platform.ToString();
                    Check.Equal("Story 7 " + target + " documents that a quota-exhausted write keeps the prior valid save", true, contract.PriorValidSaveSurvivesQuotaExhaustion);
                    Check.Equal("Story 7 " + target + " documents that an interrupted write keeps the prior valid save", true, contract.PriorValidSaveSurvivesInterruption);
                    Check.Equal("Story 7 " + target + " documents that write-then-replace is atomic", true, contract.AtomicWriteThenReplaceSupported);
                }
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

        private static void TestDeleteKeyAndDeleteAllLeaveSaveDataUnderTheArmedSaveDirectoryUntouched()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            PlayerPrefs.StorageDirectoryOverride = null;
            Application.PersistentDataPathOverride = tempDir;
            PlayerPrefs.Reload();
            try
            {
                string savePath = Path.Combine(Path.GetFullPath(Application.persistentDataPath), "campaign_slot_01.json");
                File.WriteAllText(savePath, "{\"version\":2,\"chapter\":5}");
                byte[] savePayloadBytes = File.ReadAllBytes(savePath);

                PlayerPrefs.SetInt("graphics_quality", 3);
                PlayerPrefs.SetString("control_scheme", "gamepad");
                PlayerPrefs.Save();

                PlayerPrefs.DeleteKey("graphics_quality");
                PlayerPrefs.Save();

                Check.Equal("DeleteKey removes the key from PlayerPrefs", false, PlayerPrefs.HasKey("graphics_quality"));
                Check.That("The save file under the armed save-data directory survives PlayerPrefs.DeleteKey", File.Exists(savePath));
                Check.Equal(
                    "The save file bytes under the armed save-data directory are unchanged after PlayerPrefs.DeleteKey",
                    Convert.ToBase64String(savePayloadBytes),
                    Convert.ToBase64String(File.ReadAllBytes(savePath)));

                PlayerPrefs.DeleteAll();
                PlayerPrefs.Save();

                Check.That("The save file under the armed save-data directory survives PlayerPrefs.DeleteAll", File.Exists(savePath));
                Check.Equal(
                    "The save file bytes under the armed save-data directory are unchanged after PlayerPrefs.DeleteAll",
                    Convert.ToBase64String(savePayloadBytes),
                    Convert.ToBase64String(File.ReadAllBytes(savePath)));
            }
            finally
            {
                Application.PersistentDataPathOverride = savedAppOverride;
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestPlayerPrefsSaveFailureReportsReasonAndErrorForQuotaAndInterruption()
        {
            string tempDir = CreateTempRoot();
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            PlayerPrefs.StorageDirectoryOverride = tempDir;
            Application.PersistentDataPathOverride = null;
            PlayerPrefs.Reload();
            try
            {
                string iniPath = PlayerPrefs.StorageFilePath;
                PlayerPrefs.SetInt("checkpoint_stage", 4);
                PlayerPrefs.SetString("save_owner", "PlayerOne");
                PlayerPrefs.Save();

                bool quotaFailed = false;
                try
                {
                    PlayerPrefs.SimulatedQuotaBytes = 1;
                    try
                    {
                        PlayerPrefs.Save();
                    }
                    catch (PlayerPrefsException)
                    {
                        quotaFailed = true;
                    }

                    Check.That("A quota-exhausted PlayerPrefs save fails", quotaFailed);
                    Check.Equal(
                        "A quota-exhausted PlayerPrefs save reports the QuotaExceeded failure reason",
                        (int)StorageFailureReason.QuotaExceeded,
                        (int)PlayerPrefs.LastSaveFailureReason);
                    string? quotaError = PlayerPrefs.LastSaveError;
                    Check.That(
                        "A quota-exhausted PlayerPrefs save reports an error naming the target path",
                        !string.IsNullOrEmpty(quotaError) && quotaError.Contains(iniPath, StringComparison.Ordinal));
                }
                finally
                {
                    PlayerPrefs.ResetSimulationControls();
                }

                bool interrupted = false;
                try
                {
                    PlayerPrefs.SimulatedInterruptionAfterBytes = 4;
                    try
                    {
                        PlayerPrefs.Save();
                    }
                    catch (PlayerPrefsException)
                    {
                        interrupted = true;
                    }

                    Check.That("An interrupted PlayerPrefs save fails", interrupted);
                    Check.Equal(
                        "An interrupted PlayerPrefs save reports the IoError failure reason",
                        (int)StorageFailureReason.IoError,
                        (int)PlayerPrefs.LastSaveFailureReason);
                    string? interruptedError = PlayerPrefs.LastSaveError;
                    Check.That(
                        "An interrupted PlayerPrefs save reports an error naming the target path",
                        !string.IsNullOrEmpty(interruptedError) && interruptedError.Contains(iniPath, StringComparison.Ordinal));
                }
                finally
                {
                    PlayerPrefs.ResetSimulationControls();
                }
            }
            finally
            {
                Application.PersistentDataPathOverride = savedAppOverride;
                PlayerPrefs.StorageDirectoryOverride = savedRoot;
                PlayerPrefs.Reload();
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestArmedPersistentDataPathComposesThePreferencesDirectoryApartFromSaveData()
        {
            string tempDir = CreateTempRoot();
            string seamDirectory = Path.Combine(tempDir, "seam_preferences");
            string? savedRoot = PlayerPrefs.StorageDirectoryOverride;
            string? savedAppOverride = Application.PersistentDataPathOverride;
            Application.PersistentDataPathOverride = tempDir;
            PlayerPrefs.StorageDirectoryOverride = seamDirectory;
            PlayerPrefs.Reload();
            try
            {
                string saveDataDirectory = Path.GetFullPath(Application.persistentDataPath);
                string preferencesDirectory = Path.GetDirectoryName(Path.GetFullPath(PlayerPrefs.StorageFilePath)) ?? string.Empty;

                Check.Equal(
                    "The preferences directory composes as <armed root>/prefs when the persistent data path is armed",
                    Path.Combine(saveDataDirectory, "prefs"),
                    preferencesDirectory);
                Check.That(
                    "The preferences directory differs from the save-data directory",
                    !string.Equals(preferencesDirectory, saveDataDirectory, StringComparison.Ordinal));
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
