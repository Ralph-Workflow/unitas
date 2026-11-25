using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    [Serializable]
    public struct SaveCoordStruct
    {
        public int gridX;
        public int gridY;
        [SerializeField]
        private float _elevation;

        public float GetElevation() => _elevation;

        public void SetElevation(float value) => _elevation = value;
    }

    [Serializable]
    public sealed class SaveInventoryEntry
    {
        public string itemId = string.Empty;
        public int quantity;
        [SerializeField]
        private int _durability = 100;
        [NonSerialized]
        public int cachedTooltipHash = 77;

        public int GetDurability() => _durability;

        public void SetDurability(int value) => _durability = value;
    }

    [Serializable]
    public sealed class SaveGameEnvelopeRoundTrip
    {
        public string profileId = string.Empty;
        public int schemaVersion = 2;
        public float playTimeSeconds;
        public bool hardcoreMode;
        [SerializeField]
        private string _checkpointToken = "init_token";
        [SerializeField]
        private int _secretSeed = 42;
        private int _unserializedPrivateCounter = 19;
        [NonSerialized]
        public string runtimeSessionNote = "transient_only";
        public SaveCoordStruct lastCheckpointCoord;
        public SaveInventoryEntry primaryWeapon = new SaveInventoryEntry();
        public int[] unlockedChapters = Array.Empty<int>();
        public string[] activeModifiers = Array.Empty<string>();
        public List<int> completedMilestones = new List<int>();
        public List<SaveInventoryEntry> backpackItems = new List<SaveInventoryEntry>();

        public string GetCheckpointToken() => _checkpointToken;

        public void SetCheckpointToken(string value) => _checkpointToken = value;

        public int GetSecretSeed() => _secretSeed;

        public void SetSecretSeed(int value) => _secretSeed = value;

        public int GetUnserializedPrivateCounter() => _unserializedPrivateCounter;

        public void SetUnserializedPrivateCounter(int value) => _unserializedPrivateCounter = value;
    }

    [Serializable]
    public sealed class SaveVersionProbe
    {
        public int version;
    }

    [Serializable]
    public sealed class SavePayloadV1
    {
        public int version = 1;
        public string heroName = string.Empty;
        public int gold;
        public int legacyStageIndex;
        [SerializeField]
        private string _legacyDifficultyTag = "normal";
        public string[] unlockedItemIds = Array.Empty<string>();

        public string GetLegacyDifficultyTag() => _legacyDifficultyTag;

        public void SetLegacyDifficultyTag(string value) => _legacyDifficultyTag = value;
    }

    [Serializable]
    public sealed class SavePayloadV2
    {
        public int version = 2;
        public int migratedFromVersion;
        public string heroName = string.Empty;
        public int currencyGold;
        public SaveCoordStruct worldCoord;
        [SerializeField]
        private int _difficultyTier = 1;
        [NonSerialized]
        public string transientMigrationScratch = "unpersisted";
        public List<SaveInventoryEntry> inventory = new List<SaveInventoryEntry>();

        public int GetDifficultyTier() => _difficultyTier;

        public void SetDifficultyTier(int value) => _difficultyTier = value;
    }

    public sealed class SaveMigrationReport
    {
        public int SourceVersion { get; set; }

        public int TargetVersion { get; set; }

        public string Outcome { get; set; } = string.Empty;

        public string PersistedPath { get; set; } = string.Empty;
    }

    public static class JsonUtilityRoundTripTests
    {
        private static bool _registered;

        public static void Run()
        {
            EnsureRegistered();
            TestRoundTripPublicPrivateNonSerializedNestedClassesStructsArraysAndGenericLists();
            TestFromJsonOverwritePreservesReferenceAndUpdatesSelectiveFields();
            TestSaveDataVersionMigrationFromV1ToV2WithAtomicPersistence();
        }

        public static void RunAll() => Run();

        private static void EnsureRegistered()
        {
            if (_registered)
            {
                return;
            }

            _registered = true;

            JsonUtility.RegisterType<SaveCoordStruct>(
                () => new SaveCoordStruct(),
                (val, w) =>
                {
                    w.WriteInt("gridX", val.gridX);
                    w.WriteInt("gridY", val.gridY);
                    w.WriteFloat("_elevation", val.GetElevation());
                },
                (ref SaveCoordStruct target, JsonParseNode n) =>
                {
                    target.gridX = n.ReadInt("gridX", target.gridX);
                    target.gridY = n.ReadInt("gridY", target.gridY);
                    target.SetElevation(n.ReadFloat("_elevation", target.GetElevation()));
                });

            JsonUtility.RegisterType<SaveInventoryEntry>(
                () => new SaveInventoryEntry(),
                (val, w) =>
                {
                    w.WriteString("itemId", val.itemId);
                    w.WriteInt("quantity", val.quantity);
                    w.WriteInt("_durability", val.GetDurability());
                },
                (ref SaveInventoryEntry target, JsonParseNode n) =>
                {
                    target.itemId = n.ReadString("itemId", target.itemId);
                    target.quantity = n.ReadInt("quantity", target.quantity);
                    target.SetDurability(n.ReadInt("_durability", target.GetDurability()));
                });

            JsonUtility.RegisterType<SaveGameEnvelopeRoundTrip>(
                () => new SaveGameEnvelopeRoundTrip(),
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
                (ref SaveGameEnvelopeRoundTrip target, JsonParseNode n) =>
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

            JsonUtility.RegisterType<SaveVersionProbe>(
                () => new SaveVersionProbe(),
                (val, w) =>
                {
                    w.WriteInt("version", val.version);
                },
                (ref SaveVersionProbe target, JsonParseNode n) =>
                {
                    target.version = n.ReadInt("version", target.version);
                });

            JsonUtility.RegisterType<SavePayloadV1>(
                () => new SavePayloadV1(),
                (val, w) =>
                {
                    w.WriteInt("version", val.version);
                    w.WriteString("heroName", val.heroName);
                    w.WriteInt("gold", val.gold);
                    w.WriteInt("legacyStageIndex", val.legacyStageIndex);
                    w.WriteString("_legacyDifficultyTag", val.GetLegacyDifficultyTag());
                    w.WriteStringList("unlockedItemIds", val.unlockedItemIds);
                },
                (ref SavePayloadV1 target, JsonParseNode n) =>
                {
                    target.version = n.ReadInt("version", target.version);
                    target.heroName = n.ReadString("heroName", target.heroName);
                    target.gold = n.ReadInt("gold", target.gold);
                    target.legacyStageIndex = n.ReadInt("legacyStageIndex", target.legacyStageIndex);
                    target.SetLegacyDifficultyTag(n.ReadString("_legacyDifficultyTag", target.GetLegacyDifficultyTag()));
                    target.unlockedItemIds = n.ReadStringArray("unlockedItemIds", target.unlockedItemIds);
                });

            JsonUtility.RegisterType<SavePayloadV2>(
                () => new SavePayloadV2(),
                (val, w) =>
                {
                    w.WriteInt("version", val.version);
                    w.WriteInt("migratedFromVersion", val.migratedFromVersion);
                    w.WriteString("heroName", val.heroName);
                    w.WriteInt("currencyGold", val.currencyGold);
                    w.WriteNested("worldCoord", val.worldCoord);
                    w.WriteInt("_difficultyTier", val.GetDifficultyTier());
                    w.WriteNestedList("inventory", val.inventory);
                },
                (ref SavePayloadV2 target, JsonParseNode n) =>
                {
                    target.version = n.ReadInt("version", target.version);
                    target.migratedFromVersion = n.ReadInt("migratedFromVersion", target.migratedFromVersion);
                    target.heroName = n.ReadString("heroName", target.heroName);
                    target.currencyGold = n.ReadInt("currencyGold", target.currencyGold);
                    target.worldCoord = n.ReadNested("worldCoord", target.worldCoord);
                    target.SetDifficultyTier(n.ReadInt("_difficultyTier", target.GetDifficultyTier()));
                    target.inventory = n.ReadNestedList("inventory", target.inventory);
                });
        }

        private static void TestRoundTripPublicPrivateNonSerializedNestedClassesStructsArraysAndGenericLists()
        {
            SaveCoordStruct coord = new SaveCoordStruct
            {
                gridX = 12,
                gridY = -7
            };
            coord.SetElevation(48.5f);

            SaveInventoryEntry sword = new SaveInventoryEntry
            {
                itemId = "relic_blade",
                quantity = 1,
                cachedTooltipHash = 12345
            };
            sword.SetDurability(92);

            SaveInventoryEntry potion = new SaveInventoryEntry
            {
                itemId = "healing_draught",
                quantity = 5,
                cachedTooltipHash = 8888
            };
            potion.SetDurability(100);

            SaveInventoryEntry keyItem = new SaveInventoryEntry
            {
                itemId = "vault_sigil",
                quantity = 2,
                cachedTooltipHash = 9999
            };
            keyItem.SetDurability(50);

            SaveGameEnvelopeRoundTrip source = new SaveGameEnvelopeRoundTrip
            {
                profileId = "slot_alpha",
                schemaVersion = 2,
                playTimeSeconds = 3725.25f,
                hardcoreMode = true,
                runtimeSessionNote = "should_not_serialize",
                lastCheckpointCoord = coord,
                primaryWeapon = sword,
                unlockedChapters = new[] { 1, 2, 3, 5 },
                activeModifiers = new[] { "ironman", "double_xp" }
            };
            source.SetCheckpointToken("sanctuary_gate_04");
            source.SetSecretSeed(987654);
            source.SetUnserializedPrivateCounter(555);
            source.completedMilestones.Add(10);
            source.completedMilestones.Add(20);
            source.completedMilestones.Add(35);
            source.backpackItems.Add(potion);
            source.backpackItems.Add(keyItem);

            string json = JsonUtility.ToJson(source);
            Check.That("Serialized save envelope JSON is non-empty", !string.IsNullOrEmpty(json));
            Check.That("JSON includes public field profileId", json.Contains("\"profileId\":\"slot_alpha\"", StringComparison.Ordinal));
            Check.That("JSON includes private [SerializeField] _checkpointToken", json.Contains("\"_checkpointToken\":\"sanctuary_gate_04\"", StringComparison.Ordinal));
            Check.That("JSON includes private [SerializeField] _secretSeed", json.Contains("\"_secretSeed\":987654", StringComparison.Ordinal));
            Check.That("JSON excludes [NonSerialized] runtimeSessionNote", !json.Contains("runtimeSessionNote", StringComparison.Ordinal));
            Check.That("JSON excludes [NonSerialized] cachedTooltipHash", !json.Contains("cachedTooltipHash", StringComparison.Ordinal));
            Check.That("JSON excludes unmarked private field _unserializedPrivateCounter", !json.Contains("_unserializedPrivateCounter", StringComparison.Ordinal));

            SaveGameEnvelopeRoundTrip? loaded = JsonUtility.FromJson<SaveGameEnvelopeRoundTrip>(json);
            Check.That("FromJson deserializes SaveGameEnvelopeRoundTrip instance", loaded != null);
            if (loaded != null)
            {
                Check.Equal("Round-trip public profileId", "slot_alpha", loaded.profileId);
                Check.Equal("Round-trip public schemaVersion", 2, loaded.schemaVersion);
                Check.Equal("Round-trip public playTimeSeconds", 3725.25f, loaded.playTimeSeconds);
                Check.Equal("Round-trip public hardcoreMode", true, loaded.hardcoreMode);
                Check.Equal("Round-trip private [SerializeField] _checkpointToken", "sanctuary_gate_04", loaded.GetCheckpointToken());
                Check.Equal("Round-trip private [SerializeField] _secretSeed", 987654, loaded.GetSecretSeed());
                Check.Equal("[NonSerialized] field keeps default value on deserialized object", "transient_only", loaded.runtimeSessionNote);
                Check.Equal("Unmarked private field keeps default value on deserialized object", 19, loaded.GetUnserializedPrivateCounter());

                Check.Equal("Round-trip nested struct gridX", 12, loaded.lastCheckpointCoord.gridX);
                Check.Equal("Round-trip nested struct gridY", -7, loaded.lastCheckpointCoord.gridY);
                Check.Equal("Round-trip nested struct private [SerializeField] _elevation", 48.5f, loaded.lastCheckpointCoord.GetElevation());

                Check.Equal("Round-trip nested class itemId", "relic_blade", loaded.primaryWeapon.itemId);
                Check.Equal("Round-trip nested class quantity", 1, loaded.primaryWeapon.quantity);
                Check.Equal("Round-trip nested class private [SerializeField] _durability", 92, loaded.primaryWeapon.GetDurability());
                Check.Equal("Nested class [NonSerialized] field keeps default value", 77, loaded.primaryWeapon.cachedTooltipHash);

                Check.Equal("Round-trip int[] unlockedChapters length", 4, loaded.unlockedChapters.Length);
                Check.Equal("Round-trip int[] unlockedChapters[3]", 5, loaded.unlockedChapters[3]);
                Check.Equal("Round-trip string[] activeModifiers length", 2, loaded.activeModifiers.Length);
                Check.Equal("Round-trip string[] activeModifiers[0]", "ironman", loaded.activeModifiers[0]);
                Check.Equal("Round-trip string[] activeModifiers[1]", "double_xp", loaded.activeModifiers[1]);

                Check.Equal("Round-trip generic int collection count", 3, loaded.completedMilestones.Count);
                Check.Equal("Round-trip generic int collection[2]", 35, loaded.completedMilestones[2]);
                Check.Equal("Round-trip nested generic item collection count", 2, loaded.backpackItems.Count);
                Check.Equal("Round-trip nested generic item collection[0] itemId", "healing_draught", loaded.backpackItems[0].itemId);
                Check.Equal("Round-trip nested generic item collection[0] quantity", 5, loaded.backpackItems[0].quantity);
                Check.Equal("Round-trip nested generic item collection[1] itemId", "vault_sigil", loaded.backpackItems[1].itemId);
                Check.Equal("Round-trip nested generic item collection[1] private durability", 50, loaded.backpackItems[1].GetDurability());
            }
        }

        private static void TestFromJsonOverwritePreservesReferenceAndUpdatesSelectiveFields()
        {
            SaveGameEnvelopeRoundTrip existing = new SaveGameEnvelopeRoundTrip
            {
                profileId = "slot_beta",
                schemaVersion = 2,
                playTimeSeconds = 100.0f,
                hardcoreMode = false,
                runtimeSessionNote = "keep_me_in_memory"
            };
            existing.SetCheckpointToken("initial_token");
            existing.SetSecretSeed(111);

            SaveGameEnvelopeRoundTrip originalRef = existing;
            string patchJson = "{\"playTimeSeconds\":950.5,\"_checkpointToken\":\"updated_checkpoint\",\"unlockedChapters\":[1,2,3]}";
            JsonUtility.FromJsonOverwrite(patchJson, ref existing);

            Check.That("FromJsonOverwrite mutates existing instance in place", ReferenceEquals(originalRef, existing));
            Check.Equal("FromJsonOverwrite preserves unmentioned public field profileId", "slot_beta", existing.profileId);
            Check.Equal("FromJsonOverwrite updates playTimeSeconds", 950.5f, existing.playTimeSeconds);
            Check.Equal("FromJsonOverwrite updates private [SerializeField] _checkpointToken", "updated_checkpoint", existing.GetCheckpointToken());
            Check.Equal("FromJsonOverwrite preserves unmentioned private [SerializeField] _secretSeed", 111, existing.GetSecretSeed());
            Check.Equal("FromJsonOverwrite preserves [NonSerialized] runtimeSessionNote", "keep_me_in_memory", existing.runtimeSessionNote);
            Check.Equal("FromJsonOverwrite updates int[] unlockedChapters length", 3, existing.unlockedChapters.Length);
        }

        private static void TestSaveDataVersionMigrationFromV1ToV2WithAtomicPersistence()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_sav001_migration_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string? savedAppOverride = Application.PersistentDataPathOverride;
            Application.PersistentDataPathOverride = tempDir;
            try
            {
                string saveFilePath = Path.Combine(Application.persistentDataPath, "profile_save.json");

                SavePayloadV1 legacySave = new SavePayloadV1
                {
                    version = 1,
                    heroName = "Lyra",
                    gold = 450,
                    legacyStageIndex = 6,
                    unlockedItemIds = new[] { "grappling_hook", "sun_compass" }
                };
                legacySave.SetLegacyDifficultyTag("veteran");

                string v1Json = JsonUtility.ToJson(legacySave, true);
                File.WriteAllText(saveFilePath, v1Json, new UTF8Encoding(false));

                SaveMigrationReport report = LoadAndMigrateSaveSlot(saveFilePath);

                Check.Equal("Migration report identifies source version 1", 1, report.SourceVersion);
                Check.Equal("Migration report identifies target version 2", 2, report.TargetVersion);
                Check.Equal("Migration report outcome is Migrated", "Migrated", report.Outcome);
                Check.Equal("Migration report records persisted save path", saveFilePath, report.PersistedPath);

                string[] leftoverTmpFiles = Directory.GetFiles(Application.persistentDataPath, "*.tmp*");
                Check.Equal("Atomic save migration leaves zero temporary files", 0, leftoverTmpFiles.Length);

                string persistedV2Json = File.ReadAllText(saveFilePath, Encoding.UTF8);
                SaveVersionProbe? postProbe = JsonUtility.FromJson<SaveVersionProbe>(persistedV2Json);
                Check.That("Persisted save version probe deserializes cleanly", postProbe != null);
                if (postProbe != null)
                {
                    Check.Equal("Persisted save file on disk now reports version 2", 2, postProbe.version);
                }

                SavePayloadV2? loadedV2 = JsonUtility.FromJson<SavePayloadV2>(persistedV2Json);
                Check.That("Migrated v2 payload loads cleanly through JsonUtility", loadedV2 != null);
                if (loadedV2 != null)
                {
                    Check.Equal("Migrated v2 payload has version 2", 2, loadedV2.version);
                    Check.Equal("Migrated v2 payload records migratedFromVersion 1", 1, loadedV2.migratedFromVersion);
                    Check.Equal("Migrated v2 payload preserves heroName", "Lyra", loadedV2.heroName);
                    Check.Equal("Migrated v2 payload maps v1 gold to currencyGold", 450, loadedV2.currencyGold);
                    Check.Equal("Migrated v2 payload maps legacyStageIndex to worldCoord.gridX", 60, loadedV2.worldCoord.gridX);
                    Check.Equal("Migrated v2 payload maps legacyStageIndex to worldCoord.gridY", 18, loadedV2.worldCoord.gridY);
                    Check.Equal("Migrated v2 payload maps legacyStageIndex to worldCoord._elevation", 9.0f, loadedV2.worldCoord.GetElevation());
                    Check.Equal("Migrated v2 payload maps veteran tag to _difficultyTier 2", 2, loadedV2.GetDifficultyTier());
                    Check.Equal("Migrated v2 payload converts unlockedItemIds to inventory count 2", 2, loadedV2.inventory.Count);
                    Check.Equal("Migrated v2 payload inventory[0] itemId", "grappling_hook", loadedV2.inventory[0].itemId);
                    Check.Equal("Migrated v2 payload inventory[0] quantity", 1, loadedV2.inventory[0].quantity);
                    Check.Equal("Migrated v2 payload inventory[1] itemId", "sun_compass", loadedV2.inventory[1].itemId);
                    Check.Equal("Migrated v2 payload inventory[1] durability", 100, loadedV2.inventory[1].GetDurability());
                }
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

        private static SaveMigrationReport LoadAndMigrateSaveSlot(string saveFilePath)
        {
            if (!File.Exists(saveFilePath))
            {
                return new SaveMigrationReport
                {
                    SourceVersion = 0,
                    TargetVersion = 2,
                    Outcome = "Absent",
                    PersistedPath = saveFilePath
                };
            }

            string rawJson = File.ReadAllText(saveFilePath, Encoding.UTF8);
            SaveVersionProbe? probe = JsonUtility.FromJson<SaveVersionProbe>(rawJson);
            if (probe == null || probe.version <= 0)
            {
                return new SaveMigrationReport
                {
                    SourceVersion = 0,
                    TargetVersion = 2,
                    Outcome = "Corrupt",
                    PersistedPath = saveFilePath
                };
            }

            if (probe.version == 1)
            {
                SavePayloadV1? v1 = JsonUtility.FromJson<SavePayloadV1>(rawJson);
                if (v1 == null)
                {
                    return new SaveMigrationReport
                    {
                        SourceVersion = 1,
                        TargetVersion = 2,
                        Outcome = "Corrupt",
                        PersistedPath = saveFilePath
                    };
                }

                SaveCoordStruct migratedCoord = new SaveCoordStruct
                {
                    gridX = v1.legacyStageIndex * 10,
                    gridY = v1.legacyStageIndex * 3
                };
                migratedCoord.SetElevation(v1.legacyStageIndex * 1.5f);

                int mappedTier = string.Equals(v1.GetLegacyDifficultyTag(), "veteran", StringComparison.Ordinal) ? 2 : 1;

                SavePayloadV2 v2 = new SavePayloadV2
                {
                    version = 2,
                    migratedFromVersion = 1,
                    heroName = v1.heroName,
                    currencyGold = v1.gold,
                    worldCoord = migratedCoord
                };
                v2.SetDifficultyTier(mappedTier);

                for (int i = 0; i < v1.unlockedItemIds.Length; i++)
                {
                    SaveInventoryEntry entry = new SaveInventoryEntry
                    {
                        itemId = v1.unlockedItemIds[i],
                        quantity = 1
                    };
                    entry.SetDurability(100);
                    v2.inventory.Add(entry);
                }

                WriteSaveFileAtomically(saveFilePath, JsonUtility.ToJson(v2, true));
                return new SaveMigrationReport
                {
                    SourceVersion = 1,
                    TargetVersion = 2,
                    Outcome = "Migrated",
                    PersistedPath = saveFilePath
                };
            }

            return new SaveMigrationReport
            {
                SourceVersion = probe.version,
                TargetVersion = probe.version,
                Outcome = "Current",
                PersistedPath = saveFilePath
            };
        }

        private static void WriteSaveFileAtomically(string targetPath, string serializedJson)
        {
            string directory = Path.GetDirectoryName(targetPath) ?? Application.persistentDataPath;
            Directory.CreateDirectory(directory);
            string tempPath = Path.Combine(directory, Path.GetFileName(targetPath) + ".tmp." + Guid.NewGuid().ToString("N"));
            try
            {
                File.WriteAllText(tempPath, serializedJson, new UTF8Encoding(false));
                if (File.Exists(targetPath))
                {
                    File.Delete(targetPath);
                }

                File.Move(tempPath, targetPath);
            }
            finally
            {
                if (File.Exists(tempPath))
                {
                    File.Delete(tempPath);
                }
            }
        }
    }
}
