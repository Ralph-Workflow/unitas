using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Xml;
using System.Xml.Serialization;
using Newtonsoft.Json;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public enum SaveSerializerKind
    {
        JsonUtility = 0,
        NewtonsoftJson = 1,
        XmlSerializer = 2,
        BinaryFormatter = 3
    }

    public enum SaveTargetFamily
    {
        Desktop = 0,
        Mobile = 1,
        Browser = 2
    }

    public enum SaveScriptingBackend
    {
        Mono = 0,
        Il2CppAot = 1
    }

    public enum SaveCompatibilityState
    {
        Supported = 0,
        PartiallySupported = 1,
        Unsupported = 2,
        PlatformLimited = 3
    }

    public sealed class SerializerCompatibilityAssessment
    {
        public SerializerCompatibilityAssessment(
            SaveSerializerKind serializer,
            SaveTargetFamily targetFamily,
            SaveScriptingBackend scriptingBackend,
            SaveCompatibilityState state,
            bool isAvailableOnTarget,
            bool hasAotRestrictions,
            string packageId,
            string packageVersionLine,
            string featureId,
            string diagnosticGuidance,
            IReadOnlyList<string> exclusions)
        {
            Serializer = serializer;
            TargetFamily = targetFamily;
            ScriptingBackend = scriptingBackend;
            State = state;
            IsAvailableOnTarget = isAvailableOnTarget;
            HasAotRestrictions = hasAotRestrictions;
            PackageId = packageId;
            PackageVersionLine = packageVersionLine;
            FeatureId = featureId;
            DiagnosticGuidance = diagnosticGuidance;
            Exclusions = exclusions;
        }

        public SaveSerializerKind Serializer { get; }

        public SaveTargetFamily TargetFamily { get; }

        public SaveScriptingBackend ScriptingBackend { get; }

        public SaveCompatibilityState State { get; }

        public bool IsAvailableOnTarget { get; }

        public bool HasAotRestrictions { get; }

        public string PackageId { get; }

        public string PackageVersionLine { get; }

        public string FeatureId { get; }

        public string DiagnosticGuidance { get; }

        public IReadOnlyList<string> Exclusions { get; }
    }

    public sealed class StrippedTypeDiagnostic
    {
        public StrippedTypeDiagnostic(
            SaveSerializerKind serializer,
            string strippedTypeName,
            string strippedMemberName,
            string strippingLevel,
            string message,
            IReadOnlyList<string> preservationMechanisms)
        {
            Serializer = serializer;
            StrippedTypeName = strippedTypeName;
            StrippedMemberName = strippedMemberName;
            StrippingLevel = strippingLevel;
            Message = message;
            PreservationMechanisms = preservationMechanisms;
        }

        public SaveSerializerKind Serializer { get; }

        public string StrippedTypeName { get; }

        public string StrippedMemberName { get; }

        public string StrippingLevel { get; }

        public string Message { get; }

        public IReadOnlyList<string> PreservationMechanisms { get; }
    }

    public static class SaveSerializationSupport
    {
        private static readonly string[] BrowserMonoExclusions =
        {
            "Mono JIT scripting backend is not permitted on Browser delivery targets."
        };

        private static readonly string[] XmlAotExclusions =
        {
            "BLD-003 ahead-of-time compilation limits: no runtime IL emitter serializer generation under AOT."
        };

        private static readonly string[] BinaryFormatterExclusions =
        {
            "Unavailable on the Browser target.",
            "Subject to BLD-003 ahead-of-time compilation and managed code stripping limits."
        };

        private static readonly string[] DefaultPreservationMechanisms =
        {
            "link.xml",
            "[Preserve]",
            "[AlwaysLinkAssembly]",
            "[RequireAttributeUsages]"
        };

        public static string HostRuntimeDescription => ".NET 8.0 (Desktop Mono/JIT host)";

        public static SerializerCompatibilityAssessment Assess(
            SaveSerializerKind serializer,
            SaveTargetFamily targetFamily,
            SaveScriptingBackend scriptingBackend)
        {
            if (targetFamily == SaveTargetFamily.Browser && scriptingBackend == SaveScriptingBackend.Mono)
            {
                return new SerializerCompatibilityAssessment(
                    serializer,
                    targetFamily,
                    scriptingBackend,
                    SaveCompatibilityState.Unsupported,
                    false,
                    true,
                    string.Empty,
                    string.Empty,
                    "BLD-003",
                    "Browser targets require ahead-of-time compilation (IL2CPP/Wasm); Mono JIT is invalid on Browser.",
                    BrowserMonoExclusions);
            }

            switch (serializer)
            {
                case SaveSerializerKind.JsonUtility:
                    return new SerializerCompatibilityAssessment(
                        serializer,
                        targetFamily,
                        scriptingBackend,
                        SaveCompatibilityState.Supported,
                        true,
                        false,
                        "UnityEngine.JsonSerializationModule",
                        "2022.3 LTS",
                        "SAV-001",
                        "Supported on every scripting backend (Mono and IL2CPP AOT) across Desktop, Mobile, and Browser targets.",
                        Array.Empty<string>());

                case SaveSerializerKind.NewtonsoftJson:
                    return new SerializerCompatibilityAssessment(
                        serializer,
                        targetFamily,
                        scriptingBackend,
                        SaveCompatibilityState.Supported,
                        true,
                        scriptingBackend == SaveScriptingBackend.Il2CppAot,
                        "com.unity.nuget.newtonsoft-json",
                        "3.x",
                        "SAV-001",
                        "Supported on every scripting backend via com.unity.nuget.newtonsoft-json 3.x; under AOT/stripping, preserve reflected constructors and members via link.xml or [Preserve].",
                        Array.Empty<string>());

                case SaveSerializerKind.XmlSerializer:
                    return new SerializerCompatibilityAssessment(
                        serializer,
                        targetFamily,
                        scriptingBackend,
                        SaveCompatibilityState.Supported,
                        true,
                        scriptingBackend == SaveScriptingBackend.Il2CppAot,
                        "System.Xml.XmlSerializer",
                        "2022.3 LTS",
                        "SAV-001",
                        scriptingBackend == SaveScriptingBackend.Il2CppAot
                            ? "Supported with BLD-003 ahead-of-time compilation limits: runtime code generation is unavailable under AOT, and serialized types and parameterless constructors must be preserved from managed stripping."
                            : "Supported on Mono JIT across Desktop and Mobile targets.",
                        scriptingBackend == SaveScriptingBackend.Il2CppAot
                            ? XmlAotExclusions
                            : Array.Empty<string>());

                case SaveSerializerKind.BinaryFormatter:
                    if (targetFamily == SaveTargetFamily.Browser)
                    {
                        return new SerializerCompatibilityAssessment(
                            serializer,
                            targetFamily,
                            scriptingBackend,
                            SaveCompatibilityState.Unsupported,
                            false,
                            true,
                            "System.Runtime.Serialization.Formatters.Binary.BinaryFormatter",
                            "2022.3 LTS",
                            "SAV-001",
                            "BinaryFormatter is Partially supported overall and unavailable on the Browser (WebAssembly) target. Migrate save data to JsonUtility, Newtonsoft.Json (com.unity.nuget.newtonsoft-json 3.x), or XmlSerializer.",
                            BinaryFormatterExclusions);
                    }

                    return new SerializerCompatibilityAssessment(
                        serializer,
                        targetFamily,
                        scriptingBackend,
                        SaveCompatibilityState.PartiallySupported,
                        true,
                        scriptingBackend == SaveScriptingBackend.Il2CppAot,
                        "System.Runtime.Serialization.Formatters.Binary.BinaryFormatter",
                        "2022.3 LTS",
                        "SAV-001",
                        "BinaryFormatter is Partially supported: unavailable on the Browser target and subject to BLD-003 ahead-of-time compilation and managed code stripping limits. Prefer JsonUtility, Newtonsoft.Json, or XmlSerializer.",
                        BinaryFormatterExclusions);

                default:
                    throw new ArgumentOutOfRangeException(nameof(serializer));
            }
        }

        public static StrippedTypeDiagnostic DiagnoseStrippedMemberFailure(
            SaveSerializerKind serializer,
            string strippedTypeName,
            string strippedMemberName,
            string strippingLevel)
        {
            if (string.IsNullOrWhiteSpace(strippedTypeName))
            {
                throw new ArgumentException("Type name must be provided.", nameof(strippedTypeName));
            }

            string member = string.IsNullOrWhiteSpace(strippedMemberName) ? ".ctor" : strippedMemberName;
            string level = string.IsNullOrWhiteSpace(strippingLevel) ? "High" : strippingLevel;
            string message = string.Concat(
                "SAV-001/BLD-003 serialization failure in ",
                serializer.ToString(),
                ": type '",
                strippedTypeName,
                "' and member '",
                member,
                "' were removed by managed code stripping (level: ",
                level,
                ") or ahead-of-time compilation. Preserve the type and member using link.xml, [Preserve], [AlwaysLinkAssembly], or [RequireAttributeUsages].");

            Debug.LogError(message);

            return new StrippedTypeDiagnostic(
                serializer,
                strippedTypeName,
                member,
                level,
                message,
                DefaultPreservationMechanisms);
        }
    }

    [Serializable]
    [JsonObject(MemberSerialization.OptIn)]
    public struct MatrixSaveCoordStruct
    {
        [JsonProperty("gridX")]
        public int gridX;

        [JsonProperty("gridY")]
        public int gridY;

        [SerializeField]
        [JsonProperty("_elevation")]
        private float _elevation;

        [XmlElement("_elevation")]
        public float XmlElevation
        {
            get => _elevation;
            set => _elevation = value;
        }

        public float GetElevation() => _elevation;

        public void SetElevation(float value) => _elevation = value;
    }

    [Serializable]
    [JsonObject(MemberSerialization.OptIn)]
    public sealed class MatrixSaveInventoryEntry
    {
        [JsonProperty("itemId")]
        public string itemId = string.Empty;

        [JsonProperty("quantity")]
        public int quantity;

        [SerializeField]
        [JsonProperty("_durability")]
        private int _durability = 100;

        [NonSerialized]
        [XmlIgnore]
        public int cachedTooltipHash = 77;

        [XmlElement("_durability")]
        public int XmlDurability
        {
            get => _durability;
            set => _durability = value;
        }

        public int GetDurability() => _durability;

        public void SetDurability(int value) => _durability = value;
    }

    [Serializable]
    [JsonObject(MemberSerialization.OptIn)]
    public sealed class MatrixSaveEnvelope
    {
        [JsonProperty("profileId")]
        public string profileId = string.Empty;

        [JsonProperty("schemaVersion")]
        public int schemaVersion = 2;

        [JsonProperty("playTimeSeconds")]
        public float playTimeSeconds;

        [JsonProperty("hardcoreMode")]
        public bool hardcoreMode;

        [SerializeField]
        [JsonProperty("_checkpointToken")]
        private string _checkpointToken = "init_token";

        [SerializeField]
        [JsonProperty("_secretSeed")]
        private int _secretSeed = 42;

        private int _unserializedPrivateCounter = 19;

        [NonSerialized]
        [XmlIgnore]
        public string runtimeSessionNote = "transient_only";

        [JsonProperty("lastCheckpointCoord")]
        public MatrixSaveCoordStruct lastCheckpointCoord;

        [JsonProperty("primaryWeapon")]
        public MatrixSaveInventoryEntry primaryWeapon = new MatrixSaveInventoryEntry();

        [JsonProperty("unlockedChapters")]
        public int[] unlockedChapters = Array.Empty<int>();

        [JsonProperty("activeModifiers")]
        public string[] activeModifiers = Array.Empty<string>();

        [JsonProperty("completedMilestones")]
        public List<int> completedMilestones = new List<int>();

        [JsonProperty("backpackItems")]
        public List<MatrixSaveInventoryEntry> backpackItems = new List<MatrixSaveInventoryEntry>();

        [XmlElement("_checkpointToken")]
        public string XmlCheckpointToken
        {
            get => _checkpointToken;
            set => _checkpointToken = value ?? string.Empty;
        }

        [XmlElement("_secretSeed")]
        public int XmlSecretSeed
        {
            get => _secretSeed;
            set => _secretSeed = value;
        }

        public string GetCheckpointToken() => _checkpointToken;

        public void SetCheckpointToken(string value) => _checkpointToken = value;

        public int GetSecretSeed() => _secretSeed;

        public void SetSecretSeed(int value) => _secretSeed = value;

        public int GetUnserializedPrivateCounter() => _unserializedPrivateCounter;

        public void SetUnserializedPrivateCounter(int value) => _unserializedPrivateCounter = value;
    }

    [Serializable]
    [JsonObject(MemberSerialization.OptIn)]
    public sealed class MatrixSavePayloadV1
    {
        [JsonProperty("version")]
        public int version = 1;

        [JsonProperty("heroName")]
        public string heroName = string.Empty;

        [JsonProperty("gold")]
        public int gold;

        [JsonProperty("legacyStageIndex")]
        public int legacyStageIndex;

        [SerializeField]
        [JsonProperty("_legacyDifficultyTag")]
        private string _legacyDifficultyTag = "normal";

        [JsonProperty("unlockedItemIds")]
        public string[] unlockedItemIds = Array.Empty<string>();

        [XmlElement("_legacyDifficultyTag")]
        public string XmlLegacyDifficultyTag
        {
            get => _legacyDifficultyTag;
            set => _legacyDifficultyTag = value ?? string.Empty;
        }

        public string GetLegacyDifficultyTag() => _legacyDifficultyTag;

        public void SetLegacyDifficultyTag(string value) => _legacyDifficultyTag = value;
    }

    [Serializable]
    [JsonObject(MemberSerialization.OptIn)]
    public sealed class MatrixSavePayloadV2
    {
        [JsonProperty("version")]
        public int version = 2;

        [JsonProperty("migratedFromVersion")]
        public int migratedFromVersion;

        [JsonProperty("heroName")]
        public string heroName = string.Empty;

        [JsonProperty("currencyGold")]
        public int currencyGold;

        [JsonProperty("worldCoord")]
        public MatrixSaveCoordStruct worldCoord;

        [SerializeField]
        [JsonProperty("_difficultyTier")]
        private int _difficultyTier = 1;

        [NonSerialized]
        [XmlIgnore]
        public string transientMigrationScratch = "unpersisted";

        [JsonProperty("inventory")]
        public List<MatrixSaveInventoryEntry> inventory = new List<MatrixSaveInventoryEntry>();

        [XmlElement("_difficultyTier")]
        public int XmlDifficultyTier
        {
            get => _difficultyTier;
            set => _difficultyTier = value;
        }

        public int GetDifficultyTier() => _difficultyTier;

        public void SetDifficultyTier(int value) => _difficultyTier = value;
    }

    public static class SerializerMatrixTests
    {
        private static bool _registered;

        public static void Run()
        {
            EnsureRegistered();
            TestRoundTripSaveEnvelopeAcrossJsonUtilityNewtonsoftJsonAndXmlSerializer();
            TestVersionMigrationAcrossNewtonsoftJsonAndXmlSerializer();
            TestSerializerCompatibilityMatrixAndBinaryFormatterReporting();
            TestStrippedTypeAndAotFailureDiagnosticsNameTypeAndMember();
        }

        public static void RunAll() => Run();

        private static T? DeserializeXmlSafely<T>(XmlSerializer serializer, string xmlText) where T : class
        {
            XmlReaderSettings readerSettings = new XmlReaderSettings
            {
                DtdProcessing = DtdProcessing.Prohibit,
                XmlResolver = null
            };
            using (StringReader sr = new StringReader(xmlText))
            using (XmlReader xr = XmlReader.Create(sr, readerSettings))
            {
                return serializer.Deserialize(xr) as T;
            }
        }

        private static void EnsureRegistered()
        {
            if (_registered)
            {
                return;
            }

            _registered = true;

            JsonUtility.RegisterType<MatrixSaveCoordStruct>(
                () => new MatrixSaveCoordStruct(),
                (val, w) =>
                {
                    w.WriteInt("gridX", val.gridX);
                    w.WriteInt("gridY", val.gridY);
                    w.WriteFloat("_elevation", val.GetElevation());
                },
                (ref MatrixSaveCoordStruct target, JsonParseNode n) =>
                {
                    target.gridX = n.ReadInt("gridX", target.gridX);
                    target.gridY = n.ReadInt("gridY", target.gridY);
                    target.SetElevation(n.ReadFloat("_elevation", target.GetElevation()));
                });

            JsonUtility.RegisterType<MatrixSaveInventoryEntry>(
                () => new MatrixSaveInventoryEntry(),
                (val, w) =>
                {
                    w.WriteString("itemId", val.itemId);
                    w.WriteInt("quantity", val.quantity);
                    w.WriteInt("_durability", val.GetDurability());
                },
                (ref MatrixSaveInventoryEntry target, JsonParseNode n) =>
                {
                    target.itemId = n.ReadString("itemId", target.itemId);
                    target.quantity = n.ReadInt("quantity", target.quantity);
                    target.SetDurability(n.ReadInt("_durability", target.GetDurability()));
                });

            JsonUtility.RegisterType<MatrixSaveEnvelope>(
                () => new MatrixSaveEnvelope(),
                (val, w) =>
                {
                    w.WriteString("profileId", val.profileId);
                    w.WriteInt("schemaVersion", val.schemaVersion);
                    w.WriteFloat("playTimeSeconds", val.playTimeSeconds);
                    w.WriteBool("hardcoreMode", val.hardcoreMode);
                    w.WriteString("_checkpointToken", val.GetCheckpointToken());
                    w.WriteInt("_secretSeed", val.GetSecretSeed());
                    w.WriteNested("lastCheckpointCoord", val.lastCheckpointCoord);
                    w.WriteNested("primaryWeapon", val.primaryWeapon);
                    w.WriteIntList("unlockedChapters", val.unlockedChapters);
                    w.WriteStringList("activeModifiers", val.activeModifiers);
                    w.WriteIntList("completedMilestones", val.completedMilestones);
                    w.WriteNestedList("backpackItems", val.backpackItems);
                },
                (ref MatrixSaveEnvelope target, JsonParseNode n) =>
                {
                    target.profileId = n.ReadString("profileId", target.profileId);
                    target.schemaVersion = n.ReadInt("schemaVersion", target.schemaVersion);
                    target.playTimeSeconds = n.ReadFloat("playTimeSeconds", target.playTimeSeconds);
                    target.hardcoreMode = n.ReadBool("hardcoreMode", target.hardcoreMode);
                    target.SetCheckpointToken(n.ReadString("_checkpointToken", target.GetCheckpointToken()));
                    target.SetSecretSeed(n.ReadInt("_secretSeed", target.GetSecretSeed()));
                    target.lastCheckpointCoord = n.ReadNested("lastCheckpointCoord", target.lastCheckpointCoord);
                    target.primaryWeapon = n.ReadNested("primaryWeapon", target.primaryWeapon);
                    target.unlockedChapters = n.ReadIntArray("unlockedChapters", target.unlockedChapters);
                    target.activeModifiers = n.ReadStringArray("activeModifiers", target.activeModifiers);
                    target.completedMilestones = n.ReadIntList("completedMilestones", target.completedMilestones);
                    target.backpackItems = n.ReadNestedList("backpackItems", target.backpackItems);
                });
        }

        private static MatrixSaveEnvelope CreateSampleSaveEnvelope()
        {
            MatrixSaveCoordStruct coord = new MatrixSaveCoordStruct
            {
                gridX = 18,
                gridY = -11
            };
            coord.SetElevation(64.25f);

            MatrixSaveInventoryEntry sword = new MatrixSaveInventoryEntry
            {
                itemId = "astral_claymore",
                quantity = 1,
                cachedTooltipHash = 33333
            };
            sword.SetDurability(88);

            MatrixSaveInventoryEntry elixir = new MatrixSaveInventoryEntry
            {
                itemId = "phoenix_tonic",
                quantity = 4,
                cachedTooltipHash = 44444
            };
            elixir.SetDurability(100);

            MatrixSaveInventoryEntry rune = new MatrixSaveInventoryEntry
            {
                itemId = "void_compass",
                quantity = 2,
                cachedTooltipHash = 55555
            };
            rune.SetDurability(45);

            MatrixSaveEnvelope envelope = new MatrixSaveEnvelope
            {
                profileId = "matrix_slot_01",
                schemaVersion = 2,
                playTimeSeconds = 5412.75f,
                hardcoreMode = true,
                runtimeSessionNote = "do_not_persist_me",
                lastCheckpointCoord = coord,
                primaryWeapon = sword,
                unlockedChapters = new[] { 1, 2, 4, 7 },
                activeModifiers = new[] { "no_hud", "perm_death" }
            };
            envelope.SetCheckpointToken("citadel_obelisk_09");
            envelope.SetSecretSeed(7654321);
            envelope.SetUnserializedPrivateCounter(999);
            envelope.completedMilestones.Add(100);
            envelope.completedMilestones.Add(250);
            envelope.completedMilestones.Add(500);
            envelope.backpackItems.Add(elixir);
            envelope.backpackItems.Add(rune);
            return envelope;
        }

        private static void AssertSaveEnvelopeRoundTripped(string serializerLabel, MatrixSaveEnvelope? loaded)
        {
            Check.That(serializerLabel + ": deserialized instance is non-null", loaded != null);
            if (loaded == null)
            {
                return;
            }

            Check.Equal(serializerLabel + ": public profileId", "matrix_slot_01", loaded.profileId);
            Check.Equal(serializerLabel + ": public schemaVersion", 2, loaded.schemaVersion);
            Check.Equal(serializerLabel + ": public playTimeSeconds", 5412.75f, loaded.playTimeSeconds);
            Check.Equal(serializerLabel + ": public hardcoreMode", true, loaded.hardcoreMode);
            Check.Equal(serializerLabel + ": private [SerializeField] _checkpointToken", "citadel_obelisk_09", loaded.GetCheckpointToken());
            Check.Equal(serializerLabel + ": private [SerializeField] _secretSeed", 7654321, loaded.GetSecretSeed());
            Check.Equal(serializerLabel + ": [NonSerialized] runtimeSessionNote keeps default", "transient_only", loaded.runtimeSessionNote);
            Check.Equal(serializerLabel + ": unmarked private field keeps default", 19, loaded.GetUnserializedPrivateCounter());

            Check.Equal(serializerLabel + ": nested struct gridX", 18, loaded.lastCheckpointCoord.gridX);
            Check.Equal(serializerLabel + ": nested struct gridY", -11, loaded.lastCheckpointCoord.gridY);
            Check.Equal(serializerLabel + ": nested struct private _elevation", 64.25f, loaded.lastCheckpointCoord.GetElevation());

            Check.Equal(serializerLabel + ": nested class itemId", "astral_claymore", loaded.primaryWeapon.itemId);
            Check.Equal(serializerLabel + ": nested class quantity", 1, loaded.primaryWeapon.quantity);
            Check.Equal(serializerLabel + ": nested class private _durability", 88, loaded.primaryWeapon.GetDurability());
            Check.Equal(serializerLabel + ": nested class [NonSerialized] cachedTooltipHash keeps default", 77, loaded.primaryWeapon.cachedTooltipHash);

            Check.Equal(serializerLabel + ": int[] unlockedChapters length", 4, loaded.unlockedChapters.Length);
            Check.Equal(serializerLabel + ": int[] unlockedChapters[2]", 4, loaded.unlockedChapters[2]);
            Check.Equal(serializerLabel + ": int[] unlockedChapters[3]", 7, loaded.unlockedChapters[3]);

            Check.Equal(serializerLabel + ": string[] activeModifiers length", 2, loaded.activeModifiers.Length);
            Check.Equal(serializerLabel + ": string[] activeModifiers[0]", "no_hud", loaded.activeModifiers[0]);
            Check.Equal(serializerLabel + ": string[] activeModifiers[1]", "perm_death", loaded.activeModifiers[1]);

            Check.Equal(serializerLabel + ": List<int> completedMilestones count", 3, loaded.completedMilestones.Count);
            Check.Equal(serializerLabel + ": List<int> completedMilestones[1]", 250, loaded.completedMilestones[1]);
            Check.Equal(serializerLabel + ": List<int> completedMilestones[2]", 500, loaded.completedMilestones[2]);

            Check.Equal(serializerLabel + ": List<MatrixSaveInventoryEntry> backpackItems count", 2, loaded.backpackItems.Count);
            Check.Equal(serializerLabel + ": backpackItems[0] itemId", "phoenix_tonic", loaded.backpackItems[0].itemId);
            Check.Equal(serializerLabel + ": backpackItems[0] quantity", 4, loaded.backpackItems[0].quantity);
            Check.Equal(serializerLabel + ": backpackItems[0] private _durability", 100, loaded.backpackItems[0].GetDurability());
            Check.Equal(serializerLabel + ": backpackItems[1] itemId", "void_compass", loaded.backpackItems[1].itemId);
            Check.Equal(serializerLabel + ": backpackItems[1] quantity", 2, loaded.backpackItems[1].quantity);
            Check.Equal(serializerLabel + ": backpackItems[1] private _durability", 45, loaded.backpackItems[1].GetDurability());
        }

        private static void TestRoundTripSaveEnvelopeAcrossJsonUtilityNewtonsoftJsonAndXmlSerializer()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_sav001_matrix_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(tempDir);
            string? savedAppOverride = Application.PersistentDataPathOverride;
            Application.PersistentDataPathOverride = tempDir;
            try
            {
                MatrixSaveEnvelope source = CreateSampleSaveEnvelope();

                string jsonUtilityPath = Path.Combine(Application.persistentDataPath, "save_jsonutility.json");
                string jsonUtilityText = JsonUtility.ToJson(source, true);
                File.WriteAllText(jsonUtilityPath, jsonUtilityText, new UTF8Encoding(false));
                Check.That("JsonUtility save payload excludes [NonSerialized] runtimeSessionNote", !jsonUtilityText.Contains("runtimeSessionNote", StringComparison.Ordinal));
                Check.That("JsonUtility save payload excludes unmarked private field", !jsonUtilityText.Contains("_unserializedPrivateCounter", StringComparison.Ordinal));
                MatrixSaveEnvelope? fromJsonUtility = JsonUtility.FromJson<MatrixSaveEnvelope>(File.ReadAllText(jsonUtilityPath, Encoding.UTF8));
                AssertSaveEnvelopeRoundTripped("JsonUtility", fromJsonUtility);

                string newtonsoftPath = Path.Combine(Application.persistentDataPath, "save_newtonsoft.json");
                JsonSerializerSettings newtonsoftSettings = new JsonSerializerSettings
                {
                    Culture = CultureInfo.InvariantCulture,
                    Formatting = Newtonsoft.Json.Formatting.Indented
                };
                string newtonsoftText = JsonConvert.SerializeObject(source, newtonsoftSettings);
                File.WriteAllText(newtonsoftPath, newtonsoftText, new UTF8Encoding(false));
                Check.That("Newtonsoft.Json payload includes private [SerializeField] _checkpointToken", newtonsoftText.Contains("\"_checkpointToken\"", StringComparison.Ordinal));
                Check.That("Newtonsoft.Json payload excludes [NonSerialized] runtimeSessionNote", !newtonsoftText.Contains("runtimeSessionNote", StringComparison.Ordinal));
                Check.That("Newtonsoft.Json payload excludes unmarked private field", !newtonsoftText.Contains("_unserializedPrivateCounter", StringComparison.Ordinal));
                MatrixSaveEnvelope? fromNewtonsoft = JsonConvert.DeserializeObject<MatrixSaveEnvelope>(
                    File.ReadAllText(newtonsoftPath, Encoding.UTF8),
                    newtonsoftSettings);
                AssertSaveEnvelopeRoundTripped("Newtonsoft.Json", fromNewtonsoft);

                string xmlPath = Path.Combine(Application.persistentDataPath, "save_xmlserializer.xml");
                XmlSerializer xmlSerializer = new XmlSerializer(typeof(MatrixSaveEnvelope));
                string xmlText;
                using (StringWriter sw = new StringWriter(CultureInfo.InvariantCulture))
                {
                    xmlSerializer.Serialize(sw, source);
                    xmlText = sw.ToString();
                }

                File.WriteAllText(xmlPath, xmlText, new UTF8Encoding(false));
                Check.That("XmlSerializer payload includes _checkpointToken element", xmlText.Contains("<_checkpointToken>citadel_obelisk_09</_checkpointToken>", StringComparison.Ordinal));
                Check.That("XmlSerializer payload excludes [NonSerialized] runtimeSessionNote", !xmlText.Contains("runtimeSessionNote", StringComparison.Ordinal));
                MatrixSaveEnvelope? fromXml = DeserializeXmlSafely<MatrixSaveEnvelope>(
                    xmlSerializer,
                    File.ReadAllText(xmlPath, Encoding.UTF8));

                AssertSaveEnvelopeRoundTripped("XmlSerializer", fromXml);
            }
            finally
            {
                Application.PersistentDataPathOverride = savedAppOverride;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestVersionMigrationAcrossNewtonsoftJsonAndXmlSerializer()
        {
            MatrixSavePayloadV1 legacyV1 = new MatrixSavePayloadV1
            {
                version = 1,
                heroName = "Seraphina",
                gold = 820,
                legacyStageIndex = 4,
                unlockedItemIds = new[] { "ember_lantern", "star_map", "aegis_ring" }
            };
            legacyV1.SetLegacyDifficultyTag("veteran");

            string v1Newtonsoft = JsonConvert.SerializeObject(legacyV1);
            MatrixSavePayloadV1? parsedNewtonsoftV1 = JsonConvert.DeserializeObject<MatrixSavePayloadV1>(v1Newtonsoft);
            Check.That("Newtonsoft.Json deserializes V1 save for migration", parsedNewtonsoftV1 != null);
            MatrixSavePayloadV2 migratedFromNewtonsoft = MigrateV1ToV2(parsedNewtonsoftV1!);
            string v2Newtonsoft = JsonConvert.SerializeObject(migratedFromNewtonsoft);
            MatrixSavePayloadV2? loadedNewtonsoftV2 = JsonConvert.DeserializeObject<MatrixSavePayloadV2>(v2Newtonsoft);
            AssertMigratedV2("Newtonsoft.Json migration", loadedNewtonsoftV2);

            XmlSerializer v1XmlSerializer = new XmlSerializer(typeof(MatrixSavePayloadV1));
            XmlSerializer v2XmlSerializer = new XmlSerializer(typeof(MatrixSavePayloadV2));
            string v1Xml;
            using (StringWriter sw = new StringWriter(CultureInfo.InvariantCulture))
            {
                v1XmlSerializer.Serialize(sw, legacyV1);
                v1Xml = sw.ToString();
            }

            MatrixSavePayloadV1? parsedXmlV1 = DeserializeXmlSafely<MatrixSavePayloadV1>(v1XmlSerializer, v1Xml);
            Check.That("XmlSerializer deserializes V1 save for migration", parsedXmlV1 != null);
            MatrixSavePayloadV2 migratedFromXml = MigrateV1ToV2(parsedXmlV1!);
            string v2Xml;
            using (StringWriter sw = new StringWriter(CultureInfo.InvariantCulture))
            {
                v2XmlSerializer.Serialize(sw, migratedFromXml);
                v2Xml = sw.ToString();
            }

            MatrixSavePayloadV2? loadedXmlV2 = DeserializeXmlSafely<MatrixSavePayloadV2>(v2XmlSerializer, v2Xml);
            AssertMigratedV2("XmlSerializer migration", loadedXmlV2);
        }

        private static MatrixSavePayloadV2 MigrateV1ToV2(MatrixSavePayloadV1 v1)
        {
            MatrixSaveCoordStruct coord = new MatrixSaveCoordStruct
            {
                gridX = v1.legacyStageIndex * 10,
                gridY = v1.legacyStageIndex * 3
            };
            coord.SetElevation(v1.legacyStageIndex * 1.5f);

            int difficultyTier = string.Equals(v1.GetLegacyDifficultyTag(), "veteran", StringComparison.Ordinal) ? 2 : 1;
            MatrixSavePayloadV2 v2 = new MatrixSavePayloadV2
            {
                version = 2,
                migratedFromVersion = v1.version,
                heroName = v1.heroName,
                currencyGold = v1.gold,
                worldCoord = coord
            };
            v2.SetDifficultyTier(difficultyTier);

            for (int i = 0; i < v1.unlockedItemIds.Length; i++)
            {
                MatrixSaveInventoryEntry entry = new MatrixSaveInventoryEntry
                {
                    itemId = v1.unlockedItemIds[i],
                    quantity = 1
                };
                entry.SetDurability(100);
                v2.inventory.Add(entry);
            }

            return v2;
        }

        private static void AssertMigratedV2(string label, MatrixSavePayloadV2? loadedV2)
        {
            Check.That(label + ": V2 payload is non-null", loadedV2 != null);
            if (loadedV2 == null)
            {
                return;
            }

            Check.Equal(label + ": version is 2", 2, loadedV2.version);
            Check.Equal(label + ": migratedFromVersion is 1", 1, loadedV2.migratedFromVersion);
            Check.Equal(label + ": heroName preserved", "Seraphina", loadedV2.heroName);
            Check.Equal(label + ": gold mapped to currencyGold", 820, loadedV2.currencyGold);
            Check.Equal(label + ": worldCoord.gridX", 40, loadedV2.worldCoord.gridX);
            Check.Equal(label + ": worldCoord.gridY", 12, loadedV2.worldCoord.gridY);
            Check.Equal(label + ": worldCoord._elevation", 6.0f, loadedV2.worldCoord.GetElevation());
            Check.Equal(label + ": veteran difficulty mapped to _difficultyTier 2", 2, loadedV2.GetDifficultyTier());
            Check.Equal(label + ": inventory count is 3", 3, loadedV2.inventory.Count);
            Check.Equal(label + ": inventory[0] itemId", "ember_lantern", loadedV2.inventory[0].itemId);
            Check.Equal(label + ": inventory[2] itemId", "aegis_ring", loadedV2.inventory[2].itemId);
            Check.Equal(label + ": inventory[2] durability", 100, loadedV2.inventory[2].GetDurability());
        }

        private static void TestSerializerCompatibilityMatrixAndBinaryFormatterReporting()
        {
            Check.That(
                "Host runtime description identifies .NET Desktop Mono/JIT host",
                SaveSerializationSupport.HostRuntimeDescription.Contains(".NET", StringComparison.Ordinal));

            SaveTargetFamily[] allTargets = new[]
            {
                SaveTargetFamily.Desktop,
                SaveTargetFamily.Mobile,
                SaveTargetFamily.Browser
            };
            SaveScriptingBackend[] allBackends = new[]
            {
                SaveScriptingBackend.Mono,
                SaveScriptingBackend.Il2CppAot
            };

            for (int t = 0; t < allTargets.Length; t++)
            {
                SaveTargetFamily target = allTargets[t];
                for (int b = 0; b < allBackends.Length; b++)
                {
                    SaveScriptingBackend backend = allBackends[b];
                    if (target == SaveTargetFamily.Browser && backend == SaveScriptingBackend.Mono)
                    {
                        continue;
                    }

                    SerializerCompatibilityAssessment jsonUtility = SaveSerializationSupport.Assess(
                        SaveSerializerKind.JsonUtility,
                        target,
                        backend);
                    Check.Equal("JsonUtility is Supported on " + target + "/" + backend, (int)SaveCompatibilityState.Supported, (int)jsonUtility.State);
                    Check.Equal("JsonUtility is available on " + target + "/" + backend, true, jsonUtility.IsAvailableOnTarget);
                    Check.Equal("JsonUtility feature ID is SAV-001", "SAV-001", jsonUtility.FeatureId);

                    SerializerCompatibilityAssessment newtonsoft = SaveSerializationSupport.Assess(
                        SaveSerializerKind.NewtonsoftJson,
                        target,
                        backend);
                    Check.Equal("Newtonsoft.Json is Supported on " + target + "/" + backend, (int)SaveCompatibilityState.Supported, (int)newtonsoft.State);
                    Check.Equal("Newtonsoft.Json is available on " + target + "/" + backend, true, newtonsoft.IsAvailableOnTarget);
                    Check.Equal("Newtonsoft.Json package ID matches CMP-002 matrix", "com.unity.nuget.newtonsoft-json", newtonsoft.PackageId);
                    Check.Equal("Newtonsoft.Json version line matches CMP-002 matrix", "3.x", newtonsoft.PackageVersionLine);

                    SerializerCompatibilityAssessment xml = SaveSerializationSupport.Assess(
                        SaveSerializerKind.XmlSerializer,
                        target,
                        backend);
                    Check.Equal("XmlSerializer is Supported on " + target + "/" + backend, (int)SaveCompatibilityState.Supported, (int)xml.State);
                    Check.Equal("XmlSerializer is available on " + target + "/" + backend, true, xml.IsAvailableOnTarget);
                    Check.Equal(
                        "XmlSerializer reports BLD-003 AOT restrictions on Il2CppAot",
                        backend == SaveScriptingBackend.Il2CppAot,
                        xml.HasAotRestrictions);
                    if (backend == SaveScriptingBackend.Il2CppAot)
                    {
                        Check.That(
                            "XmlSerializer AOT guidance cites BLD-003",
                            xml.DiagnosticGuidance.Contains("BLD-003", StringComparison.Ordinal));
                    }
                }
            }

            SerializerCompatibilityAssessment binaryDesktopMono = SaveSerializationSupport.Assess(
                SaveSerializerKind.BinaryFormatter,
                SaveTargetFamily.Desktop,
                SaveScriptingBackend.Mono);
            Check.Equal(
                "BinaryFormatter on Desktop/Mono is PartiallySupported",
                (int)SaveCompatibilityState.PartiallySupported,
                (int)binaryDesktopMono.State);
            Check.Equal("BinaryFormatter on Desktop/Mono is available", true, binaryDesktopMono.IsAvailableOnTarget);
            Check.That(
                "BinaryFormatter on Desktop/Mono names Browser unavailability and BLD-003 limits",
                binaryDesktopMono.DiagnosticGuidance.Contains("Browser", StringComparison.Ordinal)
                && binaryDesktopMono.DiagnosticGuidance.Contains("BLD-003", StringComparison.Ordinal));
            Check.That("BinaryFormatter on Desktop/Mono lists non-empty exclusions", binaryDesktopMono.Exclusions.Count >= 2);

            SerializerCompatibilityAssessment binaryMobileAot = SaveSerializationSupport.Assess(
                SaveSerializerKind.BinaryFormatter,
                SaveTargetFamily.Mobile,
                SaveScriptingBackend.Il2CppAot);
            Check.Equal(
                "BinaryFormatter on Mobile/Il2CppAot is PartiallySupported",
                (int)SaveCompatibilityState.PartiallySupported,
                (int)binaryMobileAot.State);
            Check.Equal("BinaryFormatter on Mobile/Il2CppAot reports AOT restrictions", true, binaryMobileAot.HasAotRestrictions);

            SerializerCompatibilityAssessment binaryBrowser = SaveSerializationSupport.Assess(
                SaveSerializerKind.BinaryFormatter,
                SaveTargetFamily.Browser,
                SaveScriptingBackend.Il2CppAot);
            Check.Equal(
                "BinaryFormatter on Browser target is Unsupported (unavailable)",
                (int)SaveCompatibilityState.Unsupported,
                (int)binaryBrowser.State);
            Check.Equal("BinaryFormatter on Browser target IsAvailableOnTarget is false", false, binaryBrowser.IsAvailableOnTarget);
            Check.That(
                "BinaryFormatter Browser guidance recommends supported save serializers",
                binaryBrowser.DiagnosticGuidance.Contains("JsonUtility", StringComparison.Ordinal)
                && binaryBrowser.DiagnosticGuidance.Contains("Newtonsoft.Json", StringComparison.Ordinal)
                && binaryBrowser.DiagnosticGuidance.Contains("XmlSerializer", StringComparison.Ordinal));
        }

        private static void TestStrippedTypeAndAotFailureDiagnosticsNameTypeAndMember()
        {
            LogRecorder.Clear();
            StrippedTypeDiagnostic diag = SaveSerializationSupport.DiagnoseStrippedMemberFailure(
                SaveSerializerKind.XmlSerializer,
                "MyGame.Saves.LegacyEquipmentRecord",
                "_enchantmentIds",
                "High");

            Check.Equal("Stripped type diagnostic records type name", "MyGame.Saves.LegacyEquipmentRecord", diag.StrippedTypeName);
            Check.Equal("Stripped type diagnostic records member name", "_enchantmentIds", diag.StrippedMemberName);
            Check.Equal("Stripped type diagnostic records stripping level", "High", diag.StrippingLevel);
            Check.That(
                "Stripped type diagnostic message names type, member, stripping level, and link.xml",
                diag.Message.Contains("MyGame.Saves.LegacyEquipmentRecord", StringComparison.Ordinal)
                && diag.Message.Contains("_enchantmentIds", StringComparison.Ordinal)
                && diag.Message.Contains("High", StringComparison.Ordinal)
                && diag.Message.Contains("link.xml", StringComparison.Ordinal));
            Check.That("Stripped type diagnostic lists preservation mechanisms", diag.PreservationMechanisms.Count == 4);

            IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
            Check.Equal("DiagnoseStrippedMemberFailure logs one error diagnostic", 1, errors.Count);
            Check.That(
                "Logged error diagnostic names stripped type and member",
                errors.Count == 1
                && errors[0].Message.Contains("MyGame.Saves.LegacyEquipmentRecord", StringComparison.Ordinal)
                && errors[0].Message.Contains("_enchantmentIds", StringComparison.Ordinal));
        }
    }
}
