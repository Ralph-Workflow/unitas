using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace UnityEngine
{
    public enum SaveMigrationOutcome
    {
        Absent = 0,
        AlreadyCurrent = 1,
        Migrated = 2,
        Corrupt = 3,
        IncompatibleNewerVersion = 4,
        MigrationFailed = 5
    }

    public sealed class SaveStorageQuotaException : IOException
    {
        public SaveStorageQuotaException()
        {
            TargetPath = string.Empty;
        }

        public SaveStorageQuotaException(string message) : base(message)
        {
            TargetPath = string.Empty;
        }

        public SaveStorageQuotaException(string message, Exception innerException) : base(message, innerException)
        {
            TargetPath = string.Empty;
        }

        public SaveStorageQuotaException(
            string targetPath,
            long requiredBytes,
            long quotaBytes,
            string message) : base(message)
        {
            TargetPath = targetPath ?? string.Empty;
            RequiredBytes = requiredBytes;
            QuotaBytes = quotaBytes;
        }

        public string TargetPath { get; }

        public long RequiredBytes { get; }

        public long QuotaBytes { get; }
    }

    public sealed class SaveWriteInterruptionException : IOException
    {
        public SaveWriteInterruptionException()
        {
            TargetPath = string.Empty;
            TempPath = string.Empty;
        }

        public SaveWriteInterruptionException(string message) : base(message)
        {
            TargetPath = string.Empty;
            TempPath = string.Empty;
        }

        public SaveWriteInterruptionException(string message, Exception innerException) : base(message, innerException)
        {
            TargetPath = string.Empty;
            TempPath = string.Empty;
        }

        public SaveWriteInterruptionException(
            string targetPath,
            string tempPath,
            int bytesWrittenBeforeInterruption,
            int totalBytes,
            string message) : base(message)
        {
            TargetPath = targetPath ?? string.Empty;
            TempPath = tempPath ?? string.Empty;
            BytesWrittenBeforeInterruption = bytesWrittenBeforeInterruption;
            TotalBytes = totalBytes;
        }

        public string TargetPath { get; }

        public string TempPath { get; }

        public int BytesWrittenBeforeInterruption { get; }

        public int TotalBytes { get; }
    }

    public delegate bool SaveMigrationStepHandler(
        string inputPayload,
        out string migratedPayload,
        IList<string> unretainedDataNotes,
        out string failureReason);

    public sealed class SaveMigrationStep
    {
        public SaveMigrationStep(int fromVersion, int toVersion, SaveMigrationStepHandler handler)
        {
            if (fromVersion <= 0)
            {
                throw new ArgumentOutOfRangeException(nameof(fromVersion));
            }

            if (toVersion <= fromVersion)
            {
                throw new ArgumentOutOfRangeException(nameof(toVersion));
            }

            if (handler == null)
            {
                throw new ArgumentNullException(nameof(handler));
            }

            FromVersion = fromVersion;
            ToVersion = toVersion;
            Handler = handler;
        }

        public int FromVersion { get; }

        public int ToVersion { get; }

        public SaveMigrationStepHandler Handler { get; }
    }

    public sealed class SaveMigrationResult
    {
        public SaveMigrationResult(
            SaveMigrationOutcome outcome,
            int inputVersion,
            int resultingVersion,
            string payload,
            string filePath,
            bool priorFilePreserved,
            string diagnosticMessage,
            IReadOnlyList<int> appliedSteps,
            IReadOnlyList<string> unretainedDataNotes)
        {
            Outcome = outcome;
            InputVersion = inputVersion;
            ResultingVersion = resultingVersion;
            Payload = payload ?? string.Empty;
            FilePath = filePath ?? string.Empty;
            PriorFilePreserved = priorFilePreserved;
            DiagnosticMessage = diagnosticMessage ?? string.Empty;
            AppliedSteps = appliedSteps ?? Array.Empty<int>();
            UnretainedDataNotes = unretainedDataNotes ?? Array.Empty<string>();
        }

        public SaveMigrationOutcome Outcome { get; }

        public int InputVersion { get; }

        public int ResultingVersion { get; }

        public string Payload { get; }

        public string FilePath { get; }

        public bool PriorFilePreserved { get; }

        public string DiagnosticMessage { get; }

        public IReadOnlyList<int> AppliedSteps { get; }

        public IReadOnlyList<string> UnretainedDataNotes { get; }
    }

    public sealed class PersistenceDurabilityContract
    {
        public PersistenceDurabilityContract(
            RuntimePlatform platform,
            string targetFamily,
            string featureId,
            string playerPrefsSaveDurabilityPoint,
            string persistentDataFileWriteDurabilityPoint,
            bool requiresExplicitAsyncFlushOnTarget,
            string asyncFlushMechanism,
            bool unsavedPlayerPrefsLostOnCrash,
            string whatCrashLosesBeforeSaveOrFlush,
            bool atomicWriteThenReplaceSupported,
            bool priorValidSaveSurvivesInterruption,
            bool priorValidSaveSurvivesQuotaExhaustion,
            bool priorValidSaveSurvivesProcessTermination,
            string defaultPersistentDataPathTemplate)
        {
            Platform = platform;
            TargetFamily = targetFamily ?? string.Empty;
            FeatureId = featureId ?? "SAV-001";
            PlayerPrefsSaveDurabilityPoint = playerPrefsSaveDurabilityPoint ?? string.Empty;
            PersistentDataFileWriteDurabilityPoint = persistentDataFileWriteDurabilityPoint ?? string.Empty;
            RequiresExplicitAsyncFlushOnTarget = requiresExplicitAsyncFlushOnTarget;
            AsyncFlushMechanism = asyncFlushMechanism ?? string.Empty;
            UnsavedPlayerPrefsLostOnCrash = unsavedPlayerPrefsLostOnCrash;
            WhatCrashLosesBeforeSaveOrFlush = whatCrashLosesBeforeSaveOrFlush ?? string.Empty;
            AtomicWriteThenReplaceSupported = atomicWriteThenReplaceSupported;
            PriorValidSaveSurvivesInterruption = priorValidSaveSurvivesInterruption;
            PriorValidSaveSurvivesQuotaExhaustion = priorValidSaveSurvivesQuotaExhaustion;
            PriorValidSaveSurvivesProcessTermination = priorValidSaveSurvivesProcessTermination;
            DefaultPersistentDataPathTemplate = defaultPersistentDataPathTemplate ?? string.Empty;
        }

        public RuntimePlatform Platform { get; }

        public string TargetFamily { get; }

        public string FeatureId { get; }

        public string PlayerPrefsSaveDurabilityPoint { get; }

        public string PersistentDataFileWriteDurabilityPoint { get; }

        public bool RequiresExplicitAsyncFlushOnTarget { get; }

        public string AsyncFlushMechanism { get; }

        public bool UnsavedPlayerPrefsLostOnCrash { get; }

        public string WhatCrashLosesBeforeSaveOrFlush { get; }

        public bool AtomicWriteThenReplaceSupported { get; }

        public bool PriorValidSaveSurvivesInterruption { get; }

        public bool PriorValidSaveSurvivesQuotaExhaustion { get; }

        public bool PriorValidSaveSurvivesProcessTermination { get; }

        public string DefaultPersistentDataPathTemplate { get; }
    }

    public static class SaveMigration
    {
        private static long _simulatedQuotaBytes = -1;
        private static int _simulatedInterruptionAfterBytes = -1;
        private static StorageFailureReason _lastWriteFailureReason = StorageFailureReason.None;
        private static string? _lastWriteError;

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

        public static StorageFailureReason LastWriteFailureReason => _lastWriteFailureReason;

        public static string? LastWriteError => _lastWriteError;

        public static void ResetSimulationControls()
        {
            _simulatedQuotaBytes = -1;
            _simulatedInterruptionAfterBytes = -1;
            _lastWriteFailureReason = StorageFailureReason.None;
            _lastWriteError = null;
        }

        public static void WriteSaveFileAtomically(string targetPath, string content)
        {
            if (string.IsNullOrEmpty(targetPath))
            {
                throw new ArgumentException("Save file path must not be null or empty.", nameof(targetPath));
            }

            if (content == null)
            {
                throw new ArgumentNullException(nameof(content));
            }

            WriteSaveFileAtomically(targetPath, new UTF8Encoding(false).GetBytes(content));
        }

        public static void WriteSaveFileAtomically(string targetPath, byte[] content)
        {
            if (string.IsNullOrEmpty(targetPath))
            {
                throw new ArgumentException("Save file path must not be null or empty.", nameof(targetPath));
            }

            if (content == null)
            {
                throw new ArgumentNullException(nameof(content));
            }

            string fullTargetPath = Path.GetFullPath(targetPath);
            string directory = Path.GetDirectoryName(fullTargetPath) ?? Application.persistentDataPath;
            string tempPath = Path.Combine(directory, Path.GetFileName(fullTargetPath) + ".tmp." + Guid.NewGuid().ToString("N"));
            bool preserveInterruptedTemp = false;

            try
            {
                Directory.CreateDirectory(directory);

                if (_simulatedQuotaBytes >= 0 && content.LongLength > _simulatedQuotaBytes)
                {
                    _lastWriteFailureReason = StorageFailureReason.QuotaExceeded;
                    string quotaMessage = string.Concat(
                        "SAV-001 storage quota exceeded writing '",
                        fullTargetPath,
                        "': required ",
                        content.LongLength.ToString(CultureInfo.InvariantCulture),
                        " bytes exceeds quota of ",
                        _simulatedQuotaBytes.ToString(CultureInfo.InvariantCulture),
                        " bytes.");
                    _lastWriteError = quotaMessage;
                    Debug.LogError(quotaMessage);
                    throw new SaveStorageQuotaException(fullTargetPath, content.LongLength, _simulatedQuotaBytes, quotaMessage);
                }

                if (_simulatedInterruptionAfterBytes >= 0 && _simulatedInterruptionAfterBytes < content.Length)
                {
                    preserveInterruptedTemp = true;
                    _lastWriteFailureReason = StorageFailureReason.IoError;
                    byte[] partialBytes = new byte[_simulatedInterruptionAfterBytes];
                    Buffer.BlockCopy(content, 0, partialBytes, 0, _simulatedInterruptionAfterBytes);
                    File.WriteAllBytes(tempPath, partialBytes);
                    string interruptedMessage = string.Concat(
                        "SAV-001 simulated termination/interruption during write to '",
                        fullTargetPath,
                        "': wrote ",
                        _simulatedInterruptionAfterBytes.ToString(CultureInfo.InvariantCulture),
                        " of ",
                        content.Length.ToString(CultureInfo.InvariantCulture),
                        " bytes to temporary staging file '",
                        tempPath,
                        "' before atomic replacement.");
                    _lastWriteError = interruptedMessage;
                    Debug.LogError(interruptedMessage);
                    throw new SaveWriteInterruptionException(
                        fullTargetPath,
                        tempPath,
                        _simulatedInterruptionAfterBytes,
                        content.Length,
                        interruptedMessage);
                }

                File.WriteAllBytes(tempPath, content);
                if (File.Exists(fullTargetPath))
                {
                    File.Replace(tempPath, fullTargetPath, null);
                }
                else
                {
                    File.Move(tempPath, fullTargetPath);
                }
                _lastWriteFailureReason = StorageFailureReason.None;
                _lastWriteError = null;
            }
            catch (SaveStorageQuotaException)
            {
                throw;
            }
            catch (SaveWriteInterruptionException)
            {
                throw;
            }
            catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException || ex is NotSupportedException)
            {
                _lastWriteFailureReason = ex is UnauthorizedAccessException
                    ? StorageFailureReason.PermissionDenied
                    : StorageFailureReason.IoError;
                string err = "SAV-001 save write failed for '" + fullTargetPath + "': " + ex.Message;
                _lastWriteError = err;
                Debug.LogError(err);
                throw;
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

        public static SaveMigrationResult MigrateFile(
            string saveFilePath,
            int currentSchemaVersion,
            IReadOnlyList<SaveMigrationStep> steps,
            string versionFieldName = "version")
        {
            if (string.IsNullOrEmpty(saveFilePath))
            {
                throw new ArgumentException("Save file path must not be null or empty.", nameof(saveFilePath));
            }

            if (currentSchemaVersion <= 0)
            {
                throw new ArgumentOutOfRangeException(nameof(currentSchemaVersion));
            }

            if (steps == null)
            {
                throw new ArgumentNullException(nameof(steps));
            }

            string fullPath = Path.GetFullPath(saveFilePath);
            if (!File.Exists(fullPath))
            {
                return new SaveMigrationResult(
                    SaveMigrationOutcome.Absent,
                    0,
                    0,
                    string.Empty,
                    fullPath,
                    false,
                    "Save file is absent at '" + fullPath + "'.",
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            string rawPayload;
            try
            {
                rawPayload = File.ReadAllText(fullPath, Encoding.UTF8);
            }
            catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException)
            {
                string ioMsg = "Failed to read save file '" + fullPath + "': " + ex.Message;
                Debug.LogError(ioMsg);
                return new SaveMigrationResult(
                    SaveMigrationOutcome.MigrationFailed,
                    0,
                    0,
                    string.Empty,
                    fullPath,
                    true,
                    ioMsg,
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            SaveMigrationResult memoryResult = MigratePayload(
                rawPayload,
                currentSchemaVersion,
                steps,
                versionFieldName,
                fullPath);

            if (memoryResult.Outcome != SaveMigrationOutcome.Migrated)
            {
                return memoryResult;
            }

            try
            {
                WriteSaveFileAtomically(fullPath, memoryResult.Payload);
                return memoryResult;
            }
            catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException || ex is NotSupportedException)
            {
                string writeFailMsg = "Save migration from v"
                    + memoryResult.InputVersion.ToString(CultureInfo.InvariantCulture)
                    + " to v"
                    + memoryResult.ResultingVersion.ToString(CultureInfo.InvariantCulture)
                    + " failed during atomic write to '"
                    + fullPath
                    + "': "
                    + ex.Message;
                Debug.LogError(writeFailMsg);
                return new SaveMigrationResult(
                    SaveMigrationOutcome.MigrationFailed,
                    memoryResult.InputVersion,
                    memoryResult.InputVersion,
                    rawPayload,
                    fullPath,
                    File.Exists(fullPath),
                    writeFailMsg,
                    Array.Empty<int>(),
                    memoryResult.UnretainedDataNotes);
            }
        }

        public static SaveMigrationResult MigratePayload(
            string? rawPayload,
            int currentSchemaVersion,
            IReadOnlyList<SaveMigrationStep> steps,
            string versionFieldName = "version",
            string filePath = "")
        {
            if (currentSchemaVersion <= 0)
            {
                throw new ArgumentOutOfRangeException(nameof(currentSchemaVersion));
            }

            if (steps == null)
            {
                throw new ArgumentNullException(nameof(steps));
            }

            string effectiveField = string.IsNullOrEmpty(versionFieldName) ? "version" : versionFieldName;
            if (string.IsNullOrWhiteSpace(rawPayload))
            {
                string emptyMsg = "Save data is corrupt: empty or whitespace payload.";
                Debug.LogError(emptyMsg);
                return new SaveMigrationResult(
                    SaveMigrationOutcome.Corrupt,
                    0,
                    0,
                    string.Empty,
                    filePath,
                    !string.IsNullOrEmpty(filePath) && File.Exists(filePath),
                    emptyMsg,
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            if (!TryExtractJsonSchemaVersion(rawPayload!, effectiveField, out int inputVersion, out string parseError))
            {
                string corruptMsg = "Save data is corrupt: " + parseError;
                Debug.LogError(corruptMsg);
                return new SaveMigrationResult(
                    SaveMigrationOutcome.Corrupt,
                    0,
                    0,
                    rawPayload!,
                    filePath,
                    !string.IsNullOrEmpty(filePath) && File.Exists(filePath),
                    corruptMsg,
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            if (inputVersion <= 0)
            {
                string invalidVerMsg = "Save data is corrupt: non-positive schema version "
                    + inputVersion.ToString(CultureInfo.InvariantCulture)
                    + ".";
                Debug.LogError(invalidVerMsg);
                return new SaveMigrationResult(
                    SaveMigrationOutcome.Corrupt,
                    inputVersion,
                    inputVersion,
                    rawPayload!,
                    filePath,
                    !string.IsNullOrEmpty(filePath) && File.Exists(filePath),
                    invalidVerMsg,
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            if (inputVersion == currentSchemaVersion)
            {
                return new SaveMigrationResult(
                    SaveMigrationOutcome.AlreadyCurrent,
                    inputVersion,
                    currentSchemaVersion,
                    rawPayload!,
                    filePath,
                    true,
                    "Save data is already at current schema version "
                        + currentSchemaVersion.ToString(CultureInfo.InvariantCulture)
                        + ".",
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            if (inputVersion > currentSchemaVersion)
            {
                string newerMsg = "Save schema version "
                    + inputVersion.ToString(CultureInfo.InvariantCulture)
                    + " is newer than supported current schema version "
                    + currentSchemaVersion.ToString(CultureInfo.InvariantCulture)
                    + ".";
                Debug.LogWarning(newerMsg);
                return new SaveMigrationResult(
                    SaveMigrationOutcome.IncompatibleNewerVersion,
                    inputVersion,
                    inputVersion,
                    rawPayload!,
                    filePath,
                    true,
                    newerMsg,
                    Array.Empty<int>(),
                    Array.Empty<string>());
            }

            string workingPayload = rawPayload!;
            int workingVersion = inputVersion;
            List<int> appliedVersions = new List<int>();
            List<string> unretainedNotes = new List<string>();

            while (workingVersion < currentSchemaVersion)
            {
                SaveMigrationStep? matchingStep = FindStepFromVersion(steps, workingVersion);
                if (matchingStep == null)
                {
                    string missingStepMsg = "No migration step registered from schema version "
                        + workingVersion.ToString(CultureInfo.InvariantCulture)
                        + " toward target version "
                        + currentSchemaVersion.ToString(CultureInfo.InvariantCulture)
                        + ".";
                    Debug.LogError(missingStepMsg);
                    return new SaveMigrationResult(
                        SaveMigrationOutcome.MigrationFailed,
                        inputVersion,
                        inputVersion,
                        rawPayload!,
                        filePath,
                        true,
                        missingStepMsg,
                        Array.Empty<int>(),
                        unretainedNotes);
                }

                string nextPayload;
                string stepFailureReason;
                bool stepSucceeded;
                try
                {
                    stepSucceeded = matchingStep.Handler(
                        workingPayload,
                        out nextPayload,
                        unretainedNotes,
                        out stepFailureReason);
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    stepSucceeded = false;
                    nextPayload = string.Empty;
                    stepFailureReason = ex.Message;
                }

                if (!stepSucceeded || string.IsNullOrWhiteSpace(nextPayload))
                {
                    string failMsg = "Migration step v"
                        + matchingStep.FromVersion.ToString(CultureInfo.InvariantCulture)
                        + " -> v"
                        + matchingStep.ToVersion.ToString(CultureInfo.InvariantCulture)
                        + " failed: "
                        + (string.IsNullOrEmpty(stepFailureReason) ? "handler returned false." : stepFailureReason);
                    Debug.LogError(failMsg);
                    return new SaveMigrationResult(
                        SaveMigrationOutcome.MigrationFailed,
                        inputVersion,
                        inputVersion,
                        rawPayload!,
                        filePath,
                        true,
                        failMsg,
                        Array.Empty<int>(),
                        unretainedNotes);
                }

                if (!TryExtractJsonSchemaVersion(nextPayload, effectiveField, out int emittedVersion, out string emittedParseErr)
                    || emittedVersion != matchingStep.ToVersion)
                {
                    string badOutputMsg = "Migration step v"
                        + matchingStep.FromVersion.ToString(CultureInfo.InvariantCulture)
                        + " -> v"
                        + matchingStep.ToVersion.ToString(CultureInfo.InvariantCulture)
                        + " produced invalid target payload: "
                        + (string.IsNullOrEmpty(emittedParseErr)
                            ? "expected version " + matchingStep.ToVersion.ToString(CultureInfo.InvariantCulture) + " but found " + emittedVersion.ToString(CultureInfo.InvariantCulture)
                            : emittedParseErr);
                    Debug.LogError(badOutputMsg);
                    return new SaveMigrationResult(
                        SaveMigrationOutcome.MigrationFailed,
                        inputVersion,
                        inputVersion,
                        rawPayload!,
                        filePath,
                        true,
                        badOutputMsg,
                        Array.Empty<int>(),
                        unretainedNotes);
                }

                workingPayload = nextPayload;
                workingVersion = matchingStep.ToVersion;
                appliedVersions.Add(workingVersion);
            }

            return new SaveMigrationResult(
                SaveMigrationOutcome.Migrated,
                inputVersion,
                workingVersion,
                workingPayload,
                filePath,
                true,
                "Migrated save data from v"
                    + inputVersion.ToString(CultureInfo.InvariantCulture)
                    + " to v"
                    + workingVersion.ToString(CultureInfo.InvariantCulture)
                    + ".",
                appliedVersions,
                unretainedNotes);
        }

        public static PersistenceDurabilityContract GetTargetDurabilityContract(RuntimePlatform targetPlatform)
        {
            switch (targetPlatform)
            {
                case RuntimePlatform.OSXEditor:
                case RuntimePlatform.OSXPlayer:
                case RuntimePlatform.OSXServer:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "Desktop",
                        "SAV-001",
                        "Durable immediately upon PlayerPrefs.Save() returning after atomic temp-file write and POSIX rename into ~/Library/Preferences/unity.<company>.<product>/PlayerPrefs.ini.",
                        "Durable when the save file write under Application.persistentDataPath flushes and closes (or when SaveMigration.WriteSaveFileAtomically completes temp-file write and atomic replace).",
                        false,
                        "Synchronous OS filesystem flush on save completion (no secondary browser IDBFS sync required).",
                        true,
                        "All PlayerPrefs.SetInt/SetFloat/SetString/DeleteKey/DeleteAll mutations since the last completed PlayerPrefs.Save() call, plus any in-flight unclosed temporary save file (.tmp) not yet atomically replaced.",
                        true,
                        true,
                        true,
                        true,
                        "~/Library/Preferences/unity.<company>.<product>");

                case RuntimePlatform.WindowsEditor:
                case RuntimePlatform.WindowsPlayer:
                case RuntimePlatform.WindowsServer:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "Desktop",
                        "SAV-001",
                        "Durable immediately upon PlayerPrefs.Save() returning after atomic temp-file write and replacement in %USERPROFILE%/AppData/LocalLow/<company>/<product>/prefs/PlayerPrefs.ini.",
                        "Durable when the save file write under Application.persistentDataPath flushes and closes (or when SaveMigration.WriteSaveFileAtomically completes temp-file write and atomic replace).",
                        false,
                        "Synchronous OS filesystem flush on save completion (no secondary browser IDBFS sync required).",
                        true,
                        "All PlayerPrefs mutations not yet committed via PlayerPrefs.Save(), plus any incomplete temporary file write interrupted before atomic replacement.",
                        true,
                        true,
                        true,
                        true,
                        "%USERPROFILE%/AppData/LocalLow/<company>/<product>/prefs");

                case RuntimePlatform.LinuxEditor:
                case RuntimePlatform.LinuxPlayer:
                case RuntimePlatform.LinuxServer:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "Desktop",
                        "SAV-001",
                        "Durable immediately upon PlayerPrefs.Save() returning after atomic temp-file write and POSIX rename into ~/.config/unity3d/<company>/<product>/prefs/PlayerPrefs.ini.",
                        "Durable when the save file write under Application.persistentDataPath flushes and closes (or when SaveMigration.WriteSaveFileAtomically completes temp-file write and atomic replace).",
                        false,
                        "Synchronous OS filesystem flush on save completion (no secondary browser IDBFS sync required).",
                        true,
                        "All PlayerPrefs mutations not yet committed via PlayerPrefs.Save(), plus any incomplete temporary file write interrupted before atomic replacement.",
                        true,
                        true,
                        true,
                        true,
                        "~/.config/unity3d/<company>/<product>/prefs");

                case RuntimePlatform.WebGLPlayer:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "Browser",
                        "SAV-001",
                        "PlayerPrefs.Save() stages the updated preference store in the Emscripten in-memory /idbfs/<company>/<product> mount and schedules an asynchronous IndexedDB (IDBFS.syncfs) transaction; durability across browser tab close or crash is reached only after the IndexedDB transaction commits.",
                        "File writes under Application.persistentDataPath (/idbfs/<company>/<product>) update the in-memory virtual filesystem first and become durable across browser reload or crash only after PlayerPrefs.Save() or explicit IDBFS IndexedDB synchronization completes without QuotaExceededError.",
                        true,
                        "IndexedDB IDBFS.syncfs asynchronous transaction commit after staging in /idbfs/<company>/<product>.",
                        true,
                        "All unsaved PlayerPrefs changes in memory AND any /idbfs/<company>/<product> file writes or PlayerPrefs.Save() calls whose asynchronous IndexedDB flush has not yet committed before tab termination or browser crash.",
                        true,
                        true,
                        true,
                        true,
                        "/idbfs/<company>/<product>");

                case RuntimePlatform.IPhonePlayer:
                case RuntimePlatform.tvOS:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "Mobile",
                        "SAV-001",
                        "Durable immediately upon PlayerPrefs.Save() returning after atomic temp-file write and replacement inside the app sandbox Documents/<company>/<product>/PlayerPrefs.ini.",
                        "Durable when the save file write under Application.persistentDataPath flushes and closes (or when SaveMigration.WriteSaveFileAtomically completes temp-file write and atomic replace).",
                        false,
                        "Synchronous sandbox filesystem flush on save completion.",
                        true,
                        "All PlayerPrefs mutations not yet committed via PlayerPrefs.Save(), plus any incomplete temporary file write interrupted before atomic replacement.",
                        true,
                        true,
                        true,
                        true,
                        "~/Documents/<company>/<product>");

                case RuntimePlatform.Android:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "Mobile",
                        "SAV-001",
                        "Durable immediately upon PlayerPrefs.Save() returning after atomic temp-file write and replacement inside /storage/emulated/0/Android/data/com.<company>.<product>/files/PlayerPrefs.ini.",
                        "Durable when the save file write under Application.persistentDataPath flushes and closes (or when SaveMigration.WriteSaveFileAtomically completes temp-file write and atomic replace).",
                        false,
                        "Synchronous app-storage filesystem flush on save completion.",
                        true,
                        "All PlayerPrefs mutations not yet committed via PlayerPrefs.Save(), plus any incomplete temporary file write interrupted before atomic replacement.",
                        true,
                        true,
                        true,
                        true,
                        "/storage/emulated/0/Android/data/com.<company>.<product>/files");

                default:
                    return new PersistenceDurabilityContract(
                        targetPlatform,
                        "ConsoleOrOther",
                        "SAV-001",
                        "Durable upon PlayerPrefs.Save() completing atomic write-then-replace in the platform's persistent user storage mount.",
                        "Durable when the save file write under Application.persistentDataPath completes atomic write-then-replace.",
                        false,
                        "Platform persistent storage commit on save completion.",
                        true,
                        "All PlayerPrefs mutations not yet committed via PlayerPrefs.Save(), plus any incomplete temporary file write interrupted before atomic replacement.",
                        true,
                        true,
                        true,
                        true,
                        "~/unitas/<company>-<product>");
            }
        }

        public static IReadOnlyList<PersistenceDurabilityContract> GetAllTargetDurabilityContracts()
        {
            return new[]
            {
                GetTargetDurabilityContract(RuntimePlatform.OSXEditor),
                GetTargetDurabilityContract(RuntimePlatform.OSXPlayer),
                GetTargetDurabilityContract(RuntimePlatform.WindowsEditor),
                GetTargetDurabilityContract(RuntimePlatform.WindowsPlayer),
                GetTargetDurabilityContract(RuntimePlatform.LinuxEditor),
                GetTargetDurabilityContract(RuntimePlatform.LinuxPlayer),
                GetTargetDurabilityContract(RuntimePlatform.WebGLPlayer),
                GetTargetDurabilityContract(RuntimePlatform.IPhonePlayer),
                GetTargetDurabilityContract(RuntimePlatform.Android),
            };
        }

        private static SaveMigrationStep? FindStepFromVersion(IReadOnlyList<SaveMigrationStep> steps, int fromVersion)
        {
            for (int i = 0; i < steps.Count; i++)
            {
                SaveMigrationStep step = steps[i];
                if (step != null && step.FromVersion == fromVersion)
                {
                    return step;
                }
            }

            return null;
        }

        private static bool TryExtractJsonSchemaVersion(
            string json,
            string versionFieldName,
            out int version,
            out string error)
        {
            version = 0;
            error = string.Empty;

            int index = 0;
            SkipWhitespace(json, ref index);
            if (index >= json.Length || json[index] != '{')
            {
                error = "payload is not a valid JSON object at position 0.";
                return false;
            }

            int depth = 0;
            bool inString = false;
            for (int i = index; i < json.Length; i++)
            {
                char ch = json[i];
                if (inString)
                {
                    if (ch == '\\')
                    {
                        i++;
                        continue;
                    }

                    if (ch == '"')
                    {
                        inString = false;
                    }

                    continue;
                }

                if (ch == '"')
                {
                    inString = true;
                }
                else if (ch == '{' || ch == '[')
                {
                    depth++;
                }
                else if (ch == '}' || ch == ']')
                {
                    depth--;
                    if (depth < 0)
                    {
                        error = "unbalanced closing bracket in JSON payload.";
                        return false;
                    }
                }
            }

            if (inString || depth != 0)
            {
                error = "unterminated JSON structure.";
                return false;
            }

            string quotedKey = "\"" + versionFieldName + "\"";
            int keyPos = json.IndexOf(quotedKey, StringComparison.Ordinal);
            if (keyPos < 0)
            {
                error = "missing required schema version field '" + versionFieldName + "'.";
                return false;
            }

            int cursor = keyPos + quotedKey.Length;
            SkipWhitespace(json, ref cursor);
            if (cursor >= json.Length || json[cursor] != ':')
            {
                error = "malformed ':' after schema version field '" + versionFieldName + "'.";
                return false;
            }

            cursor++;
            SkipWhitespace(json, ref cursor);
            if (cursor >= json.Length)
            {
                error = "missing value for schema version field '" + versionFieldName + "'.";
                return false;
            }

            int numStart = cursor;
            if (json[cursor] == '-')
            {
                cursor++;
            }

            while (cursor < json.Length && json[cursor] >= '0' && json[cursor] <= '9')
            {
                cursor++;
            }

            if (cursor == numStart || (cursor == numStart + 1 && json[numStart] == '-'))
            {
                error = "schema version field '" + versionFieldName + "' is not an integer.";
                return false;
            }

            string rawNumber = json.Substring(numStart, cursor - numStart);
            if (!int.TryParse(rawNumber, NumberStyles.Integer, CultureInfo.InvariantCulture, out version))
            {
                error = "schema version value '" + rawNumber + "' could not be parsed as an integer.";
                return false;
            }

            return true;
        }

        private static void SkipWhitespace(string text, ref int index)
        {
            while (index < text.Length)
            {
                char ch = text[index];
                if (ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n')
                {
                    index++;
                }
                else
                {
                    break;
                }
            }
        }
    }
}
