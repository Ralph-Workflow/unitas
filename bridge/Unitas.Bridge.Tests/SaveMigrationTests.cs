using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class SaveMigrationTests
    {
        private const string ProjectCompanyName = "Unitas Studios";
        private const string ProjectProductName = "Unitas Game";

        public static void Run()
        {
            TestStory8SaveMigrationReportsInputVersionResultingVersionAndDistinctOutcomes();
            TestStory8AbsentSaveFileReportsTheAbsentOutcome();
            TestStory8MalformedSaveFileReportsTheCorruptOutcome();
            TestStory8MigratedSaveFileReportsInputVersionResultingVersionAndAppliedSteps();
            TestStory7InterruptedSaveFileWriteKeepsThePriorValidSaveByteIdentical();
            TestStory7QuotaExhaustedSaveFileWriteKeepsThePriorValidSaveAndLeavesNoStagingFile();
            TestStory7BinarySaveFilePayloadRoundTripsByteIdentical();
            TestStory7SaveWriteFailuresReportTheReasonAndKeepThePriorValidSaveIntact();
            TestStory7SuccessiveWritesNeverRemoveTheTargetFileBeforeTheNewContentIsInPlace();
            TestStory11ProductKnowledgeStatesPerTargetDurabilityAndCrashLossWindow();
            TestStory26PersistentDataPathResolvesPerTargetToUnityCompanyProductLocation();
            TestStory26PlayerPrefsStoragePrefersTheArmedPersistentDataPathOverTheInternalSeam();
            TestStory35ApplicationReportsProject2022_3VersionAndIgnoresQuitDuringPlay();
        }

        public static void RunAll() => Run();

        private static void TestStory8SaveMigrationReportsInputVersionResultingVersionAndDistinctOutcomes()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");

                SaveMigrationResult absent = SaveMigration.MigrateFile(savePath, 3, CreateSteps());
                Check.Equal("Story 8 an absent save reports the Absent outcome", (int)SaveMigrationOutcome.Absent, (int)absent.Outcome);
                Check.Equal("Story 8 an absent save reports input version 0", 0, absent.InputVersion);
                Check.Equal("Story 8 an absent save reports resulting version 0", 0, absent.ResultingVersion);

                File.WriteAllText(savePath, "{\"version\":3,\"chapter\":5}");
                SaveMigrationResult current = SaveMigration.MigrateFile(savePath, 3, CreateSteps());
                Check.Equal("Story 8 a save at the current version reports the AlreadyCurrent outcome", (int)SaveMigrationOutcome.AlreadyCurrent, (int)current.Outcome);
                Check.Equal("Story 8 a save at the current version reports input version 3", 3, current.InputVersion);
                Check.Equal("Story 8 a save at the current version reports resulting version 3", 3, current.ResultingVersion);
                Check.Equal("Story 8 a save at the current version is left byte-identical", "{\"version\":3,\"chapter\":5}", File.ReadAllText(savePath));

                File.WriteAllText(savePath, "{\"version\":1,\"chapter\":5}");
                SaveMigrationResult migrated = SaveMigration.MigrateFile(savePath, 3, CreateSteps());
                Check.Equal("Story 8 an accepted older save reports the Migrated outcome", (int)SaveMigrationOutcome.Migrated, (int)migrated.Outcome);
                Check.Equal("Story 8 an accepted older save reports input version 1", 1, migrated.InputVersion);
                Check.Equal("Story 8 an accepted older save reports resulting version 3", 3, migrated.ResultingVersion);
                Check.Equal("Story 8 an accepted older save applies one step per version", 2, migrated.AppliedSteps.Count);
                Check.Equal("Story 8 an accepted older save is rewritten at the current version", "{\"version\":3,\"chapter\":7}", File.ReadAllText(savePath));
                Check.That("Story 8 an accepted older save reports the data it could not retain", migrated.UnretainedDataNotes.Count == 2);

                File.WriteAllText(savePath, "{\"chapter\":5");
                SaveMigrationResult corrupt = SaveMigration.MigrateFile(savePath, 3, CreateSteps());
                Check.Equal("Story 8 a corrupt save reports the Corrupt outcome", (int)SaveMigrationOutcome.Corrupt, (int)corrupt.Outcome);
                Check.Equal("Story 8 a corrupt save reports input version 0", 0, corrupt.InputVersion);
                Check.Equal("Story 8 a corrupt save leaves the prior file untouched", "{\"chapter\":5", File.ReadAllText(savePath));

                File.WriteAllText(savePath, "{\"version\":9,\"chapter\":5}");
                SaveMigrationResult incompatible = SaveMigration.MigrateFile(savePath, 3, CreateSteps());
                Check.Equal("Story 8 a newer save reports the IncompatibleNewerVersion outcome", (int)SaveMigrationOutcome.IncompatibleNewerVersion, (int)incompatible.Outcome);
                Check.Equal("Story 8 a newer save reports input version 9", 9, incompatible.InputVersion);
                Check.Equal("Story 8 a newer save reports resulting version 9", 9, incompatible.ResultingVersion);
                Check.Equal("Story 8 a newer save leaves the prior file untouched", "{\"version\":9,\"chapter\":5}", File.ReadAllText(savePath));

                File.WriteAllText(savePath, "{\"version\":1,\"chapter\":5}");
                SaveMigrationResult refused = SaveMigration.MigrateFile(savePath, 3, CreateRefusingSteps());
                Check.Equal("Story 8 a refused migration step reports the MigrationFailed outcome", (int)SaveMigrationOutcome.MigrationFailed, (int)refused.Outcome);
                Check.Equal("Story 8 a refused migration step reports input version 1", 1, refused.InputVersion);
                Check.Equal("Story 8 a refused migration step reports resulting version 1", 1, refused.ResultingVersion);
                Check.Equal("Story 8 a refused migration step leaves the prior file untouched", "{\"version\":1,\"chapter\":5}", File.ReadAllText(savePath));

                File.WriteAllText(savePath, "{\"version\":1,\"chapter\":5}");
                SaveMigration.SimulatedQuotaBytes = 8;
                SaveMigrationResult writeFailed;
                try
                {
                    writeFailed = SaveMigration.MigrateFile(savePath, 3, CreateSteps());
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.Equal("Story 8 a failed atomic write reports the MigrationFailed outcome", (int)SaveMigrationOutcome.MigrationFailed, (int)writeFailed.Outcome);
                Check.Equal("Story 8 a failed atomic write reports input version 1", 1, writeFailed.InputVersion);
                Check.Equal("Story 8 a failed atomic write leaves the prior file untouched", "{\"version\":1,\"chapter\":5}", File.ReadAllText(savePath));

                Check.That(
                    "Story 8 absent, corrupt, incompatible and failed outcomes stay distinct",
                    (int)SaveMigrationOutcome.Absent != (int)SaveMigrationOutcome.Corrupt
                    && (int)SaveMigrationOutcome.Absent != (int)SaveMigrationOutcome.IncompatibleNewerVersion
                    && (int)SaveMigrationOutcome.Absent != (int)SaveMigrationOutcome.MigrationFailed
                    && (int)SaveMigrationOutcome.Corrupt != (int)SaveMigrationOutcome.IncompatibleNewerVersion
                    && (int)SaveMigrationOutcome.Corrupt != (int)SaveMigrationOutcome.MigrationFailed
                    && (int)SaveMigrationOutcome.IncompatibleNewerVersion != (int)SaveMigrationOutcome.MigrationFailed);
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory8AbsentSaveFileReportsTheAbsentOutcome()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "missing.json");

                SaveMigrationResult absent = SaveMigration.MigrateFile(savePath, 3, CreateSteps());

                Check.Equal("Story 8 an absent save file reports the Absent outcome", (int)SaveMigrationOutcome.Absent, (int)absent.Outcome);
                Check.Equal("Story 8 an absent save file reports the path it looked at", Path.GetFullPath(savePath), absent.FilePath);
                Check.That("Story 8 an absent save file reports a diagnostic naming the save file", absent.DiagnosticMessage.Contains("missing.json", StringComparison.Ordinal));
                Check.That("Story 8 an absent save file leaves no file behind", !File.Exists(savePath));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory8MalformedSaveFileReportsTheCorruptOutcome()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");
                const string malformed = "chapter 5 with no version field";
                File.WriteAllText(savePath, malformed);
                byte[] priorBytes = File.ReadAllBytes(savePath);

                SaveMigrationResult corrupt = SaveMigration.MigrateFile(savePath, 3, CreateSteps());

                Check.Equal("Story 8 a malformed save file reports the Corrupt outcome", (int)SaveMigrationOutcome.Corrupt, (int)corrupt.Outcome);
                Check.Equal("Story 8 a malformed save file reports input version 0", 0, corrupt.InputVersion);
                Check.Equal("Story 8 a malformed save file reports no applied step", 0, corrupt.AppliedSteps.Count);
                Check.That("Story 8 a malformed save file reports the prior file as preserved", corrupt.PriorFilePreserved);
                Check.That("Story 8 a malformed save file leaves the prior file byte-identical", priorBytes.SequenceEqual(File.ReadAllBytes(savePath)));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory8MigratedSaveFileReportsInputVersionResultingVersionAndAppliedSteps()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");
                File.WriteAllText(savePath, "{\"version\":1,\"chapter\":5}");

                SaveMigrationResult migrated = SaveMigration.MigrateFile(savePath, 2, CreateSteps());

                Check.Equal("Story 8 a migrated save file reports the Migrated outcome", (int)SaveMigrationOutcome.Migrated, (int)migrated.Outcome);
                Check.Equal("Story 8 a migrated save file reports its input version", 1, migrated.InputVersion);
                Check.Equal("Story 8 a migrated save file reports its resulting version", 2, migrated.ResultingVersion);
                Check.That("Story 8 a migrated save file reports the applied steps", migrated.AppliedSteps.Count > 0);
                Check.Equal("Story 8 a migrated save file applies one step per version", 1, migrated.AppliedSteps.Count);
                Check.Equal("Story 8 a migrated save file reports the version its applied step produced", 2, migrated.AppliedSteps[0]);
                Check.Equal("Story 8 a migrated save file is rewritten at the resulting version", "{\"version\":2,\"chapter\":6}", File.ReadAllText(savePath));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory7InterruptedSaveFileWriteKeepsThePriorValidSaveByteIdentical()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");
                SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":1,\"chapter\":5}");
                byte[] priorBytes = File.ReadAllBytes(savePath);

                SaveMigration.SimulatedInterruptionAfterBytes = 4;
                Exception? failure = null;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":2,\"chapter\":9}");
                }
                catch (IOException ex)
                {
                    failure = ex;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 an interrupted save file write reports the interruption to the caller", failure is SaveWriteInterruptionException);
                Check.That("Story 7 an interrupted save file write reports no quota failure", !(failure is SaveStorageQuotaException));
                Check.That("Story 7 an interrupted save file write leaves the prior valid save byte-identical", priorBytes.SequenceEqual(File.ReadAllBytes(savePath)));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory7QuotaExhaustedSaveFileWriteKeepsThePriorValidSaveAndLeavesNoStagingFile()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");
                SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":1,\"chapter\":5}");
                byte[] priorBytes = File.ReadAllBytes(savePath);

                SaveMigration.SimulatedQuotaBytes = 8;
                Exception? failure = null;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":2,\"chapter\":9}");
                }
                catch (IOException ex)
                {
                    failure = ex;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 a quota exhausted save file write reports the quota refusal to the caller", failure is SaveStorageQuotaException);
                Check.That("Story 7 a quota exhausted save file write reports no interruption", !(failure is SaveWriteInterruptionException));
                Check.That("Story 7 a quota exhausted save file write leaves the prior valid save byte-identical", priorBytes.SequenceEqual(File.ReadAllBytes(savePath)));
                Check.Equal("Story 7 a quota exhausted save file write leaves the target file as the only file in its directory", 1, Directory.GetFiles(tempDir).Length);
                Check.Equal("Story 7 a quota exhausted save file write leaves no staging file behind", 0, Directory.GetFiles(tempDir, "*.tmp*").Length);
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory7BinarySaveFilePayloadRoundTripsByteIdentical()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.bin");
                byte[] payload = { 0x00, 0x01, 0x7F, 0x80, 0xFF, 0x00, 0x0A, 0x0D, 0x1A };
                SaveMigration.WriteSaveFileAtomically(savePath, payload);

                Check.That("Story 7 a binary save file payload round-trips byte-identical", payload.SequenceEqual(File.ReadAllBytes(savePath)));

                byte[] priorBytes = File.ReadAllBytes(savePath);
                SaveMigration.SimulatedInterruptionAfterBytes = 3;
                Exception? failure = null;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(savePath, new byte[] { 0xFF, 0x00, 0xFF, 0x00, 0x01 });
                }
                catch (IOException ex)
                {
                    failure = ex;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 a failed binary save file write reports the interruption to the caller", failure is SaveWriteInterruptionException);
                Check.That("Story 7 a failed binary save file write leaves the prior save byte-identical", priorBytes.SequenceEqual(File.ReadAllBytes(savePath)));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory7SaveWriteFailuresReportTheReasonAndKeepThePriorValidSaveIntact()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");
                SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":1,\"chapter\":5}");
                byte[] priorBytes = File.ReadAllBytes(savePath);
                string fullTargetPath = Path.GetFullPath(savePath);

                bool quotaRefused = false;
                StorageFailureReason quotaReason = StorageFailureReason.None;
                string? quotaError = null;
                SaveMigration.SimulatedQuotaBytes = 8;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":2,\"chapter\":9}");
                }
                catch (SaveStorageQuotaException)
                {
                    quotaRefused = true;
                    quotaReason = SaveMigration.LastWriteFailureReason;
                    quotaError = SaveMigration.LastWriteError;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 a quota exhausted save file write reaches the caller as a quota refusal", quotaRefused);
                Check.That("Story 7 a quota exhausted save file write leaves the prior valid save byte-identical", priorBytes.SequenceEqual(File.ReadAllBytes(savePath)));
                Check.Equal("Story 7 a quota exhausted save file write leaves no staging file behind", 0, Directory.GetFiles(tempDir, "*.tmp*").Length);
                Check.Equal("Story 7 a quota exhausted save file write reports the quota failure reason", (int)StorageFailureReason.QuotaExceeded, (int)quotaReason);
                Check.That("Story 7 a quota exhausted save file write reports an error naming the target file", quotaError != null && quotaError.Length > 0 && quotaError.Contains(fullTargetPath, StringComparison.Ordinal));

                bool interrupted = false;
                StorageFailureReason interruptionReason = StorageFailureReason.None;
                string? interruptionError = null;
                SaveMigration.SimulatedInterruptionAfterBytes = 4;
                try
                {
                    SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":2,\"chapter\":9}");
                }
                catch (SaveWriteInterruptionException)
                {
                    interrupted = true;
                    interruptionReason = SaveMigration.LastWriteFailureReason;
                    interruptionError = SaveMigration.LastWriteError;
                }
                finally
                {
                    SaveMigration.ResetSimulationControls();
                }

                Check.That("Story 7 an interrupted save file write reaches the caller as an interruption", interrupted);
                Check.That("Story 7 an interrupted save file write leaves the prior valid save byte-identical", priorBytes.SequenceEqual(File.ReadAllBytes(savePath)));
                Check.Equal("Story 7 an interrupted save file write reports the io failure reason", (int)StorageFailureReason.IoError, (int)interruptionReason);
                Check.That("Story 7 an interrupted save file write reports an error naming the target file", interruptionError != null && interruptionError.Length > 0 && interruptionError.Contains(fullTargetPath, StringComparison.Ordinal));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory7SuccessiveWritesNeverRemoveTheTargetFileBeforeTheNewContentIsInPlace()
        {
            string tempDir = CreateTempRoot();
            try
            {
                string savePath = Path.Combine(tempDir, "campaign.json");
                SaveMigration.WriteSaveFileAtomically(savePath, "{\"version\":1,\"chapter\":5}");

                const int writeCount = 32;
                int missingObservations = 0;
                long observations = 0;
                bool stop = false;
                Thread poller = new Thread(() =>
                {
                    while (!Volatile.Read(ref stop))
                    {
                        Interlocked.Increment(ref observations);
                        if (!File.Exists(savePath))
                        {
                            Interlocked.Increment(ref missingObservations);
                        }
                    }
                });
                poller.IsBackground = true;
                poller.Start();

                try
                {
                    for (int i = 0; i < writeCount; i++)
                    {
                        SaveMigration.WriteSaveFileAtomically(savePath, "{\"chapter\":9,\"revision\":\"" + new string('x', i + 1) + "\"}");
                    }
                }
                finally
                {
                    Volatile.Write(ref stop, true);
                    poller.Join();
                }

                Check.That("Story 7 the replace safety observation polled the target file", observations > 0);
                Check.Equal("Story 7 successive writes never leave the target file missing", 0, missingObservations);
                Check.Equal(
                    "Story 7 the last successive write lands in the target file",
                    "{\"chapter\":9,\"revision\":\"" + new string('x', writeCount) + "\"}",
                    File.ReadAllText(savePath));
            }
            finally
            {
                CleanupTempRoot(tempDir);
            }
        }

        private static void TestStory11ProductKnowledgeStatesPerTargetDurabilityAndCrashLossWindow()
        {
            IReadOnlyList<PersistenceDurabilityContract> contracts = SaveMigration.GetAllTargetDurabilityContracts();
            Check.That("Story 11 product knowledge covers the desktop, browser and mobile targets", contracts.Count >= 4);

            foreach (PersistenceDurabilityContract contract in contracts)
            {
                string target = contract.Platform.ToString();
                Check.Equal("Story 11 " + target + " product knowledge is identified as SAV-001", "SAV-001", contract.FeatureId);
                Check.That(
                    "Story 11 " + target + " states when PlayerPrefs.Save becomes durable",
                    contract.PlayerPrefsSaveDurabilityPoint.Contains("PlayerPrefs.Save", StringComparison.Ordinal));
                Check.That(
                    "Story 11 " + target + " states when persistent-data file writes become durable",
                    contract.PersistentDataFileWriteDurabilityPoint.Contains("Application.persistentDataPath", StringComparison.Ordinal));
                Check.That(
                    "Story 11 " + target + " states what a crash loses",
                    contract.WhatCrashLosesBeforeSaveOrFlush.Contains("PlayerPrefs", StringComparison.Ordinal));
                Check.Equal("Story 11 " + target + " states that unsaved PlayerPrefs are lost on a crash", true, contract.UnsavedPlayerPrefsLostOnCrash);
                Check.Equal("Story 11 " + target + " states that write-then-replace is atomic", true, contract.AtomicWriteThenReplaceSupported);
                Check.Equal("Story 11 " + target + " states that the prior valid save survives an interrupted write", true, contract.PriorValidSaveSurvivesInterruption);
                Check.Equal("Story 11 " + target + " states that the prior valid save survives quota exhaustion", true, contract.PriorValidSaveSurvivesQuotaExhaustion);
                Check.Equal("Story 11 " + target + " states that the prior valid save survives process termination", true, contract.PriorValidSaveSurvivesProcessTermination);
                Check.That("Story 11 " + target + " names the persistent data path template", contract.DefaultPersistentDataPathTemplate.Length > 0);
            }

            PersistenceDurabilityContract browser = SaveMigration.GetTargetDurabilityContract(RuntimePlatform.WebGLPlayer);
            Check.Equal("Story 11 the browser target belongs to the browser family", "Browser", browser.TargetFamily);
            Check.Equal("Story 11 the browser target needs an explicit asynchronous flush", true, browser.RequiresExplicitAsyncFlushOnTarget);
            Check.That("Story 11 the browser target names its asynchronous flush mechanism", browser.AsyncFlushMechanism.Contains("IndexedDB", StringComparison.Ordinal));

            PersistenceDurabilityContract desktop = SaveMigration.GetTargetDurabilityContract(RuntimePlatform.OSXPlayer);
            Check.Equal("Story 11 the desktop target belongs to the desktop family", "Desktop", desktop.TargetFamily);
            Check.Equal("Story 11 the desktop target needs no explicit asynchronous flush", false, desktop.RequiresExplicitAsyncFlushOnTarget);

            PersistenceDurabilityContract mobile = SaveMigration.GetTargetDurabilityContract(RuntimePlatform.Android);
            Check.Equal("Story 11 the mobile target belongs to the mobile family", "Mobile", mobile.TargetFamily);
            Check.Equal("Story 11 the mobile target needs no explicit asynchronous flush", false, mobile.RequiresExplicitAsyncFlushOnTarget);
        }

        private static void TestStory26PersistentDataPathResolvesPerTargetToUnityCompanyProductLocation()
        {
            // The shim resolves the location rule per target; the real device path of a shipped
            // player belongs to the target workstream, so no device directory is asserted here.
            string unixHome = "/home/builder";
            string windowsHome = "C:\\Users\\Builder";

            Check.Equal(
                "Story 26 the macOS target resolves to Library/Application Support/<company>/<product>",
                "/home/builder/Library/Application Support/Unitas Studios/Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.OSXPlayer, ProjectCompanyName, ProjectProductName, unixHome));
            Check.Equal(
                "Story 26 the macOS editor target resolves to the same macOS location",
                "/home/builder/Library/Application Support/Unitas Studios/Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.OSXEditor, ProjectCompanyName, ProjectProductName, unixHome));
            Check.Equal(
                "Story 26 the Windows target resolves to AppData/LocalLow/<company>/<product>",
                "C:\\Users\\Builder\\AppData\\LocalLow\\Unitas Studios\\Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.WindowsPlayer, ProjectCompanyName, ProjectProductName, windowsHome));
            Check.Equal(
                "Story 26 the Windows editor target resolves to the same Windows location",
                "C:\\Users\\Builder\\AppData\\LocalLow\\Unitas Studios\\Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.WindowsEditor, ProjectCompanyName, ProjectProductName, windowsHome));
            Check.Equal(
                "Story 26 the Linux target resolves to .config/unity3d/<company>/<product>",
                "/home/builder/.config/unity3d/Unitas Studios/Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.LinuxPlayer, ProjectCompanyName, ProjectProductName, unixHome));
            Check.Equal(
                "Story 26 the Linux editor target resolves to the same Linux location",
                "/home/builder/.config/unity3d/Unitas Studios/Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.LinuxEditor, ProjectCompanyName, ProjectProductName, unixHome));
            Check.Equal(
                "Story 26 the browser target resolves to /idbfs/<company>/<product>",
                "/idbfs/Unitas Studios/Unitas Game",
                Application.ResolvePersistentDataPathForTarget(RuntimePlatform.WebGLPlayer, ProjectCompanyName, ProjectProductName, unixHome));

            IApplicationHost? savedHost = Application.Host;
            FakeProjectHost host = new FakeProjectHost("/home/builder");
            Application.Host = host;
            Application.PlatformOverride = RuntimePlatform.WebGLPlayer;
            try
            {
                Check.Equal("Story 26 the target platform drives the resolution", (int)RuntimePlatform.WebGLPlayer, (int)Application.platform);
                Check.Equal(
                    "Story 26 Application.persistentDataPath resolves to the browser location of the project's company and product",
                    "/idbfs/" + ProjectCompanyName + "/" + ProjectProductName,
                    Application.persistentDataPath);
                Check.Equal("Story 26 the project's company name is used for the location", ProjectCompanyName, Application.companyName);
                Check.Equal("Story 26 the project's product name is used for the location", ProjectProductName, Application.productName);
            }
            finally
            {
                Application.PlatformOverride = null;
                Application.Host = savedHost;
            }
        }

        private static void TestStory26PlayerPrefsStoragePrefersTheArmedPersistentDataPathOverTheInternalSeam()
        {
            string armedRoot = CreateTempRoot();
            string seamRoot = CreateTempRoot();
            string? savedSeam = PlayerPrefs.StorageDirectoryOverride;
            string? savedArmed = Application.PersistentDataPathOverride;
            try
            {
                PlayerPrefs.StorageDirectoryOverride = seamRoot;
                Application.PersistentDataPathOverride = armedRoot;
                PlayerPrefs.Reload();

                Check.Equal(
                    "Story 26 the armed persistent data path outranks the internal storage directory seam",
                    Path.Combine(Path.GetFullPath(armedRoot), "prefs", "PlayerPrefs.ini"),
                    Path.GetFullPath(PlayerPrefs.StorageFilePath));

                Application.PersistentDataPathOverride = null;
                PlayerPrefs.StorageDirectoryOverride = seamRoot;
                PlayerPrefs.Reload();

                Check.Equal(
                    "Story 26 the internal storage directory seam applies when no persistent data path is armed",
                    Path.Combine(Path.GetFullPath(seamRoot), "PlayerPrefs.ini"),
                    Path.GetFullPath(PlayerPrefs.StorageFilePath));
            }
            finally
            {
                Application.PersistentDataPathOverride = savedArmed;
                PlayerPrefs.StorageDirectoryOverride = savedSeam;
                PlayerPrefs.Reload();
                CleanupTempRoot(armedRoot);
                CleanupTempRoot(seamRoot);
            }
        }

        private static void TestStory35ApplicationReportsProject2022_3VersionAndIgnoresQuitDuringPlay()
        {
            IApplicationHost? savedHost = Application.Host;
            string? savedVersionOverride = Application.UnityVersionOverride;
            Application.Host = null;
            Application.UnityVersionOverride = null;
            string tempProject = CreateTempRoot();
            try
            {
                Check.Equal("Story 35 the headless session reports the project's 2022.3 LTS version", "2022.3.0f1", Application.unityVersion);
                Check.That("Story 35 the reported version is a 2022.3 version", Application.unityVersion.StartsWith("2022.3.", StringComparison.Ordinal));

                string projectSettings = Path.Combine(tempProject, "ProjectSettings");
                Directory.CreateDirectory(projectSettings);
                File.WriteAllText(
                    Path.Combine(projectSettings, "ProjectVersion.txt"),
                    "m_EditorVersion: 2022.3.20f1\nm_EditorVersionWithRevision: 2022.3.20f1 (61c2feb0970d)\n");
                Check.Equal(
                    "Story 35 the project's own ProjectVersion.txt supplies the 2022.3 version",
                    "2022.3.20f1",
                    Application.ResolveUnityVersionForProject(tempProject));

                FakeProjectHost host = new FakeProjectHost(tempProject);
                Application.Host = host;
                Check.Equal("Story 35 the project session reports the project's 2022.3 version", "2022.3.20f1", Application.unityVersion);

                int ignoredBefore = Application.IgnoredPlayModeQuitCount;
                int requestsBefore = Application.QuitRequestCount;
                LogRecorder.Clear();
                Application.IgnoreQuitInPlayMode = true;
                try
                {
                    Application.Quit();
                }
                finally
                {
                    Application.IgnoreQuitInPlayMode = false;
                }

                Check.That("Story 35 Application.Quit is ignored during play", Application.isPlaying);
                Check.Equal("Story 35 the ignored quit does not reach the host session", 0, host.EndSessionCount);
                Check.Equal("Story 35 the ignored quit is counted as ignored", ignoredBefore + 1, Application.IgnoredPlayModeQuitCount);
                Check.Equal("Story 35 the ignored quit is counted as a quit request", requestsBefore + 1, Application.QuitRequestCount);
                Check.That(
                    "Story 35 the ignored quit is logged",
                    LogRecorder.Search("Application.Quit is ignored in the Editor during Play Mode.").Count == 1);

                Application.Quit();
                Check.Equal("Story 35 a quit outside play mode reaches the host session", 1, host.EndSessionCount);
            }
            finally
            {
                Application.Host = savedHost;
                Application.UnityVersionOverride = savedVersionOverride;
                CleanupTempRoot(tempProject);
            }
        }

        private static SaveMigrationStep[] CreateSteps()
        {
            return new[]
            {
                new SaveMigrationStep(
                    1,
                    2,
                    (string input, out string migrated, IList<string> unretainedDataNotes, out string failureReason) =>
                    {
                        migrated = input
                            .Replace("\"version\":1", "\"version\":2", StringComparison.Ordinal)
                            .Replace("\"chapter\":5", "\"chapter\":6", StringComparison.Ordinal);
                        unretainedDataNotes.Add("legacy field 'checkpoint' is not retained");
                        failureReason = string.Empty;
                        return true;
                    }),
                new SaveMigrationStep(
                    2,
                    3,
                    (string input, out string migrated, IList<string> unretainedDataNotes, out string failureReason) =>
                    {
                        migrated = input
                            .Replace("\"version\":2", "\"version\":3", StringComparison.Ordinal)
                            .Replace("\"chapter\":6", "\"chapter\":7", StringComparison.Ordinal);
                        unretainedDataNotes.Add("legacy field 'difficulty' is not retained");
                        failureReason = string.Empty;
                        return true;
                    }),
            };
        }

        private static SaveMigrationStep[] CreateRefusingSteps()
        {
            return new[]
            {
                new SaveMigrationStep(
                    1,
                    2,
                    (string input, out string migrated, IList<string> unretainedDataNotes, out string failureReason) =>
                    {
                        migrated = string.Empty;
                        failureReason = "the load code does not accept this save";
                        return false;
                    }),
            };
        }

        private static string CreateTempRoot()
        {
            string path = Path.Combine(Path.GetTempPath(), "unitas_save_migration_test_" + Guid.NewGuid().ToString("N"));
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

        private sealed class FakeProjectHost : IApplicationHost
        {
            public FakeProjectHost(string dataPath)
            {
                DataPath = dataPath;
            }

            public bool IsPlaying => true;

            public bool IsFocused => true;

            public bool IsPaused => false;

            public ApplicationBuildKind BuildKind => ApplicationBuildKind.Editor;

            public string DataPath { get; }

            public string StreamingAssetsPath => Path.Combine(DataPath, "StreamingAssets");

            public string CompanyName => ProjectCompanyName;

            public string ProductName => ProjectProductName;

            public string Version => "0.1";

            public int EndSessionCount { get; private set; }

            public void EndSession()
            {
                EndSessionCount++;
            }
        }
    }
}
